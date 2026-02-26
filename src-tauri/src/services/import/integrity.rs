use std::collections::HashMap;
use std::path::Path;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use super::file_import::ImportRowDetail;
use super::call_log::ActivityImportResult;
use super::shared::{normalize_date, normalize_mbi, normalize_phone, upsert_client, ImportClientData, UpsertAction};

#[derive(serde::Deserialize)]
struct IntegrityExport {
    result: Vec<IntegrityLead>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityLead {
    leads_id: Option<i64>,
    first_name: Option<String>,
    last_name: Option<String>,
    middle_name: Option<String>,
    birthdate: Option<String>,
    gender: Option<String>,
    medicare_beneficiary_id: Option<String>,
    #[serde(rename = "partA")]
    part_a: Option<String>,
    #[serde(rename = "partB")]
    part_b: Option<String>,
    has_medic_aid: Option<i32>,
    subsidy_level: Option<String>,
    status_name: Option<String>,
    notes: Option<String>,
    lead_source: Option<String>,
    #[serde(default)]
    addresses: Vec<IntegrityAddress>,
    #[serde(default)]
    phones: Vec<IntegrityPhone>,
    #[serde(default)]
    emails: Vec<IntegrityEmail>,
    #[serde(default)]
    activities: Vec<IntegrityActivity>,
    #[serde(default)]
    lead_tags: Vec<IntegrityTag>,
    // Fields we skip
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityAddress {
    address1: Option<String>,
    address2: Option<String>,
    city: Option<String>,
    state_code: Option<String>,
    postal_code: Option<String>,
    county: Option<String>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityPhone {
    lead_phone: Option<String>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityEmail {
    lead_email: Option<String>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityActivity {
    activity_type_name: Option<String>,
    activity_note: Option<String>,
    activity_subject: Option<String>,
    activity_body: Option<String>,
    create_date: Option<String>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityTag {
    tag: Option<IntegrityTagInner>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityTagInner {
    tag_label: Option<String>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

/// Import clients and activities from an Integrity JSON export.
pub fn import_integrity_from_json(
    conn: &Connection,
    source_path: &str,
) -> Result<ActivityImportResult, AppError> {
    if !Path::new(source_path).exists() {
        return Err(AppError::Import(format!(
            "Source file not found: {}",
            source_path
        )));
    }

    let file_content = std::fs::read_to_string(source_path)
        .map_err(|e| AppError::Import(format!("Failed to read JSON file: {}", e)))?;

    let export: IntegrityExport = serde_json::from_str(&file_content)
        .map_err(|e| AppError::Import(format!("Failed to parse Integrity JSON: {}", e)))?;

    let total_source_rows = export.result.len();
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut unmatched = 0usize; // used for errors here
    let mut imported_details = Vec::new();
    let mut skipped_details = Vec::new();
    let mut unmatched_details = Vec::new();

    for lead in &export.result {
        let first_name = match lead.first_name.as_deref() {
            Some(n) if !n.trim().is_empty() => n.trim().to_string(),
            _ => {
                unmatched += 1;
                unmatched_details.push(ImportRowDetail {
                    label: format!("Lead #{}", lead.leads_id.unwrap_or(0)),
                    detail: "Missing first name".to_string(),
                });
                continue;
            }
        };
        let last_name = match lead.last_name.as_deref() {
            Some(n) if !n.trim().is_empty() => n.trim().to_string(),
            _ => {
                unmatched += 1;
                unmatched_details.push(ImportRowDetail {
                    label: first_name.clone(),
                    detail: "Missing last name".to_string(),
                });
                continue;
            }
        };

        let mbi = lead
            .medicare_beneficiary_id
            .as_deref()
            .and_then(|s| normalize_mbi(s));
        let dob = lead.birthdate.as_deref().and_then(normalize_date);

        // Build tags from statusName + leadTags
        let mut tag_parts = Vec::new();
        if let Some(ref status) = lead.status_name {
            if !status.is_empty() {
                tag_parts.push(status.clone());
            }
        }
        for lt in &lead.lead_tags {
            if let Some(ref tag_inner) = lt.tag {
                if let Some(ref label) = tag_inner.tag_label {
                    if !label.is_empty() {
                        tag_parts.push(label.clone());
                    }
                }
            }
        }
        let tags = if tag_parts.is_empty() {
            None
        } else {
            Some(tag_parts.join(", "))
        };

        // Subsidy level: skip "Not Answered" and "No"
        let lis_level = lead.subsidy_level.as_deref().and_then(|s| {
            let s = s.trim();
            if s.eq_ignore_ascii_case("not answered") || s.eq_ignore_ascii_case("no") || s.is_empty()
            {
                None
            } else {
                Some(s.to_string())
            }
        });

        let notes = lead.notes.as_deref().map(|n| format!("[Integrity] {}", n));

        let addr = lead.addresses.first();

        let client_data = ImportClientData {
            first_name: first_name.clone(),
            last_name: last_name.clone(),
            middle_name: lead.middle_name.clone(),
            dob,
            gender: lead.gender.clone(),
            phone: lead
                .phones
                .first()
                .and_then(|p| p.lead_phone.as_deref())
                .and_then(normalize_phone),
            email: lead
                .emails
                .first()
                .and_then(|e| e.lead_email.clone()),
            address_line1: addr.and_then(|a| a.address1.clone()),
            address_line2: addr.and_then(|a| a.address2.clone()),
            city: addr.and_then(|a| a.city.clone()),
            state: addr.and_then(|a| a.state_code.clone()),
            zip: addr.and_then(|a| a.postal_code.clone()),
            county: addr.and_then(|a| a.county.clone()),
            mbi,
            part_a_date: lead.part_a.as_deref().and_then(normalize_date),
            part_b_date: lead.part_b.as_deref().and_then(normalize_date),
            is_dual_eligible: lead.has_medic_aid.map(|v| v != 0),
            dual_status_code: None,
            lis_level,
            medicaid_id: None,
            lead_source: lead.lead_source.clone(),
            tags,
            notes,
            ..Default::default()
        };

        let client_label = format!("{} {}", first_name, last_name);

        match upsert_client(conn, &client_data, Some("integrity")) {
            Ok((client_id, action)) => {
                // Import activities as conversation entries
                let activity_count = import_integrity_activities(conn, &client_id, &lead.activities)?;

                match action {
                    UpsertAction::Inserted => {
                        imported += 1;
                        imported_details.push(ImportRowDetail {
                            label: client_label,
                            detail: format!("New client + {} activities", activity_count),
                        });
                    }
                    UpsertAction::Updated => {
                        imported += 1;
                        imported_details.push(ImportRowDetail {
                            label: client_label,
                            detail: format!("Enriched + {} activities", activity_count),
                        });
                    }
                    UpsertAction::Skipped => {
                        if activity_count > 0 {
                            imported += 1;
                            imported_details.push(ImportRowDetail {
                                label: client_label,
                                detail: format!("{} new activities", activity_count),
                            });
                        } else {
                            skipped += 1;
                            skipped_details.push(ImportRowDetail {
                                label: client_label,
                                detail: "No new data".to_string(),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                unmatched += 1;
                unmatched_details.push(ImportRowDetail {
                    label: client_label,
                    detail: format!("Error: {}", e),
                });
            }
        }
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

/// Import Integrity activities as conversation entries.
fn import_integrity_activities(
    conn: &Connection,
    client_id: &str,
    activities: &[IntegrityActivity],
) -> Result<usize, AppError> {
    if activities.is_empty() {
        return Ok(0);
    }

    let conv_title = "CRM Import: Integrity History";
    let conversation_id: String = conn
        .query_row(
            "SELECT id FROM conversations WHERE client_id = ?1 AND title = ?2 AND is_active = 1",
            rusqlite::params![client_id, conv_title],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO conversations (id, client_id, title, status) VALUES (?1, ?2, ?3, 'CLOSED')",
                rusqlite::params![id, client_id, conv_title],
            )
            .ok();
            id
        });

    let mut count = 0;
    for activity in activities {
        let entry_type = match activity.activity_type_name.as_deref() {
            Some("Call") | Some("call") => "CALL",
            Some("Email") | Some("email") => "EMAIL",
            Some("Meeting") | Some("meeting") => "MEETING",
            Some("SMS") | Some("sms") | Some("Text") => "SMS",
            _ => "NOTE",
        };

        let subject = activity
            .activity_subject
            .as_deref()
            .unwrap_or("");
        let body = {
            let mut parts = Vec::new();
            if let Some(ref b) = activity.activity_body {
                if !b.is_empty() {
                    parts.push(b.as_str());
                }
            }
            if let Some(ref n) = activity.activity_note {
                if !n.is_empty() {
                    parts.push(n.as_str());
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n\n"))
            }
        };
        let occurred_at = activity.create_date.as_deref();

        // Idempotency check
        let already_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM conversation_entries
                 WHERE client_id = ?1 AND entry_type = ?2 AND occurred_at = ?3 AND subject = ?4 AND is_active = 1",
                rusqlite::params![client_id, entry_type, occurred_at, subject],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if already_exists {
            continue;
        }

        let entry_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO conversation_entries
                (id, conversation_id, client_id, entry_type, subject, body, occurred_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                entry_id,
                conversation_id,
                client_id,
                entry_type,
                subject,
                body,
                occurred_at,
            ],
        )?;
        count += 1;
    }

    Ok(count)
}
