use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;
use calamine::Reader;
use chrono::NaiveDate;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;

/// Parsed file result: headers and sample rows
#[derive(serde::Serialize)]
pub struct ParsedFile {
    pub headers: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub total_rows: usize,
}

/// Validation result for an import
#[derive(serde::Serialize)]
pub struct ValidationResult {
    pub valid_rows: Vec<Vec<String>>,
    pub error_rows: Vec<ErrorRow>,
    pub total: usize,
}

#[derive(serde::Serialize)]
pub struct ErrorRow {
    pub row_number: usize,
    pub data: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct ImportRowDetail {
    pub label: String,
    pub detail: String,
}

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: usize,
    pub total: usize,
    pub inserted_details: Vec<ImportRowDetail>,
    pub updated_details: Vec<ImportRowDetail>,
    pub skipped_details: Vec<ImportRowDetail>,
    pub error_details: Vec<ImportRowDetail>,
}

/// Parse a CSV or XLSX file and return headers + sample rows
pub fn parse_file(file_path: &str) -> Result<ParsedFile, AppError> {
    let lower = file_path.to_lowercase();
    if lower.ends_with(".csv") {
        parse_csv(file_path)
    } else if lower.ends_with(".xlsx") || lower.ends_with(".xls") {
        parse_xlsx(file_path)
    } else {
        Err(AppError::Import(
            "Unsupported file format. Please use CSV or XLSX.".to_string(),
        ))
    }
}

fn parse_csv(file_path: &str) -> Result<ParsedFile, AppError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(file_path)
        .map_err(|e| AppError::Import(format!("Failed to read CSV: {}", e)))?;

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| AppError::Import(format!("Failed to read CSV headers: {}", e)))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut all_rows = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| AppError::Import(format!("CSV parse error: {}", e)))?;
        all_rows.push(record.iter().map(|f| f.to_string()).collect::<Vec<_>>());
    }

    let total_rows = all_rows.len();
    let sample_rows: Vec<Vec<String>> = all_rows.into_iter().take(10).collect();

    Ok(ParsedFile {
        headers,
        sample_rows,
        total_rows,
    })
}

fn parse_xlsx(file_path: &str) -> Result<ParsedFile, AppError> {
    let mut workbook = calamine::open_workbook_auto(file_path)
        .map_err(|e| AppError::Import(format!("Failed to open workbook: {}", e)))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = sheet_names
        .first()
        .ok_or_else(|| AppError::Import("Workbook has no sheets".to_string()))?
        .clone();

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::Import(format!("Failed to read sheet: {}", e)))?;

    let mut rows_iter = range.rows();

    let headers: Vec<String> = rows_iter
        .next()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .unwrap_or_default();

    let mut all_rows = Vec::new();
    for row in rows_iter {
        all_rows.push(row.iter().map(|cell| cell.to_string()).collect::<Vec<_>>());
    }

    let total_rows = all_rows.len();
    let sample_rows: Vec<Vec<String>> = all_rows.into_iter().take(10).collect();

    Ok(ParsedFile {
        headers,
        sample_rows,
        total_rows,
    })
}

