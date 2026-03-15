use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::DbState;
use crate::models::CreateClientInput;
use crate::services::convex_service::{self, BulkPushResult, CloudClient, ConvexConfig};
use crate::services::client_service;

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct LocalClientSummary {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub dob: Option<String>,
    pub mbi: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloudClientSummary {
    pub cloud_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub dob: Option<String>,
    pub mbi: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FieldDiff {
    pub field: String,
    pub local: Option<String>,
    pub cloud: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClientConflict {
    pub local: LocalClientSummary,
    pub cloud: CloudClientSummary,
    pub diffs: Vec<FieldDiff>,
}

#[derive(Debug, Serialize)]
pub struct MatchedPair {
    pub local: LocalClientSummary,
    pub cloud: CloudClientSummary,
}

#[derive(Debug, Serialize)]
pub struct ReconciliationResult {
    pub only_local: Vec<LocalClientSummary>,
    pub only_cloud: Vec<CloudClientSummary>,
    pub conflicts: Vec<ClientConflict>,
    pub matched: Vec<MatchedPair>,
}

#[derive(Debug, Serialize)]
pub struct SyncDecision {
    pub cloud_record_id: String,
    pub decision: String,
    pub diff: Option<String>,
    pub decided_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveDecisionInput {
    pub cloud_record_id: String,
    pub decision: String,
    pub diff: Option<String>,
    pub expires_days: Option<i64>,
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn normalize(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

fn cloud_to_summary(c: &CloudClient) -> CloudClientSummary {
    CloudClientSummary {
        cloud_id: c.id.clone(),
        first_name: c.first_name.clone(),
        last_name: c.last_name.clone(),
        dob: c.dob.clone(),
        mbi: c.mbi.clone(),
        phone: c.phone.clone(),
        email: c.email.clone(),
        address_line1: c.address_line1.clone(),
        city: c.city.clone(),
        state: c.state.clone(),
        zip: c.zip.clone(),
    }
}

fn compute_diffs(local: &LocalClientSummary, cloud: &CloudClientSummary) -> Vec<FieldDiff> {
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("first_name", Some(local.first_name.as_str()), cloud.first_name.as_deref()),
        ("last_name", Some(local.last_name.as_str()), cloud.last_name.as_deref()),
        ("dob", local.dob.as_deref(), cloud.dob.as_deref()),
        ("mbi", local.mbi.as_deref(), cloud.mbi.as_deref()),
        ("phone", local.phone.as_deref(), cloud.phone.as_deref()),
        ("email", local.email.as_deref(), cloud.email.as_deref()),
        ("address_line1", local.address_line1.as_deref(), cloud.address_line1.as_deref()),
        ("city", local.city.as_deref(), cloud.city.as_deref()),
        ("state", local.state.as_deref(), cloud.state.as_deref()),
        ("zip", local.zip.as_deref(), cloud.zip.as_deref()),
    ];

    pairs.iter().filter_map(|(name, lv, cv)| {
        let l = lv.map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty());
        let c = cv.map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty());
        if l != c {
            Some(FieldDiff {
                field: name.to_string(),
                local: lv.map(|s| s.to_string()),
                cloud: cv.map(|s| s.to_string()),
            })
        } else {
            None
        }
    }).collect()
}

// Try to match a cloud client to a local client list.
// Returns index into `locals` vec.
fn find_local_match(locals: &[LocalClientSummary], cloud: &CloudClient) -> Option<usize> {
    let c_mbi = cloud.mbi.as_deref().map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty());
    let c_last = cloud.last_name.as_deref().map(normalize).unwrap_or_default();
    let c_dob = cloud.dob.as_deref().map(normalize).filter(|s| !s.is_empty());
    let c_first = cloud.first_name.as_deref().map(normalize).unwrap_or_default();

    // Tier 1: MBI match
    if let Some(ref mbi) = c_mbi {
        if let Some(idx) = locals.iter().position(|l| {
            l.mbi.as_deref().map(|s| s.trim().to_ascii_lowercase()).as_deref() == Some(mbi.as_str())
        }) {
            return Some(idx);
        }
    }

    // Tier 2: last + dob
    if let Some(ref dob) = c_dob {
        if let Some(idx) = locals.iter().position(|l| {
            normalize(&l.last_name) == c_last
                && l.dob.as_deref().map(normalize).as_deref() == Some(dob.as_str())
        }) {
            return Some(idx);
        }
    }

    // Tier 3: last + first (normalized)
    if !c_last.is_empty() && !c_first.is_empty() {
        if let Some(idx) = locals.iter().position(|l| {
            normalize(&l.last_name) == c_last && normalize(&l.first_name) == c_first
        }) {
            return Some(idx);
        }
    }

    None
}

// ── Commands ───────────────────────────────────────────────────────────────────

/// Debug: show raw client count, deserialized count, and first parsed CloudClient.
#[tauri::command]
pub async fn debug_pull_raw_client(state: State<'_, DbState>) -> Result<String, String> {
    let config = state
        .with_conn(|conn| Ok(ConvexConfig::from_settings(conn)))
        .map_err(|e| e.to_string())?
        .ok_or("Convex not configured")?;

    let client = reqwest::Client::new();
    let url = format!("{}/api/sync/pull", config.base_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .bearer_auth(&config.token)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {}", e))?;
    let arr = val.get("clients").and_then(|v| v.as_array());
    let raw_count = arr.map(|a| a.len()).unwrap_or(0);

    let parsed: Vec<CloudClient> = arr
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| serde_json::from_value::<CloudClient>(v.clone()).ok())
        .collect();

    let first_err = arr.unwrap_or(&vec![]).first().map(|v| {
        serde_json::from_value::<CloudClient>(v.clone())
            .err()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "ok".to_string())
    });

    Ok(format!(
        "raw_count={}, parsed_count={}, first_parse_result={:?}, first_parsed={:?}",
        raw_count, parsed.len(), first_err, parsed.first()
    ))
}

#[tauri::command]
pub async fn compare_with_convex(state: State<'_, DbState>) -> Result<ReconciliationResult, String> {
    let config = state
        .with_conn(|conn| Ok(ConvexConfig::from_settings(conn)))
        .map_err(|e| e.to_string())?;

    let Some(config) = config else {
        return Err("Convex not configured. Set token and URL in Settings → Compass Cloud.".to_string());
    };

    // Pull cloud clients
    let cloud_clients = convex_service::pull_clients_full(&config).await?;

    // Load local active clients
    let locals: Vec<LocalClientSummary> = state
        .with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, first_name, last_name, dob, mbi, phone, email,
                        address_line1, city, state, zip
                 FROM clients WHERE is_active = 1",
            ).map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let locals: Vec<LocalClientSummary> = stmt
                .query_map([], |row| {
                    Ok(LocalClientSummary {
                        id: row.get(0)?,
                        first_name: row.get(1)?,
                        last_name: row.get(2)?,
                        dob: row.get(3)?,
                        mbi: row.get(4)?,
                        phone: row.get(5)?,
                        email: row.get(6)?,
                        address_line1: row.get(7)?,
                        city: row.get(8)?,
                        state: row.get(9)?,
                        zip: row.get(10)?,
                    })
                })
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(locals)
        })
        .map_err(|e| e.to_string())?;

    let mut matched_local_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut only_cloud: Vec<CloudClientSummary> = Vec::new();
    let mut conflicts: Vec<ClientConflict> = Vec::new();
    let mut matched: Vec<MatchedPair> = Vec::new();

    for cloud in &cloud_clients {
        match find_local_match(&locals, cloud) {
            Some(idx) => {
                matched_local_indices.insert(idx);
                let local = &locals[idx];
                let cloud_summary = cloud_to_summary(cloud);
                let diffs = compute_diffs(local, &cloud_summary);
                if !diffs.is_empty() {
                    conflicts.push(ClientConflict {
                        local: local.clone(),
                        cloud: cloud_summary,
                        diffs,
                    });
                } else {
                    matched.push(MatchedPair {
                        local: local.clone(),
                        cloud: cloud_summary,
                    });
                }
            }
            None => {
                only_cloud.push(cloud_to_summary(cloud));
            }
        }
    }

    // Local clients not matched by any cloud client
    let only_local: Vec<LocalClientSummary> = locals
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !matched_local_indices.contains(i))
        .map(|(_, l)| l)
        .collect();

    Ok(ReconciliationResult {
        only_local,
        only_cloud,
        conflicts,
        matched,
    })
}

