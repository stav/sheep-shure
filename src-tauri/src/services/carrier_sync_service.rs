use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    ConfirmDisenrollmentResult, CreateClientInput, CreateEnrollmentInput, ImportPortalResult,
    PortalMember, SyncDisenrollment, SyncLogEntry, SyncMatch, SyncResult,
};
use crate::models::CreateProviderInput;
use crate::services::{client_service, conversation_service, enrollment_service, matching, provider_service};

/// Internal struct for matching local enrollments against portal data.
struct LocalEnrollment {
    enrollment_id: String,
    client_id: String,
    client_first_name: String,
    client_last_name: String,
    client_mbi: Option<String>,
    client_dob: Option<String>,
    plan_name: Option<String>,
}

/// Compare portal members against local enrollments for a given carrier,
/// auto-update disenrolled records, and return a summary.
pub fn run_sync(
    conn: &Connection,
    carrier_id: &str,
    carrier_name: &str,
    portal_members: &[PortalMember],
) -> Result<SyncResult, AppError> {
    // 1. Fetch local active enrollments for this carrier
    let local = get_local_enrollments(conn, carrier_id)?;
    let local_count = local.len();
    let portal_count = portal_members.len();

    // 2. Match portal members to local enrollments
    let mut matched_enrollment_ids: Vec<String> = Vec::new();
    let mut matched_members: Vec<SyncMatch> = Vec::new();
    let mut new_in_portal: Vec<PortalMember> = Vec::new();

    for pm in portal_members {
        if let Some((local_match, tier)) = find_match(&local, pm) {
            matched_enrollment_ids.push(local_match.enrollment_id.clone());
            matched_members.push(SyncMatch {
                client_name: format!("{} {}", local_match.client_first_name, local_match.client_last_name),
                client_id: local_match.client_id.clone(),
                portal_member: pm.clone(),
                match_tier: tier.to_string(),
            });
        } else if let Some(client_id) = find_existing_client(conn, pm) {
            // Matched an existing client (active or inactive) but no active enrollment
            let client_name: String = conn
                .query_row(
                    "SELECT first_name || ' ' || last_name FROM clients WHERE id = ?1",
                    params![client_id],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| format!("{} {}", pm.first_name, pm.last_name));
            matched_members.push(SyncMatch {
                client_name,
                client_id,
                portal_member: pm.clone(),
                match_tier: "existing_client".to_string(),
            });
        } else {
            new_in_portal.push(pm.clone());
        }
    }

    // 3. Local enrollments NOT matched in portal → candidates for disenrollment
    //    (reported to the user for confirmation, NOT auto-disenrolled)
    let mut disenrolled: Vec<SyncDisenrollment> = Vec::new();
    for le in &local {
        if !matched_enrollment_ids.contains(&le.enrollment_id) {
            disenrolled.push(SyncDisenrollment {
                client_name: format!("{} {}", le.client_first_name, le.client_last_name),
                client_id: le.client_id.clone(),
                enrollment_id: le.enrollment_id.clone(),
                plan_name: le.plan_name.clone(),
            });
        }
    }

    let matched = matched_members.len();

    // 4. Log the sync (disenrolled=0 because disenrollment is now user-confirmed)
    log_sync(conn, carrier_id, portal_count, matched, 0, new_in_portal.len())?;

    Ok(SyncResult {
        carrier_name: carrier_name.to_string(),
        portal_count,
        local_count,
        matched,
        matched_members,
        disenrolled,
        new_in_portal,
    })
}