/// Auto-map source column headers to target fields using fuzzy matching
pub fn auto_map_columns(headers: &[String]) -> HashMap<String, String> {
    let aliases: HashMap<&str, Vec<&str>> = HashMap::from([
        (
            "first_name",
            vec![
                "first name",
                "first",
                "fname",
                "given name",
                "member first name",
                "first_name",
            ],
        ),
        (
            "last_name",
            vec![
                "last name",
                "last",
                "lname",
                "surname",
                "family name",
                "member last name",
                "last_name",
            ],
        ),
        (
            "middle_name",
            vec!["middle name", "middle", "mname", "mi", "middle_name"],
        ),
        (
            "dob",
            vec![
                "dob",
                "date of birth",
                "birth date",
                "birthdate",
                "birth_date",
                "date_of_birth",
            ],
        ),
        ("gender", vec!["gender", "sex"]),
        (
            "phone",
            vec![
                "phone",
                "phone number",
                "telephone",
                "phone1",
                "primary phone",
                "phone_number",
            ],
        ),
        (
            "phone2",
            vec![
                "phone 2",
                "phone2",
                "secondary phone",
                "alt phone",
                "alternate phone",
            ],
        ),
        (
            "email",
            vec!["email", "email address", "e-mail", "email_address"],
        ),
        (
            "address_line1",
            vec![
                "address",
                "address line 1",
                "address1",
                "street",
                "street address",
                "street address 1",
                "street address1",
                "address_line1",
                "street_address1",
            ],
        ),
        (
            "address_line2",
            vec![
                "address 2",
                "address line 2",
                "address2",
                "apt",
                "suite",
                "unit",
                "street address 2",
                "street address2",
                "address_line2",
                "street_address2",
            ],
        ),
        ("city", vec!["city", "town"]),
        ("state", vec!["state", "st", "state code", "state_code"]),
        (
            "zip",
            vec![
                "zip",
                "zip code",
                "zipcode",
                "postal code",
                "postal",
                "zip_code",
            ],
        ),
        ("county", vec!["county", "county name", "county_name"]),
        (
            "mbi",
            vec![
                "mbi",
                "medicare id",
                "medicare beneficiary identifier",
                "hicn",
                "medicare_id",
                "member id",
                "member_id",
            ],
        ),
        (
            "part_a_date",
            vec![
                "part a date",
                "part_a_date",
                "part a",
                "part a effective",
            ],
        ),
        (
            "part_b_date",
            vec![
                "part b date",
                "part_b_date",
                "part b",
                "part b effective",
            ],
        ),
        (
            "plan_name",
            vec![
                "plan",
                "plan name",
                "plan_name",
                "product name",
                "product",
            ],
        ),
        (
            "carrier_name",
            vec![
                "carrier",
                "carrier name",
                "carrier_name",
                "insurance company",
                "company",
            ],
        ),
        (
            "plan_type_code",
            vec![
                "plan type",
                "plan_type",
                "plan type code",
                "product type",
                "line of business",
                "lob",
            ],
        ),
        (
            "effective_date",
            vec![
                "effective date",
                "effective",
                "eff date",
                "eff_date",
                "effective_date",
                "start date",
            ],
        ),
        (
            "termination_date",
            vec![
                "termination date",
                "term date",
                "term_date",
                "termination_date",
                "end date",
                "disenrollment date",
            ],
        ),
        (
            "premium",
            vec!["premium", "monthly premium", "plan premium"],
        ),
        (
            "contract_number",
            vec![
                "contract",
                "contract number",
                "contract_number",
                "contract id",
            ],
        ),
        (
            "pbp_number",
            vec!["pbp", "pbp number", "pbp_number", "pbp id"],
        ),
        (
            "confirmation_number",
            vec![
                "confirmation",
                "confirmation number",
                "confirmation_number",
                "app id",
                "application id",
            ],
        ),
        (
            "lead_source",
            vec!["lead source", "source", "lead_source", "referral source"],
        ),
        (
            "dual_status_code",
            vec!["dual status", "dual_status", "dual status code", "dual", "medicaid level", "medicaid_level"],
        ),
        (
            "lis_level",
            vec!["lis", "lis level", "lis_level", "low income subsidy", "lis copay level", "lis_copay_level"],
        ),
        (
            "medicaid_id",
            vec![
                "medicaid id",
                "medicaid_id",
                "medicaid number",
                "medicaid",
            ],
        ),
        (
            "notes",
            vec!["notes", "note", "comments", "comment", "remarks"],
        ),
    ]);

    let mut mapping = HashMap::new();

    for header in headers {
        let normalized = header.trim().to_lowercase().replace(['_', '-'], " ");

        for (target, alias_list) in &aliases {
            if alias_list.iter().any(|a| *a == normalized) {
                mapping.insert(header.clone(), target.to_string());
                break;
            }
        }
    }

    mapping
}

