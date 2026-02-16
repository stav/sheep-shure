use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{PortalMember, SyncDisenrollment, SyncLogEntry, SyncResult};

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
    let mut new_in_portal: Vec<PortalMember> = Vec::new();

    for pm in portal_members {
        if let Some(local_match) = find_match(&local, pm) {
            matched_enrollment_ids.push(local_match.enrollment_id.clone());
        } else {
            new_in_portal.push(pm.clone());
        }
    }

    // 3. Local enrollments NOT matched in portal → disenroll
    let mut disenrolled: Vec<SyncDisenrollment> = Vec::new();
    for le in &local {
        if !matched_enrollment_ids.contains(&le.enrollment_id) {
            disenroll_enrollment(conn, &le.enrollment_id)?;
            disenrolled.push(SyncDisenrollment {
                client_name: format!("{} {}", le.client_first_name, le.client_last_name),
                client_id: le.client_id.clone(),
                enrollment_id: le.enrollment_id.clone(),
                plan_name: le.plan_name.clone(),
            });
        }
    }

    let matched = matched_enrollment_ids.len();

    // 4. Log the sync
    log_sync(conn, carrier_id, portal_count, matched, disenrolled.len(), new_in_portal.len())?;

    Ok(SyncResult {
        carrier_name: carrier_name.to_string(),
        portal_count,
        local_count,
        matched,
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
/// Strategy: MBI first (most reliable), then last_name + first_name.
fn find_match<'a>(locals: &'a [LocalEnrollment], portal: &PortalMember) -> Option<&'a LocalEnrollment> {
    // Try MBI match first (if portal provides a member_id that could be an MBI)
    if let Some(ref portal_member_id) = portal.member_id {
        let mbi_match = locals.iter().find(|le| {
            le.client_mbi
                .as_ref()
                .map(|mbi| mbi.eq_ignore_ascii_case(portal_member_id))
                .unwrap_or(false)
        });
        if mbi_match.is_some() {
            return mbi_match;
        }
    }

    // Fall back to name matching (case-insensitive)
    locals.iter().find(|le| {
        le.client_last_name.eq_ignore_ascii_case(&portal.last_name)
            && le.client_first_name.eq_ignore_ascii_case(&portal.first_name)
    })
}

/// Mark an enrollment as disenrolled (involuntary).
fn disenroll_enrollment(conn: &Connection, enrollment_id: &str) -> Result<(), AppError> {
    let sql = "UPDATE enrollments
               SET status_code = 'DISENROLLED',
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
