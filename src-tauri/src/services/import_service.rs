use std::collections::HashMap;
use calamine::Reader;
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

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: usize,
    pub total: usize,
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
                "address_line1",
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
                "address_line2",
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
            vec!["dual status", "dual_status", "dual status code", "dual"],
        ),
        (
            "lis_level",
            vec!["lis", "lis level", "lis_level", "low income subsidy"],
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
) -> Result<ImportResult, AppError> {
    let mut inserted = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for row in rows {
        match import_single_row(conn, row, headers, mapping) {
            Ok(action) => match action {
                ImportAction::Inserted => inserted += 1,
                ImportAction::Updated => updated += 1,
                ImportAction::Skipped => skipped += 1,
            },
            Err(e) => {
                tracing::warn!("Import row error: {}", e);
                errors += 1;
            }
        }
    }

    Ok(ImportResult {
        inserted,
        updated,
        skipped,
        errors,
        total: inserted + updated + skipped + errors,
    })
}

enum ImportAction {
    Inserted,
    Updated,
    Skipped,
}

fn import_single_row(
    conn: &Connection,
    row: &[String],
    headers: &[String],
    mapping: &HashMap<String, String>,
) -> Result<ImportAction, AppError> {
    let get_val = |target: &str| -> Option<String> {
        let idx = find_mapped_index(headers, mapping, target)?;
        let val = row.get(idx)?.trim().to_string();
        if val.is_empty() {
            None
        } else {
            Some(val)
        }
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

    if let Some(client_id) = existing_id {
        // Update existing client
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        macro_rules! set_if {
            ($field:expr, $col:expr) => {
                if let Some(val) = get_val($field) {
                    sets.push(format!("{} = ?{}", $col, idx));
                    params.push(Box::new(val));
                    idx += 1;
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

        if sets.is_empty() {
            return Ok(ImportAction::Skipped);
        }

        let sql = format!(
            "UPDATE clients SET {} WHERE id = ?{}",
            sets.join(", "),
            idx
        );
        params.push(Box::new(client_id));
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;
        Ok(ImportAction::Updated)
    } else {
        // Insert new client
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO clients (id, first_name, last_name, middle_name, dob, gender, phone, phone2, email,
             address_line1, address_line2, city, state, zip, county, mbi, lead_source, dual_status_code, lis_level, medicaid_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
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
                get_val("medicaid_id")
            ],
        )?;
        Ok(ImportAction::Inserted)
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
