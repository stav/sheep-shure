use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use crate::services::matching::{self, MatchOptions};

// Re-export normalization functions so existing `use super::shared::` paths work
pub use matching::{normalize_date, normalize_mbi, normalize_phone};

/// All client fields as Option<String> for unified upsert.
#[derive(Default)]
pub struct ImportClientData {
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub dob: Option<String>,
    pub gender: Option<String>,
    pub phone: Option<String>,
    pub phone2: Option<String>,
    pub email: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub county: Option<String>,
    pub mbi: Option<String>,
    pub part_a_date: Option<String>,
    pub part_b_date: Option<String>,
    pub is_dual_eligible: Option<bool>,
    pub dual_status_code: Option<String>,
    pub lis_level: Option<String>,
    pub medicaid_id: Option<String>,
    pub lead_source: Option<String>,
    pub tags: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum UpsertAction {
    Inserted,
    Updated,
    Skipped,
}

/// Find an existing client by MBI first, then by name+DOB (with fuzzy matching).
pub fn find_client(
    conn: &Connection,
    mbi: Option<&str>,
    first_name: &str,
    last_name: &str,
    dob: Option<&str>,
) -> Option<String> {
    matching::find_client_match(conn, mbi, first_name, last_name, dob, &MatchOptions::default())
        .map(|m| m.client_id)
}

/// Upsert a client: insert if new, or fill NULL/empty fields on existing record.
/// Returns (client_id, action).
pub fn upsert_client(
    conn: &Connection,
    data: &ImportClientData,
) -> Result<(String, UpsertAction), AppError> {
    let existing_id = find_client(
        conn,
        data.mbi.as_deref(),
        &data.first_name,
        &data.last_name,
        data.dob.as_deref(),
    );

    if let Some(client_id) = existing_id {
        // Update: only fill NULL/empty columns
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;
        let mut updated_fields = Vec::new();

        macro_rules! fill_if_null {
            ($field:expr, $col:expr) => {
                if let Some(ref val) = $field {
                    if !val.is_empty() {
                        let current: Option<String> = conn
                            .query_row(
                                &format!(
                                    "SELECT {} FROM clients WHERE id = ?1",
                                    $col
                                ),
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

        // Fill gaps for name parts (middle_name only)
        fill_if_null!(data.middle_name, "middle_name");
        fill_if_null!(data.dob, "dob");
        fill_if_null!(data.gender, "gender");
        fill_if_null!(data.phone, "phone");
        fill_if_null!(data.phone2, "phone2");
        fill_if_null!(data.email, "email");
        fill_if_null!(data.address_line1, "address_line1");
        fill_if_null!(data.address_line2, "address_line2");
        fill_if_null!(data.city, "city");
        fill_if_null!(data.state, "state");
        fill_if_null!(data.zip, "zip");
        fill_if_null!(data.county, "county");
        fill_if_null!(data.mbi, "mbi");
        fill_if_null!(data.part_a_date, "part_a_date");
        fill_if_null!(data.part_b_date, "part_b_date");
        fill_if_null!(data.dual_status_code, "dual_status_code");
        fill_if_null!(data.lis_level, "lis_level");
        fill_if_null!(data.medicaid_id, "medicaid_id");
        fill_if_null!(data.tags, "tags");

        // is_dual_eligible: only set to true if currently false
        if data.is_dual_eligible == Some(true) {
            let current: bool = conn
                .query_row(
                    "SELECT is_dual_eligible FROM clients WHERE id = ?1",
                    rusqlite::params![client_id],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if !current {
                sets.push(format!("is_dual_eligible = ?{}", idx));
                params.push(Box::new(true));
                idx += 1;
                updated_fields.push("is_dual_eligible".to_string());
            }
        }

        // Notes: append with source prefix rather than overwrite
        if let Some(ref new_notes) = data.notes {
            if !new_notes.is_empty() {
                let current_notes: Option<String> = conn
                    .query_row(
                        "SELECT notes FROM clients WHERE id = ?1",
                        rusqlite::params![client_id],
                        |row| row.get(0),
                    )
                    .ok()
                    .flatten();
                // Only append if not already present (idempotency)
                let should_append = current_notes
                    .as_ref()
                    .map_or(true, |existing| !existing.contains(new_notes.as_str()));
                if should_append {
                    let merged = match current_notes {
                        Some(existing) if !existing.is_empty() => {
                            format!("{}\n\n{}", existing, new_notes)
                        }
                        _ => new_notes.clone(),
                    };
                    sets.push(format!("notes = ?{}", idx));
                    params.push(Box::new(merged));
                    idx += 1;
                    updated_fields.push("notes".to_string());
                }
            }
        }

        if sets.is_empty() {
            return Ok((client_id, UpsertAction::Skipped));
        }

        let sql = format!(
            "UPDATE clients SET {} WHERE id = ?{}",
            sets.join(", "),
            idx
        );
        params.push(Box::new(client_id.clone()));
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

        Ok((client_id, UpsertAction::Updated))
    } else {
        // Insert new client
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO clients (id, first_name, last_name, middle_name, dob, gender,
             phone, phone2, email, address_line1, address_line2, city, state, zip, county,
             mbi, part_a_date, part_b_date, is_dual_eligible, dual_status_code, lis_level,
             medicaid_id, lead_source, tags, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                     ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
            rusqlite::params![
                id,
                data.first_name,
                data.last_name,
                data.middle_name,
                data.dob,
                data.gender,
                data.phone,
                data.phone2,
                data.email,
                data.address_line1,
                data.address_line2,
                data.city,
                data.state,
                data.zip,
                data.county,
                data.mbi,
                data.part_a_date,
                data.part_b_date,
                data.is_dual_eligible.unwrap_or(false),
                data.dual_status_code,
                data.lis_level,
                data.medicaid_id,
                data.lead_source,
                data.tags,
                data.notes,
            ],
        )?;
        Ok((id, UpsertAction::Inserted))
    }
}