/// Validate import rows based on column mapping
pub fn validate_rows(
    all_rows: &[Vec<String>],
    headers: &[String],
    mapping: &HashMap<String, String>,
) -> ValidationResult {
    let mut valid_rows = Vec::new();
    let mut error_rows = Vec::new();

    // Find index for key columns
    let first_name_idx = find_mapped_index(headers, mapping, "first_name");
    let last_name_idx = find_mapped_index(headers, mapping, "last_name");
    let mbi_idx = find_mapped_index(headers, mapping, "mbi");

    for (i, row) in all_rows.iter().enumerate() {
        let mut errors = Vec::new();

        // Check required fields
        if let Some(idx) = first_name_idx {
            if row.get(idx).map_or(true, |v| v.trim().is_empty()) {
                errors.push("Missing first name".to_string());
            }
        }
        if let Some(idx) = last_name_idx {
            if row.get(idx).map_or(true, |v| v.trim().is_empty()) {
                errors.push("Missing last name".to_string());
            }
        }

        // Validate MBI format if present
        if let Some(idx) = mbi_idx {
            if let Some(mbi) = row.get(idx) {
                let mbi = mbi.trim();
                if !mbi.is_empty()
                    && (mbi.len() != 11 || !mbi.chars().all(|c| c.is_ascii_alphanumeric()))
                {
                    errors.push(format!("Invalid MBI format: '{}'", mbi));
                }
            }
        }

        if errors.is_empty() {
            valid_rows.push(row.clone());
        } else {
            error_rows.push(ErrorRow {
                row_number: i + 1, // 1-indexed
                data: row.clone(),
                errors,
            });
        }
    }

    let total = valid_rows.len() + error_rows.len();
    ValidationResult {
        valid_rows,
        error_rows,
        total,
    }
}

/// Execute the actual import - insert/update clients
pub fn execute_import(
    conn: &Connection,
    rows: &[Vec<String>],
    headers: &[String],
    mapping: &HashMap<String, String>,
    constant_values: &HashMap<String, String>,
) -> Result<ImportResult, AppError> {
    let mut inserted = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;
    let mut inserted_details = Vec::new();
    let mut updated_details = Vec::new();
    let mut skipped_details = Vec::new();
    let mut error_details = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        match import_single_row(conn, row, headers, mapping, constant_values) {
            Ok(action) => match action {
                ImportAction::Inserted { name } => {
                    inserted += 1;
                    inserted_details.push(ImportRowDetail { label: name, detail: String::new() });
                }
                ImportAction::Updated { name, fields } => {
                    updated += 1;
                    updated_details.push(ImportRowDetail { label: name, detail: fields.join(", ") });
                }
                ImportAction::Skipped { name } => {
                    skipped += 1;
                    skipped_details.push(ImportRowDetail { label: name, detail: "No new data".to_string() });
                }
            },
            Err(e) => {
                tracing::warn!("Import row error: {}", e);
                errors += 1;
                error_details.push(ImportRowDetail {
                    label: format!("Row {}", i + 1),
                    detail: e.to_string(),
                });
            }
        }
    }

    Ok(ImportResult {
        inserted,
        updated,
        skipped,
        errors,
        total: inserted + updated + skipped + errors,
        inserted_details,
        updated_details,
        skipped_details,
        error_details,
    })
}

enum ImportAction {
    Inserted { name: String },
    Updated { name: String, fields: Vec<String> },
    Skipped { name: String },
}