/// Fetch all active enrollments for a given carrier, joined with client info.
fn get_local_enrollments(conn: &Connection, carrier_id: &str) -> Result<Vec<LocalEnrollment>, AppError> {
    let sql = "SELECT e.id, e.client_id, c.first_name, c.last_name, c.mbi, c.dob, e.plan_name
               FROM enrollments e
               JOIN clients c ON e.client_id = c.id
               WHERE e.carrier_id = ?1
                 AND e.status_code IN ('ACTIVE', 'PENDING')
                 AND e.is_active = 1
                 AND c.is_active = 1";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params![carrier_id], |row| {
            Ok(LocalEnrollment {
                enrollment_id: row.get(0)?,
                client_id: row.get(1)?,
                client_first_name: row.get(2)?,
                client_last_name: row.get(3)?,
                client_mbi: row.get(4)?,
                client_dob: row.get(5)?,
                plan_name: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Try to match a portal member to a local enrollment.
///
/// Strategy (all comparisons lowercased):
///   1. Last name must match exactly.
///   2. Then accept if ANY of these hold:
///      a. First name matches exactly (normalized)
///      b. First name fuzzy-matches + DOB matches
///      c. MBI matches + DOB matches
fn find_match<'a>(locals: &'a [LocalEnrollment], portal: &PortalMember) -> Option<(&'a LocalEnrollment, &'static str)> {
    let p_last = portal.last_name.to_ascii_lowercase();
    let p_first = portal.first_name.to_ascii_lowercase();
    let p_dob_norm = portal.dob.as_deref().and_then(matching::normalize_date);
    let p_mbi = portal.member_id.as_deref();

    // Filter to last-name matches first
    let candidates: Vec<&LocalEnrollment> = locals
        .iter()
        .filter(|le| le.client_last_name.to_ascii_lowercase() == p_last)
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Tier 1: exact first name match (strongest — no DOB needed)
    if let Some(m) = candidates.iter().find(|le| {
        matching::normalize_first_name(&le.client_first_name) == matching::normalize_first_name(&p_first)
    }) {
        return Some((m, "exact"));
    }

    // Tier 2: fuzzy first name + DOB
    if let Some(ref dob) = p_dob_norm {
        if let Some(m) = candidates.iter().find(|le| {
            matching::fuzzy_first_name(&le.client_first_name, &p_first)
                && le.client_dob.as_deref().and_then(matching::normalize_date).as_deref() == Some(dob.as_str())
        }) {
            return Some((m, "fuzzy"));
        }
    }

    // Tier 3: MBI + DOB
    if let (Some(mbi), Some(ref dob)) = (p_mbi, &p_dob_norm) {
        if !mbi.is_empty() {
            if let Some(m) = candidates.iter().find(|le| {
                le.client_mbi.as_deref() == Some(mbi)
                    && le.client_dob.as_deref().and_then(matching::normalize_date).as_deref() == Some(dob.as_str())
            }) {
                return Some((m, "mbi"));
            }
        }
    }

    None
}

/// Mark an enrollment as disenrolled (involuntary).
fn disenroll_enrollment(conn: &Connection, enrollment_id: &str) -> Result<(), AppError> {
    let sql = "UPDATE enrollments
               SET status_code = 'DISENROLLED_INVOLUNTARY',
                   disenrollment_reason = 'Carrier portal sync - not found in portal',
                   termination_date = date('now'),
                   updated_at = datetime('now')
               WHERE id = ?1";

    conn.execute(sql, params![enrollment_id])?;
    Ok(())
}

/// Insert a sync log entry.
fn log_sync(
    conn: &Connection,
    carrier_id: &str,
    portal_count: usize,
    matched: usize,
    disenrolled: usize,
    new_found: usize,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let sql = "INSERT INTO carrier_sync_logs (id, carrier_id, portal_count, matched, disenrolled, new_found)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

    conn.execute(
        sql,
        params![id, carrier_id, portal_count as i64, matched as i64, disenrolled as i64, new_found as i64],
    )?;
    Ok(())
}

/// Find an existing client by MBI or by (first_name, last_name, DOB).
/// Searches both active and inactive clients so that a re-appearing member
/// reuses the existing record instead of creating a duplicate.
fn find_existing_client(
    conn: &Connection,
    member: &PortalMember,
) -> Option<String> {
    matching::find_client_match(
        conn,
        member.mbi.as_deref(),
        &member.first_name,
        &member.last_name,
        member.dob.as_deref(),
        &matching::MatchOptions::default(),
    )
    .map(|m| m.client_id)
}

/// Import portal members as new clients with enrollments linked to the carrier.
pub fn import_portal_members(
    conn: &Connection,
    carrier_id: &str,
    members: &[PortalMember],
) -> Result<ImportPortalResult, AppError> {
    let mut imported = 0usize;
    let mut imported_names = Vec::new();
    let mut errors = Vec::new();

    for member in members {
        // Check for an existing client (active or inactive) before creating a new one
        let client_id = if let Some(existing_id) = find_existing_client(conn, member) {
            // Reactivate if the matched client is inactive
            let _ = conn.execute(
                "UPDATE clients SET is_active = 1, updated_at = datetime('now') WHERE id = ?1 AND is_active = 0",
                params![existing_id],
            );
            existing_id
        } else {
            let client_input = CreateClientInput {
                first_name: member.first_name.clone(),
                last_name: member.last_name.clone(),
                middle_name: member.middle_name.clone(),
                dob: member.dob.clone(),
                gender: member.gender.clone(),
                phone: member.phone.clone(),
                phone2: None,
                email: member.email.clone(),
                address_line1: member.address_line1.clone(),
                address_line2: member.address_line2.clone(),
                city: member.city.clone(),
                state: member.state.clone(),
                zip: member.zip.clone(),
                county: member.county.clone(),
                mbi: member.mbi.clone(),
                part_a_date: None,
                part_b_date: None,
                orec: None,
                is_dual_eligible: None,
                dual_status_code: None,
                lis_level: None,
                medicaid_id: member.medicaid_id.clone(),
                lead_source: Some("carrier_sync".to_string()),
                member_record_locator: member.member_record_locator.clone(),
                tags: None,
                notes: None,
            };

            match client_service::create_client(conn, &client_input) {
                Ok(c) => c.id,
                Err(e) => {
                    errors.push(format!(
                        "{} {}: failed to create client — {}",
                        member.first_name, member.last_name, e
                    ));
                    continue;
                }
            }
        };

        let event_data = serde_json::json!({
            "carrier_id": carrier_id,
            "source": "carrier_sync",
        })
        .to_string();
        let _ = conversation_service::create_system_event(
            conn,
            &client_id,
            "CLIENT_IMPORTED",
            Some(&event_data),
        );

        let status_code = {
            let s = member.status.as_deref().unwrap_or("").to_lowercase();
            let ps = member.policy_status.as_deref().unwrap_or("").to_lowercase();
            // Explicitly inactive / canceled
            if ps.contains("inactive") || s.contains("cancel") || s.contains("inactive") || s == "not_enrolled" || s == "terminated" {
                "CANCELLED"
            // Explicitly active
            } else if ps.contains("active") || s.contains("active") || s == "enrolled" {
                "ACTIVE"
            // Blank status = active (e.g. Medical Mutual)
            } else if s.trim().is_empty() {
                "ACTIVE"
            } else {
                "PENDING"
            }
        };

        // If the member is canceled/inactive on the portal, deactivate the client
        if status_code == "CANCELLED" {
            let _ = conn.execute(
                "UPDATE clients SET is_active = 0, updated_at = datetime('now') WHERE id = ?1",
                params![client_id],
            );
        }

        let enrollment_input = CreateEnrollmentInput {
            client_id: client_id.clone(),
            plan_id: None,
            carrier_id: Some(carrier_id.to_string()),
            plan_type_code: None,
            plan_name: member.plan_name.clone(),
            contract_number: None,
            pbp_number: None,
            effective_date: member.effective_date.clone(),
            termination_date: None,
            application_date: member.application_date.clone(),
            status_code: Some(status_code.to_string()),
            enrollment_period: None,
            disenrollment_reason: None,
            premium: None,
            confirmation_number: None,
            enrollment_source: Some("carrier_sync".to_string()),
        };

        match enrollment_service::create_enrollment(conn, &enrollment_input) {
            Ok(_) => {
                imported += 1;
                imported_names.push(format!("{} {}", member.first_name, member.last_name));
            }
            Err(e) => {
                errors.push(format!(
                    "{} {}: enrollment failed — {}",
                    member.first_name, member.last_name, e
                ));
            }
        }

        // Create provider if present
        if member.provider_first_name.is_some() || member.provider_last_name.is_some() {
            let provider_input = CreateProviderInput {
                client_id: client_id.clone(),
                first_name: member.provider_first_name.clone(),
                last_name: member.provider_last_name.clone(),
                npi: None,
                specialty: None,
                phone: None,
                is_pcp: Some(true),
                source: Some("carrier_sync".to_string()),
            };
            let _ = provider_service::create_provider(conn, &provider_input);
        }
    }

    Ok(ImportPortalResult { imported, imported_names, errors })
}

/// Confirm disenrollment for selected enrollment IDs.
pub fn confirm_disenrollments(
    conn: &Connection,
    enrollment_ids: &[String],
) -> Result<ConfirmDisenrollmentResult, AppError> {
    let mut disenrolled = 0usize;
    let mut errors = Vec::new();

    for eid in enrollment_ids {
        match disenroll_enrollment(conn, eid) {
            Ok(()) => disenrolled += 1,
            Err(e) => errors.push(format!("Enrollment {}: {}", eid, e)),
        }
    }

    Ok(ConfirmDisenrollmentResult { disenrolled, errors })
}

/// Get sync log history for a carrier (most recent first).
pub fn get_sync_logs(conn: &Connection, carrier_id: Option<&str>) -> Result<Vec<SyncLogEntry>, AppError> {
    let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(cid) = carrier_id {
        (
            "SELECT sl.id, sl.carrier_id, cr.name, sl.synced_at, sl.portal_count, sl.matched, sl.disenrolled, sl.new_found, sl.status
             FROM carrier_sync_logs sl
             LEFT JOIN carriers cr ON sl.carrier_id = cr.id
             WHERE sl.carrier_id = ?1
             ORDER BY sl.synced_at DESC
             LIMIT 50".to_string(),
            vec![Box::new(cid.to_string()) as Box<dyn rusqlite::types::ToSql>],
        )
    } else {
        (
            "SELECT sl.id, sl.carrier_id, cr.name, sl.synced_at, sl.portal_count, sl.matched, sl.disenrolled, sl.new_found, sl.status
             FROM carrier_sync_logs sl
             LEFT JOIN carriers cr ON sl.carrier_id = cr.id
             ORDER BY sl.synced_at DESC
             LIMIT 50".to_string(),
            vec![],
        )
    };

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(SyncLogEntry {
                id: row.get(0)?,
                carrier_id: row.get(1)?,
                carrier_name: row.get(2)?,
                synced_at: row.get(3)?,
                portal_count: row.get(4)?,
                matched: row.get(5)?,
                disenrolled: row.get(6)?,
                new_found: row.get(7)?,
                status: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}