#[tauri::command]
pub fn save_sync_decision(
    input: SaveDecisionInput,
    state: State<'_, DbState>,
) -> Result<(), String> {
    state
        .with_conn(|conn| {
            let expires_at = input.expires_days.map(|d| {
                let now = chrono::Utc::now();
                let expires = now + chrono::Duration::days(d);
                expires.format("%Y-%m-%d %H:%M:%S").to_string()
            });

            conn.execute(
                "INSERT INTO convex_sync_decisions (cloud_record_id, decision, diff, expires_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(cloud_record_id) DO UPDATE SET
                     decision = excluded.decision,
                     diff = excluded.diff,
                     decided_at = datetime('now'),
                     expires_at = excluded.expires_at",
                params![input.cloud_record_id, input.decision, input.diff, expires_at],
            ).map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            Ok(())
        })
        .map_err(|e| e.to_string())
}

/// Push a single local client (and their enrollments) to Convex.
#[tauri::command]
pub async fn push_client_to_convex(
    client_id: String,
    state: State<'_, DbState>,
) -> Result<BulkPushResult, String> {
    let (config, clients, enrollments) = state
        .with_conn(|conn| {
            let config = ConvexConfig::from_settings(conn);

            let mut stmt = conn.prepare(
                "SELECT first_name, last_name, middle_name, dob, gender,
                        phone, phone2, email,
                        address_line1, address_line2, city, state, zip, county,
                        mbi, part_a_date, part_b_date, orec,
                        is_dual_eligible, dual_status_code, lis_level,
                        medicaid_id, lead_source, member_record_locator,
                        notes, is_active
                 FROM clients WHERE id = ?1",
            ).map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let clients: Vec<serde_json::Value> = stmt
                .query_map(params![client_id], |row| {
                    Ok(serde_json::json!({
                        "firstName": row.get::<_, Option<String>>(0)?,
                        "lastName": row.get::<_, Option<String>>(1)?,
                        "middleName": row.get::<_, Option<String>>(2)?,
                        "dob": row.get::<_, Option<String>>(3)?,
                        "gender": row.get::<_, Option<String>>(4)?,
                        "phone": row.get::<_, Option<String>>(5)?,
                        "phone2": row.get::<_, Option<String>>(6)?,
                        "email": row.get::<_, Option<String>>(7)?,
                        "addressLine1": row.get::<_, Option<String>>(8)?,
                        "addressLine2": row.get::<_, Option<String>>(9)?,
                        "city": row.get::<_, Option<String>>(10)?,
                        "state": row.get::<_, Option<String>>(11)?,
                        "zip": row.get::<_, Option<String>>(12)?,
                        "county": row.get::<_, Option<String>>(13)?,
                        "mbi": row.get::<_, Option<String>>(14)?,
                        "partADate": row.get::<_, Option<String>>(15)?,
                        "partBDate": row.get::<_, Option<String>>(16)?,
                        "orec": row.get::<_, Option<String>>(17)?,
                        "isDualEligible": row.get::<_, Option<bool>>(18)?.unwrap_or(false),
                        "dualStatusCode": row.get::<_, Option<String>>(19)?,
                        "lisLevel": row.get::<_, Option<String>>(20)?,
                        "medicaidId": row.get::<_, Option<String>>(21)?,
                        "leadSource": row.get::<_, Option<String>>(22)?,
                        "memberRecordLocator": row.get::<_, Option<String>>(23)?,
                        "notes": row.get::<_, Option<String>>(24)?,
                        "isActive": row.get::<_, Option<bool>>(25)?.unwrap_or(true),
                    }))
                })
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            let mut stmt2 = conn.prepare(
                "SELECT c.first_name, c.last_name, c.mbi,
                        ca.short_name,
                        e.plan_name, e.plan_type_code, e.contract_number, e.pbp_number,
                        e.effective_date, e.termination_date, e.application_date,
                        e.status_code, e.enrollment_period, e.confirmation_number,
                        e.enrollment_source, e.is_active
                 FROM enrollments e
                 JOIN clients c ON e.client_id = c.id
                 LEFT JOIN carriers ca ON e.carrier_id = ca.id
                 WHERE c.id = ?1",
            ).map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let enrollments: Vec<serde_json::Value> = stmt2
                .query_map(params![client_id], |row| {
                    Ok(serde_json::json!({
                        "clientFirstName": row.get::<_, Option<String>>(0)?,
                        "clientLastName": row.get::<_, Option<String>>(1)?,
                        "clientMbi": row.get::<_, Option<String>>(2)?,
                        "carrierShortName": row.get::<_, Option<String>>(3)?,
                        "planName": row.get::<_, Option<String>>(4)?,
                        "planTypeCode": row.get::<_, Option<String>>(5)?,
                        "contractNumber": row.get::<_, Option<String>>(6)?,
                        "pbpNumber": row.get::<_, Option<String>>(7)?,
                        "effectiveDate": row.get::<_, Option<String>>(8)?,
                        "terminationDate": row.get::<_, Option<String>>(9)?,
                        "applicationDate": row.get::<_, Option<String>>(10)?,
                        "statusCode": row.get::<_, Option<String>>(11)
                            .map(|s| s.unwrap_or_else(|| "PENDING".to_string()))?,
                        "enrollmentPeriod": row.get::<_, Option<String>>(12)?,
                        "confirmationNumber": row.get::<_, Option<String>>(13)?,
                        "enrollmentSource": row.get::<_, Option<String>>(14)?,
                        "isActive": row.get::<_, Option<bool>>(15)?.unwrap_or(true),
                    }))
                })
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            Ok((config, clients, enrollments))
        })
        .map_err(|e| e.to_string())?;

    let Some(config) = config else {
        return Err("Convex not configured. Set token and URL in Settings → Compass Cloud.".to_string());
    };

    convex_service::push_all(&config, clients, enrollments).await
}

