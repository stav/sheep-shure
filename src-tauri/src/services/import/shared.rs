use chrono::NaiveDate;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;

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

/// Normalize a date string from various formats into YYYY-MM-DD.
pub fn normalize_date(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    // Try ISO datetime: 2025-07-01T00:00:00 or with fractional seconds
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.format("%Y-%m-%d").to_string());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(dt.format("%Y-%m-%d").to_string());
    }
    // Already ISO date
    if let Ok(d) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    // US format: 07/06/1960
    if let Ok(d) = NaiveDate::parse_from_str(raw, "%m/%d/%Y") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    // LeadsMaster: "Sep 25 1958 12:00AM" — handle variable spacing
    let compressed = raw.replace("  ", " ");
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&compressed, "%b %d %Y %I:%M%p") {
        return Some(dt.format("%Y-%m-%d").to_string());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&compressed, "%b %d %Y %I:%M%P") {
        return Some(dt.format("%Y-%m-%d").to_string());
    }

    None
}

/// Normalize MBI: strip dashes, validate 11 alphanumeric chars.
pub fn normalize_mbi(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if cleaned.len() == 11 {
        Some(cleaned)
    } else {
        None
    }
}

/// Normalize phone: strip non-digits, validate 10 digits.
pub fn normalize_phone(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    // Handle 11-digit numbers starting with 1
    if digits.len() == 11 && digits.starts_with('1') {
        return Some(digits[1..].to_string());
    }
    if digits.len() == 10 {
        Some(digits)
    } else {
        None
    }
}

/// Find an existing client by MBI first, then by name+DOB.
pub fn find_client(
    conn: &Connection,
    mbi: Option<&str>,
    first_name: &str,
    last_name: &str,
    dob: Option<&str>,
) -> Option<String> {
    // Try MBI match first
    if let Some(mbi_val) = mbi {
        if !mbi_val.is_empty() {
            if let Ok(id) = conn.query_row(
                "SELECT id FROM clients WHERE mbi = ?1 AND is_active = 1",
                rusqlite::params![mbi_val],
                |row| row.get::<_, String>(0),
            ) {
                return Some(id);
            }
        }
    }

    // Name + DOB fallback
    if let Some(dob_val) = dob {
        if !dob_val.is_empty() {
            // Try exact match first
            if let Ok(id) = conn.query_row(
                "SELECT id FROM clients WHERE first_name = ?1 AND last_name = ?2 AND dob = ?3 AND is_active = 1",
                rusqlite::params![first_name, last_name, dob_val],
                |row| row.get::<_, String>(0),
            ) {
                return Some(id);
            }
            // Try with first name as prefix (e.g. "Kenneth E" matches "Kenneth")
            let first_base = first_name.split_whitespace().next().unwrap_or(first_name);
            if first_base != first_name {
                if let Ok(id) = conn.query_row(
                    "SELECT id FROM clients WHERE first_name = ?1 AND last_name = ?2 AND dob = ?3 AND is_active = 1",
                    rusqlite::params![first_base, last_name, dob_val],
                    |row| row.get::<_, String>(0),
                ) {
                    return Some(id);
                }
            }
        }
    }

    None
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
