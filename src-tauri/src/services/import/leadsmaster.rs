use std::path::Path;
use rusqlite::Connection;

use crate::error::AppError;
use crate::services::conversation_service;
use super::file_import::ImportRowDetail;
use super::call_log::ActivityImportResult;
use super::shared::{normalize_date, normalize_mbi, find_client};

/// Enrich existing Compass clients with data from LeadsMaster SQLite database.
pub fn enrich_from_leadsmaster(
    conn: &Connection,
    source_path: &str,
) -> Result<ActivityImportResult, AppError> {
    if !Path::new(source_path).exists() {
        return Err(AppError::Import(format!(
            "Source database not found: {}",
            source_path
        )));
    }

    let source_conn = Connection::open_with_flags(
        source_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| AppError::Import(format!("Failed to open source database: {}", e)))?;

    let mut stmt = source_conn
        .prepare(
            "SELECT first_name, last_name, mbi, dob, medicaid_number, medicaid_level,
                    lis_copay_level, eligibility, contract_pbp_segment, ai_summary
             FROM leads",
        )
        .map_err(|e| AppError::Import(format!("Failed to prepare query: {}", e)))?;

    struct LeadRow {
        first_name: Option<String>,
        last_name: Option<String>,
        mbi: Option<String>,
        dob: Option<String>,
        medicaid_number: Option<String>,
        medicaid_level: Option<String>,
        lis_copay_level: Option<String>,
        eligibility: Option<String>,
        contract_pbp_segment: Option<String>,
        ai_summary: Option<String>,
    }

    let rows: Vec<LeadRow> = stmt
        .query_map([], |row| {
            Ok(LeadRow {
                first_name: row.get(0)?,
                last_name: row.get(1)?,
                mbi: row.get(2)?,
                dob: row.get(3)?,
                medicaid_number: row.get(4)?,
                medicaid_level: row.get(5)?,
                lis_copay_level: row.get(6)?,
                eligibility: row.get(7)?,
                contract_pbp_segment: row.get(8)?,
                ai_summary: row.get(9)?,
            })
        })
        .map_err(|e| AppError::Import(format!("Failed to query source: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Import(format!("Failed to read source rows: {}", e)))?;

    let total_source_rows = rows.len();
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut unmatched = 0usize;
    let mut imported_details = Vec::new();
    let mut skipped_details = Vec::new();
    let mut unmatched_details = Vec::new();

    for src in &rows {
        let first_name = match src.first_name.as_deref() {
            Some(n) if !n.trim().is_empty() => n.trim(),
            _ => continue,
        };
        let last_name = match src.last_name.as_deref() {
            Some(n) if !n.trim().is_empty() => n.trim(),
            _ => continue,
        };

        let mbi = src.mbi.as_deref().and_then(normalize_mbi);
        let dob = src.dob.as_deref().and_then(normalize_date);
        let label = format!("{} {}", first_name, last_name);

        // Find existing Compass client
        let client_id = find_client(conn, mbi.as_deref(), first_name, last_name, dob.as_deref());
        let client_id = match client_id {
            Some(id) => id,
            None => {
                unmatched += 1;
                unmatched_details.push(ImportRowDetail {
                    label,
                    detail: format!("MBI: {}", mbi.as_deref().unwrap_or("(none)")),
                });
                continue;
            }
        };

        // Build enrichment updates: only fill NULL fields
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;
        let mut updated_fields = Vec::new();

        macro_rules! enrich_field {
            ($value:expr, $col:expr) => {
                if let Some(ref val) = $value {
                    if !val.is_empty() {
                        let current: Option<String> = conn
                            .query_row(
                                &format!("SELECT {} FROM clients WHERE id = ?1", $col),
                                rusqlite::params![client_id],
                                |row| row.get(0),
                            )
                            .ok()
                            .flatten();
                        if current.as_ref().map_or(true, |v| v.is_empty()) {
                            sets.push(format!("{} = ?{}", $col, idx));
                            params.push(Box::new(val.clone()));
                            idx += 1;
                            updated_fields.push($col.to_string());
                        }
                    }
                }
            };
        }

        enrich_field!(src.medicaid_number, "medicaid_id");
        enrich_field!(src.medicaid_level, "dual_status_code");
        enrich_field!(src.lis_copay_level, "lis_level");

        // Set is_dual_eligible if eligibility is "dsnp"
        if let Some(ref elig) = src.eligibility {
            if elig.eq_ignore_ascii_case("dsnp") {
                let current_dual: bool = conn
                    .query_row(
                        "SELECT is_dual_eligible FROM clients WHERE id = ?1",
                        rusqlite::params![client_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);
                if !current_dual {
                    sets.push(format!("is_dual_eligible = ?{}", idx));
                    params.push(Box::new(true));
                    idx += 1;
                    updated_fields.push("is_dual_eligible".to_string());
                }
            }
        }

        // Append contract_pbp_segment and ai_summary to notes
        let mut note_additions = Vec::new();
        if let Some(ref cpbs) = src.contract_pbp_segment {
            if !cpbs.is_empty() {
                note_additions.push(format!("[LeadsMaster] Contract-PBP-Segment: {}", cpbs));
            }
        }
        if let Some(ref ai) = src.ai_summary {
            if !ai.is_empty() {
                note_additions.push(format!("[LeadsMaster AI] {}", ai));
            }
        }

        if !note_additions.is_empty() {
            let addition = note_additions.join("\n");
            let current_notes: Option<String> = conn
                .query_row(
                    "SELECT notes FROM clients WHERE id = ?1",
                    rusqlite::params![client_id],
                    |row| row.get(0),
                )
                .ok()
                .flatten();

            let should_append = current_notes
                .as_ref()
                .map_or(true, |existing| !existing.contains(&addition));

            if should_append {
                let merged = match current_notes {
                    Some(existing) if !existing.is_empty() => {
                        format!("{}\n\n{}", existing, addition)
                    }
                    _ => addition,
                };
                sets.push(format!("notes = ?{}", idx));
                params.push(Box::new(merged));
                idx += 1;
                updated_fields.push("notes".to_string());
            }
        }

        if sets.is_empty() {
            skipped += 1;
            skipped_details.push(ImportRowDetail {
                label,
                detail: "No new data".to_string(),
            });
            continue;
        }

        let sql = format!(
            "UPDATE clients SET {} WHERE id = ?{}",
            sets.join(", "),
            idx
        );
        params.push(Box::new(client_id.clone()));
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

        let event_data = serde_json::json!({
            "source": "leadsmaster",
            "fields": updated_fields,
        })
        .to_string();
        let _ = conversation_service::create_system_event(
            conn,
            &client_id,
            "CLIENT_UPDATED",
            Some(&event_data),
        );

        imported += 1;
        imported_details.push(ImportRowDetail {
            label,
            detail: updated_fields.join(", "),
        });
    }

    Ok(ActivityImportResult {
        imported,
        skipped,
        unmatched,
        total_source_rows,
        imported_details,
        skipped_details,
        unmatched_details,
    })
}
