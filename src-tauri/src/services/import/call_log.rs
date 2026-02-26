use std::path::Path;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use crate::services::matching;
use super::file_import::ImportRowDetail;

#[derive(serde::Serialize)]
pub struct ActivityImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub unmatched: usize,
    pub total_source_rows: usize,
    pub imported_details: Vec<ImportRowDetail>,
    pub skipped_details: Vec<ImportRowDetail>,
    pub unmatched_details: Vec<ImportRowDetail>,
}

/// Row read from the external call_log + leads tables
struct SourceCallLog {
    contact_type: String,
    disposition: String,
    notes: Option<String>,
    call_date: Option<String>,
    follow_up_date: Option<String>,
    lead_mbi: Option<String>,
    lead_first_name: Option<String>,
    lead_last_name: Option<String>,
    lead_dob: Option<String>,
    lead_phone: Option<String>,
}

fn map_disposition_to_call_outcome(disposition: &str) -> &'static str {
    match disposition.trim().to_lowercase().as_str() {
        "no answer" => "NO_ANSWER",
        "left voicemail" => "VOICEMAIL",
        "callback needed" => "CALLBACK_REQUESTED",
        "busy" => "BUSY",
        "wrong number" | "disconnected" => "WRONG_NUMBER",
        _ => "ANSWERED",
    }
}

/// Find or create a "CRM Import: Call History" conversation for a given client.
fn find_or_create_import_conversation(
    conn: &Connection,
    client_id: &str,
) -> Result<String, AppError> {
    let title = "CRM Import: Call History";
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM conversations WHERE client_id = ?1 AND title = ?2 AND is_active = 1",
            rusqlite::params![client_id, title],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO conversations (id, client_id, title, status) VALUES (?1, ?2, ?3, 'CLOSED')",
        rusqlite::params![id, client_id, title],
    )?;
    Ok(id)
}

/// Import call_log records from an external unencrypted SQLite database into Compass.
pub fn import_call_log_from_db(
    app_conn: &Connection,
    source_path: &str,
) -> Result<ActivityImportResult, AppError> {
    if !Path::new(source_path).exists() {
        return Err(AppError::Import(format!(
            "Source database not found: {}",
            source_path
        )));
    }

    // Open source DB read-only
    let source_conn = Connection::open_with_flags(
        source_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| AppError::Import(format!("Failed to open source database: {}", e)))?;

    // Read all call_log rows joined with leads
    let mut stmt = source_conn
        .prepare(
            "SELECT
                cl.contact_type,
                cl.disposition,
                cl.notes,
                cl.call_date,
                cl.follow_up_date,
                l.mbi,
                l.first_name,
                l.last_name,
                l.dob,
                l.phone
             FROM call_log cl
             JOIN leads l ON cl.lead_id = l.id
             ORDER BY cl.call_date",
        )
        .map_err(|e| AppError::Import(format!("Failed to prepare source query: {}", e)))?;

    let rows: Vec<SourceCallLog> = stmt
        .query_map([], |row| {
            Ok(SourceCallLog {
                contact_type: row.get(0)?,
                disposition: row.get(1)?,
                notes: row.get(2)?,
                call_date: row.get(3)?,
                follow_up_date: row.get(4)?,
                lead_mbi: row.get(5)?,
                lead_first_name: row.get(6)?,
                lead_last_name: row.get(7)?,
                lead_dob: row.get(8)?,
                lead_phone: row.get(9)?,
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
        let lead_label = format!(
            "{} {} ({})",
            src.lead_first_name.as_deref().unwrap_or("?"),
            src.lead_last_name.as_deref().unwrap_or("?"),
            src.call_date.as_deref().unwrap_or("no date"),
        );

        // Match lead to Compass client using canonical matching
        let client_id: Option<String> = {
            let mbi = src.lead_mbi.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
            let first = src.lead_first_name.as_deref().unwrap_or("").trim();
            let last = src.lead_last_name.as_deref().unwrap_or("").trim();
            let dob = src.lead_dob.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

            if first.is_empty() && last.is_empty() && mbi.is_none() {
                None
            } else {
                matching::find_client_match(
                    app_conn,
                    mbi,
                    first,
                    last,
                    dob,
                    &matching::MatchOptions::default(),
                )
                .map(|m| m.client_id)
            }
        };

        let client_id = match client_id {
            Some(id) => id,
            None => {
                unmatched += 1;
                unmatched_details.push(ImportRowDetail {
                    label: lead_label,
                    detail: format!(
                        "MBI: {}",
                        src.lead_mbi.as_deref().unwrap_or("(none)")
                    ),
                });
                continue;
            }
        };

        let entry_type = if src.contact_type.to_lowercase() == "call" {
            "CALL"
        } else {
            "SMS"
        };

        let subject = &src.disposition;
        let occurred_at = src.call_date.as_deref();

        // Idempotency: skip if same client_id + entry_type + occurred_at + subject exists
        let already_exists: bool = app_conn
            .query_row(
                "SELECT COUNT(*) FROM conversation_entries
                 WHERE client_id = ?1 AND entry_type = ?2 AND occurred_at = ?3 AND subject = ?4 AND is_active = 1",
                rusqlite::params![client_id, entry_type, occurred_at, subject],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if already_exists {
            skipped += 1;
            skipped_details.push(ImportRowDetail {
                label: lead_label,
                detail: "Duplicate entry".to_string(),
            });
            continue;
        }

        let conversation_id = find_or_create_import_conversation(app_conn, &client_id)?;
        let entry_id = Uuid::new_v4().to_string();

        // For calls: set call_direction, call_outcome, call_phone_number
        // For texts: leave call fields NULL
        let (call_direction, call_outcome, call_phone_number): (
            Option<&str>,
            Option<&str>,
            Option<&str>,
        ) = if entry_type == "CALL" {
            (
                Some("OUTBOUND"),
                Some(map_disposition_to_call_outcome(subject)),
                src.lead_phone.as_deref(),
            )
        } else {
            (None, None, None)
        };

        app_conn.execute(
            "INSERT INTO conversation_entries
                (id, conversation_id, client_id, entry_type, subject, body, occurred_at,
                 follow_up_date, call_direction, call_outcome, call_phone_number)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                entry_id,
                conversation_id,
                client_id,
                entry_type,
                subject,
                src.notes,
                occurred_at,
                src.follow_up_date,
                call_direction,
                call_outcome,
                call_phone_number,
            ],
        )?;

        imported += 1;
        imported_details.push(ImportRowDetail {
            label: lead_label,
            detail: format!("{} - {}", entry_type, subject),
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