fn import_single_row(
    conn: &Connection,
    row: &[String],
    headers: &[String],
    mapping: &HashMap<String, String>,
    constant_values: &HashMap<String, String>,
) -> Result<ImportAction, AppError> {
    let get_val = |target: &str| -> Option<String> {
        // Try column mapping first
        if let Some(idx) = find_mapped_index(headers, mapping, target) {
            let val = row.get(idx)?.trim().to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
        // Fall back to constant value
        constant_values.get(target).filter(|v| !v.is_empty()).cloned()
    };

    let first_name =
        get_val("first_name").ok_or_else(|| AppError::Import("Missing first name".into()))?;
    let last_name =
        get_val("last_name").ok_or_else(|| AppError::Import("Missing last name".into()))?;
    let mbi = get_val("mbi");

    // Try to find existing client by MBI first, then by name+DOB
    let existing_id: Option<String> = if let Some(ref mbi_val) = mbi {
        conn.query_row(
            "SELECT id FROM clients WHERE mbi = ?1 AND is_active = 1",
            rusqlite::params![mbi_val],
            |row| row.get(0),
        )
        .ok()
    } else {
        let dob = get_val("dob");
        if let Some(ref dob_val) = dob {
            conn.query_row(
                "SELECT id FROM clients WHERE first_name = ?1 AND last_name = ?2 AND dob = ?3 AND is_active = 1",
                rusqlite::params![first_name, last_name, dob_val],
                |row| row.get(0),
            )
            .ok()
        } else {
            None
        }
    };

    let client_name = format!("{} {}", first_name, last_name);

    if let Some(client_id) = existing_id {
        // Update existing client
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;
        let mut updated_fields = Vec::new();

        macro_rules! set_if {
            ($field:expr, $col:expr) => {
                if let Some(val) = get_val($field) {
                    sets.push(format!("{} = ?{}", $col, idx));
                    params.push(Box::new(val));
                    idx += 1;
                    updated_fields.push($col.to_string());
                }
            };
        }

        set_if!("phone", "phone");
        set_if!("email", "email");
        set_if!("address_line1", "address_line1");
        set_if!("address_line2", "address_line2");
        set_if!("city", "city");
        set_if!("state", "state");
        set_if!("zip", "zip");
        set_if!("county", "county");
        set_if!("dual_status_code", "dual_status_code");
        set_if!("lis_level", "lis_level");
        set_if!("medicaid_id", "medicaid_id");
        set_if!("notes", "notes");

        if sets.is_empty() {
            return Ok(ImportAction::Skipped { name: client_name });
        }

        let sql = format!(
            "UPDATE clients SET {} WHERE id = ?{}",
            sets.join(", "),
            idx
        );
        params.push(Box::new(client_id));
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;
        Ok(ImportAction::Updated { name: client_name, fields: updated_fields })
    } else {
        // Insert new client
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO clients (id, first_name, last_name, middle_name, dob, gender, phone, phone2, email,
             address_line1, address_line2, city, state, zip, county, mbi, lead_source, dual_status_code, lis_level, medicaid_id, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            rusqlite::params![
                id,
                first_name,
                last_name,
                get_val("middle_name"),
                get_val("dob"),
                get_val("gender"),
                get_val("phone"),
                get_val("phone2"),
                get_val("email"),
                get_val("address_line1"),
                get_val("address_line2"),
                get_val("city"),
                get_val("state"),
                get_val("zip"),
                get_val("county"),
                mbi,
                get_val("lead_source"),
                get_val("dual_status_code"),
                get_val("lis_level"),
                get_val("medicaid_id"),
                get_val("notes")
            ],
        )?;
        Ok(ImportAction::Inserted { name: client_name })
    }
}

fn find_mapped_index(
    headers: &[String],
    mapping: &HashMap<String, String>,
    target: &str,
) -> Option<usize> {
    for (source, mapped) in mapping {
        if mapped == target {
            return headers.iter().position(|h| h == source);
        }
    }
    None
}

/// Get all rows from a file (not just sample)
pub fn get_all_rows(file_path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
    let lower = file_path.to_lowercase();
    if lower.ends_with(".csv") {
        get_all_rows_csv(file_path)
    } else if lower.ends_with(".xlsx") || lower.ends_with(".xls") {
        get_all_rows_xlsx(file_path)
    } else {
        Err(AppError::Import(
            "Unsupported file format".to_string(),
        ))
    }
}

fn get_all_rows_csv(file_path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(file_path)
        .map_err(|e| AppError::Import(format!("Failed to read CSV: {}", e)))?;

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| AppError::Import(format!("Failed to read CSV headers: {}", e)))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| AppError::Import(format!("CSV parse error: {}", e)))?;
        rows.push(record.iter().map(|f| f.to_string()).collect());
    }
    Ok((headers, rows))
}

fn get_all_rows_xlsx(file_path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
    let mut workbook = calamine::open_workbook_auto(file_path)
        .map_err(|e| AppError::Import(format!("Failed to open workbook: {}", e)))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = sheet_names
        .first()
        .ok_or_else(|| AppError::Import("No sheets".to_string()))?
        .clone();

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::Import(format!("Failed to read sheet: {}", e)))?;

    let mut rows_iter = range.rows();

    let headers: Vec<String> = rows_iter
        .next()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .unwrap_or_default();

    let rows = rows_iter
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .collect();

    Ok((headers, rows))
}

// =============================================================================
// Call Log Import from external CRM SQLite database
// =============================================================================

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

        // Match lead to Compass client: MBI first, then name+DOB fallback
        let client_id: Option<String> = if let Some(ref mbi) = src.lead_mbi {
            if !mbi.trim().is_empty() {
                app_conn
                    .query_row(
                        "SELECT id FROM clients WHERE mbi = ?1 AND is_active = 1",
                        rusqlite::params![mbi.trim()],
                        |row| row.get(0),
                    )
                    .ok()
            } else {
                None
            }
        } else {
            None
        }
        .or_else(|| {
            let first = src.lead_first_name.as_deref()?.trim();
            let last = src.lead_last_name.as_deref()?.trim();
            let dob = src.lead_dob.as_deref()?.trim();
            if first.is_empty() || last.is_empty() || dob.is_empty() {
                return None;
            }
            app_conn
                .query_row(
                    "SELECT id FROM clients WHERE first_name = ?1 AND last_name = ?2 AND dob = ?3 AND is_active = 1",
                    rusqlite::params![first, last, dob],
                    |row| row.get(0),
                )
                .ok()
        });

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