#[tauri::command]
pub fn get_sync_decisions(state: State<'_, DbState>) -> Result<Vec<SyncDecision>, String> {
    state
        .with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT cloud_record_id, decision, diff, decided_at, expires_at
                 FROM convex_sync_decisions
                 ORDER BY decided_at DESC",
            ).map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let items = stmt
                .query_map([], |row| {
                    Ok(SyncDecision {
                        cloud_record_id: row.get(0)?,
                        decision: row.get(1)?,
                        diff: row.get(2)?,
                        decided_at: row.get(3)?,
                        expires_at: row.get(4)?,
                    })
                })
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(items)
        })
        .map_err(|e| e.to_string())
}

/// Pull a cloud-only client into the local database.
#[tauri::command]
pub fn pull_client_from_cloud(
    client: CloudClientSummary,
    state: State<'_, DbState>,
) -> Result<(), String> {
    state
        .with_conn(|conn| {
            let input = CreateClientInput {
                first_name: client.first_name.unwrap_or_default(),
                last_name: client.last_name.unwrap_or_default(),
                middle_name: None,
                dob: client.dob,
                gender: None,
                phone: client.phone,
                phone2: None,
                email: client.email,
                address_line1: client.address_line1,
                address_line2: None,
                city: client.city,
                state: client.state,
                zip: client.zip,
                county: None,
                mbi: client.mbi,
                part_a_date: None,
                part_b_date: None,
                orec: None,
                is_dual_eligible: None,
                dual_status_code: None,
                lis_level: None,
                medicaid_id: None,
                lead_source: Some("cloud_sync".to_string()),
                member_record_locator: None,
                tags: None,
                notes: None,
            };
            client_service::create_client(conn, &input)
                .map(|_| ())
                .map_err(|e| crate::error::AppError::Database(e.to_string()))
        })
        .map_err(|e| e.to_string())
}