// =============================================================================
// Shared Import Infrastructure: normalizers, matching, upsert
// =============================================================================

/// Normalize a date string from various formats into YYYY-MM-DD.
fn normalize_date(raw: &str) -> Option<String> {
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
fn normalize_mbi(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if cleaned.len() == 11 {
        Some(cleaned)
    } else {
        None
    }
}

/// Normalize phone: strip non-digits, validate 10 digits.
fn normalize_phone(raw: &str) -> Option<String> {
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

/// All client fields as Option<String> for unified upsert.
#[derive(Default)]
struct ImportClientData {
    first_name: String,
    last_name: String,
    middle_name: Option<String>,
    dob: Option<String>,
    gender: Option<String>,
    phone: Option<String>,
    phone2: Option<String>,
    email: Option<String>,
    address_line1: Option<String>,
    address_line2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    zip: Option<String>,
    county: Option<String>,
    mbi: Option<String>,
    part_a_date: Option<String>,
    part_b_date: Option<String>,
    is_dual_eligible: Option<i32>,
    dual_status_code: Option<String>,
    lis_level: Option<String>,
    medicaid_id: Option<String>,
    lead_source: Option<String>,
    tags: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum UpsertAction {
    Inserted,
    Updated,
    Skipped,
}

/// Find an existing client by MBI first, then by name+DOB.
fn find_client(
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
fn upsert_client(
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

        // is_dual_eligible: only set to 1 if currently 0
        if let Some(1) = data.is_dual_eligible {
            let current: i32 = conn
                .query_row(
                    "SELECT is_dual_eligible FROM clients WHERE id = ?1",
                    rusqlite::params![client_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if current == 0 {
                sets.push(format!("is_dual_eligible = ?{}", idx));
                params.push(Box::new(1i32));
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
                data.is_dual_eligible.unwrap_or(0),
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

// =============================================================================
// Integrity JSON Import
// =============================================================================

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
            is_dual_eligible: lead.has_medic_aid,
            dual_status_code: None,
            lis_level,
            medicaid_id: None,
            lead_source: lead.lead_source.clone(),
            tags,
            notes,
            ..Default::default()
        };

        let client_label = format!("{} {}", first_name, last_name);

        match upsert_client(conn, &client_data) {
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

// =============================================================================
// Sirem pg_dump Import
// =============================================================================

/// Parse a COPY block from a pg_dump file.
/// Returns (column_names, rows) where each row is a Vec<Option<String>>.
fn parse_copy_block(
    lines: &[String],
    table_name: &str,
) -> (Vec<String>, Vec<Vec<Option<String>>>) {
    let prefix = format!("COPY public.{} (", table_name);
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut in_block = false;

    for line in lines {
        if !in_block {
            if line.starts_with(&prefix) {
                // Parse column names from: COPY public.table (col1, col2, ...) FROM stdin;
                let start = prefix.len();
                if let Some(end) = line.find(") FROM stdin;") {
                    let cols_str = &line[start..end];
                    columns = cols_str.split(", ").map(|s| s.trim().to_string()).collect();
                }
                in_block = true;
            }
        } else if line == "\\." {
            break;
        } else {
            // Parse tab-separated row, handling \N as NULL and COPY escaping
            let fields: Vec<Option<String>> = line
                .split('\t')
                .map(|field| {
                    if field == "\\N" {
                        None
                    } else {
                        // Unescape COPY format: \n -> newline, \t -> tab, \\ -> backslash
                        let unescaped = field
                            .replace("\\n", "\n")
                            .replace("\\t", "\t")
                            .replace("\\\\", "\\");
                        Some(unescaped)
                    }
                })
                .collect();
            rows.push(fields);
        }
    }

    (columns, rows)
}

/// Helper to get a field value from a COPY row by column name.
fn get_copy_field<'a>(
    row: &'a [Option<String>],
    columns: &[String],
    col_name: &str,
) -> Option<&'a str> {
    columns
        .iter()
        .position(|c| c == col_name)
        .and_then(|idx| row.get(idx))
        .and_then(|v| v.as_deref())
}

/// Map a Sirem carrier name to Compass carrier_id.
fn map_sirem_carrier(carrier: &str) -> Option<&'static str> {
    let lower = carrier.to_lowercase();
    if lower.contains("humana") {
        Some("carrier-humana")
    } else if lower.contains("devoted") {
        Some("carrier-devoted")
    } else if lower.contains("anthem") {
        Some("carrier-anthem")
    } else if lower.contains("aetna") {
        Some("carrier-aetna")
    } else if lower.contains("caresource") {
        Some("carrier-caresource")
    } else if lower.contains("medical mutual") || lower.contains("medmutual") {
        Some("carrier-medmutual")
    } else if lower.contains("united") || lower.contains("uhc") {
        Some("carrier-uhc")
    } else if lower.contains("wellcare") {
        Some("carrier-wellcare")
    } else if lower.contains("cigna") {
        Some("carrier-cigna")
    } else if lower.contains("molina") {
        Some("carrier-molina")
    } else if lower.contains("zing") {
        Some("carrier-zing")
    } else if lower.contains("summacare") {
        Some("carrier-summacare")
    } else if lower.contains("silverscript") {
        Some("carrier-ss")
    } else if lower.contains("bcbs") || lower.contains("blue cross") {
        Some("carrier-bcbs")
    } else {
        None
    }
}

/// Map Sirem type_program + type_snp to Compass plan_type_code.
fn map_sirem_plan_type(type_program: Option<&str>, type_snp: Option<&str>) -> &'static str {
    match (type_program, type_snp) {
        (_, Some("D")) => "DSNP",
        (_, Some("C")) => "CSNP",
        (_, Some("I")) => "ISNP",
        (Some("MAPD"), _) => "MAPD",
        (Some("MA"), _) => "MA",
        (Some("PDP"), _) => "PDP",
        (Some("SNP"), _) => "DSNP",
        _ => "MAPD", // default
    }
}

/// Import clients and enrollments from a Sirem pg_dump file.
pub fn import_sirem_from_dump(
    conn: &Connection,
    dump_path: &str,
) -> Result<ActivityImportResult, AppError> {
    if !Path::new(dump_path).exists() {
        return Err(AppError::Import(format!(
            "Dump file not found: {}",
            dump_path
        )));
    }

    // Read entire file
    let file = std::fs::File::open(dump_path)
        .map_err(|e| AppError::Import(format!("Failed to open dump: {}", e)))?;
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .map(|l| l.unwrap_or_default())
        .collect();

    // Parse relevant COPY blocks
    let (contact_cols, contact_rows) = parse_copy_block(&lines, "contacts");
    let (addr_cols, addr_rows) = parse_copy_block(&lines, "addresses");
    let (enroll_cols, enroll_rows) = parse_copy_block(&lines, "enrollments");
    let (plan_cols, plan_rows) = parse_copy_block(&lines, "plans");

    // Build lookup maps: sirem_id -> data
    let mut addr_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, row) in addr_rows.iter().enumerate() {
        if let Some(contact_id) = get_copy_field(row, &addr_cols, "contact_id") {
            addr_map
                .entry(contact_id.to_string())
                .or_default()
                .push(i);
        }
    }

    let mut plan_map: HashMap<String, usize> = HashMap::new();
    for (i, row) in plan_rows.iter().enumerate() {
        if let Some(plan_id) = get_copy_field(row, &plan_cols, "id") {
            plan_map.insert(plan_id.to_string(), i);
        }
    }

    // Map Sirem contact UUID -> Compass client UUID
    let mut sirem_to_compass: HashMap<String, String> = HashMap::new();

    let total_source_rows = contact_rows.len();
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut unmatched = 0usize;
    let mut imported_details = Vec::new();
    let mut skipped_details = Vec::new();
    let mut unmatched_details = Vec::new();

    // Phase 1: Import contacts
    for row in &contact_rows {
        let sirem_id = match get_copy_field(row, &contact_cols, "id") {
            Some(id) => id.to_string(),
            None => continue,
        };

        let first_name = match get_copy_field(row, &contact_cols, "first_name") {
            Some(n) if !n.trim().is_empty() => n.trim().to_string(),
            _ => {
                unmatched += 1;
                unmatched_details.push(ImportRowDetail {
                    label: sirem_id.clone(),
                    detail: "Missing first name".to_string(),
                });
                continue;
            }
        };
        let last_name = match get_copy_field(row, &contact_cols, "last_name") {
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

        let mbi = get_copy_field(row, &contact_cols, "medicare_beneficiary_id")
            .and_then(normalize_mbi);
        let dob = get_copy_field(row, &contact_cols, "birthdate")
            .and_then(normalize_date);
        let phone = get_copy_field(row, &contact_cols, "phone")
            .and_then(normalize_phone);
        let email = get_copy_field(row, &contact_cols, "email")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let has_medicaid = get_copy_field(row, &contact_cols, "has_medicaid")
            .map(|v| if v == "t" { Some(1) } else { None })
            .unwrap_or(None);

        let subsidy = get_copy_field(row, &contact_cols, "subsidy_level").and_then(|s| {
            let s = s.trim();
            if s.eq_ignore_ascii_case("not answered") || s.eq_ignore_ascii_case("no") || s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });

        // Get address from addr_map
        let addr_idx = addr_map
            .get(&sirem_id)
            .and_then(|indices| indices.first())
            .copied();
        let addr_row = addr_idx.map(|i| &addr_rows[i]);

        let part_a = get_copy_field(row, &contact_cols, "part_a_status")
            .and_then(normalize_date);
        let part_b = get_copy_field(row, &contact_cols, "part_b_status")
            .and_then(normalize_date);

        let notes = get_copy_field(row, &contact_cols, "notes")
            .map(|n| format!("[Sirem] {}", n));

        let client_data = ImportClientData {
            first_name: first_name.clone(),
            last_name: last_name.clone(),
            middle_name: get_copy_field(row, &contact_cols, "middle_name")
                .map(|s| s.to_string()),
            dob,
            gender: get_copy_field(row, &contact_cols, "gender")
                .map(|s| s.to_string()),
            phone,
            email,
            address_line1: addr_row
                .and_then(|r| get_copy_field(r, &addr_cols, "address1").map(|s| s.to_string())),
            address_line2: addr_row
                .and_then(|r| get_copy_field(r, &addr_cols, "address2").map(|s| s.to_string())),
            city: addr_row
                .and_then(|r| get_copy_field(r, &addr_cols, "city").map(|s| s.to_string())),
            state: addr_row
                .and_then(|r| get_copy_field(r, &addr_cols, "state_code").map(|s| s.to_string())),
            zip: addr_row
                .and_then(|r| get_copy_field(r, &addr_cols, "postal_code").map(|s| s.to_string())),
            county: addr_row
                .and_then(|r| get_copy_field(r, &addr_cols, "county").map(|s| s.to_string())),
            mbi,
            part_a_date: part_a,
            part_b_date: part_b,
            is_dual_eligible: has_medicaid,
            dual_status_code: None,
            lis_level: subsidy,
            medicaid_id: None,
            lead_source: get_copy_field(row, &contact_cols, "lead_source")
                .map(|s| s.to_string()),
            tags: None,
            notes,
            ..Default::default()
        };

        let client_label = format!("{} {}", first_name, last_name);

        match upsert_client(conn, &client_data) {
            Ok((client_id, action)) => {
                sirem_to_compass.insert(sirem_id, client_id);
                match action {
                    UpsertAction::Inserted => {
                        imported += 1;
                        imported_details.push(ImportRowDetail {
                            label: client_label,
                            detail: "New client".to_string(),
                        });
                    }
                    UpsertAction::Updated => {
                        imported += 1;
                        imported_details.push(ImportRowDetail {
                            label: client_label,
                            detail: "Enriched".to_string(),
                        });
                    }
                    UpsertAction::Skipped => {
                        skipped += 1;
                        skipped_details.push(ImportRowDetail {
                            label: client_label,
                            detail: "No new data".to_string(),
                        });
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

    // Phase 2: Import enrollments
    let mut enroll_imported = 0usize;
    let mut enroll_skipped = 0usize;
    for row in &enroll_rows {
        let sirem_contact_id = match get_copy_field(row, &enroll_cols, "contact_id") {
            Some(id) => id.to_string(),
            None => continue,
        };

        let client_id = match sirem_to_compass.get(&sirem_contact_id) {
            Some(id) => id.clone(),
            None => continue, // contact wasn't imported
        };

        let sirem_plan_id = get_copy_field(row, &enroll_cols, "plan_id");
        let plan_row = sirem_plan_id.and_then(|pid| plan_map.get(pid)).map(|&i| &plan_rows[i]);

        let carrier_name = plan_row.and_then(|r| get_copy_field(r, &plan_cols, "carrier"));
        let carrier_id = carrier_name.and_then(map_sirem_carrier);
        let plan_name = plan_row
            .and_then(|r| get_copy_field(r, &plan_cols, "name"))
            .unwrap_or("Unknown Plan");
        let type_program = plan_row.and_then(|r| get_copy_field(r, &plan_cols, "type_program"));
        let type_snp = plan_row.and_then(|r| get_copy_field(r, &plan_cols, "type_snp"));
        let plan_type_code = map_sirem_plan_type(type_program, type_snp);
        let contract_number =
            plan_row.and_then(|r| get_copy_field(r, &plan_cols, "cms_contract_number"));
        let pbp_number = plan_row.and_then(|r| get_copy_field(r, &plan_cols, "cms_plan_number"));

        let effective_date = get_copy_field(row, &enroll_cols, "coverage_effective_date")
            .and_then(normalize_date);
        let termination_date = get_copy_field(row, &enroll_cols, "coverage_end_date")
            .and_then(normalize_date);

        let enrollment_status = get_copy_field(row, &enroll_cols, "enrollment_status");
        let status_code = match enrollment_status {
            Some(s) if s.eq_ignore_ascii_case("active") => "ACTIVE",
            Some(s) if s.eq_ignore_ascii_case("terminated") => "DISENROLLED_VOLUNTARY",
            Some(s) if s.eq_ignore_ascii_case("pending") => "PENDING",
            _ => "ACTIVE",
        };

        let premium: Option<f64> = get_copy_field(row, &enroll_cols, "premium_monthly_at_enrollment")
            .and_then(|s| s.parse().ok());

        let pcp_name = get_copy_field(row, &enroll_cols, "pcp_name");
        let agent_notes = get_copy_field(row, &enroll_cols, "agent_notes");
        let mut notes_parts = Vec::new();
        if let Some(pcp) = pcp_name {
            if !pcp.is_empty() {
                notes_parts.push(format!("PCP: {}", pcp));
            }
        }
        if let Some(notes) = agent_notes {
            if !notes.is_empty() {
                notes_parts.push(notes.to_string());
            }
        }
        let enroll_notes = if notes_parts.is_empty() {
            None
        } else {
            Some(notes_parts.join(" | "))
        };

        // Idempotency: skip if client_id + plan_name + effective_date already exists
        let eff_str = effective_date.as_deref();
        let already_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM enrollments WHERE client_id = ?1 AND plan_name = ?2 AND effective_date = ?3 AND is_active = 1",
                rusqlite::params![client_id, plan_name, eff_str],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if already_exists {
            enroll_skipped += 1;
            continue;
        }

        let enrollment_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO enrollments (id, client_id, carrier_id, plan_type_code, plan_name,
             contract_number, pbp_number, effective_date, termination_date, status_code,
             premium, enrollment_source, confirmation_number)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                enrollment_id,
                client_id,
                carrier_id,
                plan_type_code,
                plan_name,
                contract_number,
                pbp_number,
                effective_date,
                termination_date,
                status_code,
                premium,
                "Sirem Import",
                enroll_notes,
            ],
        )?;
        enroll_imported += 1;
    }

    // Add enrollment summary to result
    if enroll_imported > 0 || enroll_skipped > 0 {
        imported_details.push(ImportRowDetail {
            label: "Enrollments".to_string(),
            detail: format!("{} imported, {} skipped", enroll_imported, enroll_skipped),
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

// =============================================================================
// LeadsMaster Enrichment
// =============================================================================

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
                let current_dual: i32 = conn
                    .query_row(
                        "SELECT is_dual_eligible FROM clients WHERE id = ?1",
                        rusqlite::params![client_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                if current_dual == 0 {
                    sets.push(format!("is_dual_eligible = ?{}", idx));
                    params.push(Box::new(1i32));
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
        params.push(Box::new(client_id));
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

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
