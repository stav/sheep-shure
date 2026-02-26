use std::collections::HashMap;
use calamine::Reader;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use crate::services::conversation_service;

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
        ("state", vec!["state", "st", "state code", "state_code", "abbreviated state"]),
        (
            "zip",
            vec![
                "zip",
                "zip code",
                "zipcode",
                "postal code",
                "postal",
                "zip_code",
                "5 digit zip code",
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
        let mut normalized = header.trim().to_lowercase().replace(['_', '-'], " ");
        // Strip parenthetical suffixes like "(required)" or "(optional, MM/DD/YYYY)"
        if let Some(pos) = normalized.find('(') {
            normalized = normalized[..pos].trim().to_string();
        }

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
    let dob_idx = find_mapped_index(headers, mapping, "dob");

    // Track seen rows for within-file duplicate detection.
    // Key: (lowercase first_name, lowercase last_name, lowercase mbi_or_empty, dob_or_empty)
    let mut seen: HashMap<(String, String, String, String), usize> = HashMap::new();

    for (i, row) in all_rows.iter().enumerate() {
        let mut errors = Vec::new();

        let first = first_name_idx
            .and_then(|idx| row.get(idx))
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let last = last_name_idx
            .and_then(|idx| row.get(idx))
            .map(|v| v.trim().to_string())
            .unwrap_or_default();

        // Check required fields
        if first.is_empty() {
            errors.push("Missing first name".to_string());
        }
        if last.is_empty() {
            errors.push("Missing last name".to_string());
        }

        // Validate MBI format if present
        let mbi_val = mbi_idx
            .and_then(|idx| row.get(idx))
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if !mbi_val.is_empty()
            && (mbi_val.len() != 11 || !mbi_val.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            errors.push(format!("Invalid MBI format: '{}'", mbi_val));
        }

        // Within-file duplicate detection
        if errors.is_empty() {
            let dob_val = dob_idx
                .and_then(|idx| row.get(idx))
                .map(|v| v.trim().to_string())
                .unwrap_or_default();
            let key = (
                first.to_lowercase(),
                last.to_lowercase(),
                mbi_val.to_lowercase(),
                dob_val.to_lowercase(),
            );
            if let Some(&prev_row) = seen.get(&key) {
                errors.push(format!("Duplicate of row {}", prev_row));
            } else {
                seen.insert(key, i + 1);
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

// ── Preview types ──────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ImportPreview {
    pub inserts: Vec<PreviewInsert>,
    pub updates: Vec<PreviewUpdate>,
    pub skipped: Vec<PreviewSkipped>,
    pub errors: Vec<ErrorRow>,
}

#[derive(serde::Serialize)]
pub struct PreviewInsert {
    pub row_index: usize,
    pub name: String,
}

#[derive(serde::Serialize)]
pub struct PreviewUpdate {
    pub row_index: usize,
    pub client_id: String,
    pub name: String,
    pub diffs: Vec<FieldDiff>,
}

#[derive(serde::Serialize)]
pub struct FieldDiff {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

#[derive(serde::Serialize)]
pub struct PreviewSkipped {
    pub row_index: usize,
    pub name: String,
    pub reason: String,
}

const UPDATABLE_FIELDS: &[&str] = &[
    "phone", "email", "address_line1", "address_line2",
    "city", "state", "zip", "county",
    "dual_status_code", "lis_level", "medicaid_id", "notes",
];

/// Build a preview of what the import will do (dry-run with DB lookup)
pub fn preview_import(
    conn: &Connection,
    rows: &[Vec<String>],
    headers: &[String],
    mapping: &HashMap<String, String>,
    constant_values: &HashMap<String, String>,
) -> Result<ImportPreview, AppError> {
    let mut inserts = Vec::new();
    let mut updates = Vec::new();
    let mut skipped = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let get_val = |target: &str| -> Option<String> {
            if let Some(idx) = find_mapped_index(headers, mapping, target) {
                let val = row.get(idx).map(|v| v.trim().to_string()).unwrap_or_default();
                if !val.is_empty() {
                    return Some(val);
                }
            }
            constant_values.get(target).filter(|v| !v.is_empty()).cloned()
        };

        let first_name = match get_val("first_name") {
            Some(v) => v,
            None => continue,
        };
        let last_name = match get_val("last_name") {
            Some(v) => v,
            None => continue,
        };
        let mbi = get_val("mbi");
        let client_name = format!("{} {}", first_name, last_name);

        let existing_id = find_existing_client(conn, &first_name, &last_name, &mbi, &get_val);

        if let Some(client_id) = existing_id {
            // Compare each updatable field
            let mut diffs = Vec::new();
            for &field in UPDATABLE_FIELDS {
                let import_val = match get_val(field) {
                    Some(v) => v,
                    None => continue, // no value in import → skip (don't overwrite with blank)
                };
                let current_val: String = conn
                    .query_row(
                        &format!("SELECT COALESCE({}, '') FROM clients WHERE id = ?1", field),
                        rusqlite::params![client_id],
                        |row| row.get(0),
                    )
                    .unwrap_or_default();

                if import_val.trim() != current_val.trim() {
                    diffs.push(FieldDiff {
                        field: field.to_string(),
                        old_value: current_val,
                        new_value: import_val,
                    });
                }
            }

            if diffs.is_empty() {
                skipped.push(PreviewSkipped {
                    row_index: i,
                    name: client_name,
                    reason: "No changes".to_string(),
                });
            } else {
                updates.push(PreviewUpdate {
                    row_index: i,
                    client_id,
                    name: client_name,
                    diffs,
                });
            }
        } else {
            inserts.push(PreviewInsert {
                row_index: i,
                name: client_name,
            });
        }
    }

    Ok(ImportPreview {
        inserts,
        updates,
        skipped,
        errors: Vec::new(), // errors come from validation, merged by the command layer
    })
}

/// Execute the actual import - insert/update clients
pub fn execute_import(
    conn: &Connection,
    rows: &[Vec<String>],
    headers: &[String],
    mapping: &HashMap<String, String>,
    constant_values: &HashMap<String, String>,
    approved_updates: Option<&HashMap<String, Vec<String>>>,
    approved_inserts: Option<&Vec<usize>>,
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
        match import_single_row(conn, row, i, headers, mapping, constant_values, approved_updates, approved_inserts) {
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
    row_index: usize,
    headers: &[String],
    mapping: &HashMap<String, String>,
    constant_values: &HashMap<String, String>,
    approved_updates: Option<&HashMap<String, Vec<String>>>,
    approved_inserts: Option<&Vec<usize>>,
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

    let existing_id = find_existing_client(conn, &first_name, &last_name, &mbi, &get_val);

    let client_name = format!("{} {}", first_name, last_name);

    if let Some(client_id) = existing_id {
        // If approved_updates is provided, check if this client was approved
        if let Some(approved) = approved_updates {
            match approved.get(&client_id) {
                None => return Ok(ImportAction::Skipped { name: client_name }),
                Some(approved_fields) if approved_fields.is_empty() => {
                    return Ok(ImportAction::Skipped { name: client_name });
                }
                _ => {}
            }
        }
        // Update existing client
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;
        let mut updated_fields = Vec::new();

        // Get the approved field list for this client (if approval filtering is active)
        let approved_field_list: Option<&Vec<String>> =
            approved_updates.and_then(|a| a.get(&client_id));

        macro_rules! set_if {
            ($field:expr, $col:expr) => {
                if let Some(ref list) = approved_field_list {
                    if !list.iter().any(|f| f == $col) {
                        // Field not approved — skip
                    } else if let Some(val) = get_val($field) {
                        sets.push(format!("{} = ?{}", $col, idx));
                        params.push(Box::new(val));
                        idx += 1;
                        updated_fields.push($col.to_string());
                    }
                } else if let Some(val) = get_val($field) {
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
        params.push(Box::new(client_id.clone()));
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

        let event_data = serde_json::json!({
            "source": "file_import",
            "fields": updated_fields,
        })
        .to_string();
        let _ = conversation_service::create_system_event(
            conn,
            &client_id,
            "CLIENT_UPDATED",
            Some(&event_data),
        );

        Ok(ImportAction::Updated { name: client_name, fields: updated_fields })
    } else {
        // If approved_inserts is provided, check if this row was approved
        if let Some(approved) = approved_inserts {
            if !approved.contains(&row_index) {
                return Ok(ImportAction::Skipped { name: client_name });
            }
        }
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

        let event_data = serde_json::json!({
            "source": "file_import",
        })
        .to_string();
        let _ = conversation_service::create_system_event(
            conn,
            &id,
            "CLIENT_IMPORTED",
            Some(&event_data),
        );

        Ok(ImportAction::Inserted { name: client_name })
    }
}

/// Find an existing client by MBI → name+DOB → name-only cascade
fn find_existing_client(
    conn: &Connection,
    first_name: &str,
    last_name: &str,
    mbi: &Option<String>,
    get_val: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    // 1. Try MBI
    if let Some(ref mbi_val) = mbi {
        if let Ok(id) = conn.query_row(
            "SELECT id FROM clients WHERE mbi = ?1 AND is_active = 1",
            rusqlite::params![mbi_val],
            |row| row.get::<_, String>(0),
        ) {
            return Some(id);
        }
    }

    // 2. Try name + DOB
    if let Some(dob_val) = get_val("dob") {
        if let Ok(id) = conn.query_row(
            "SELECT id FROM clients WHERE first_name = ?1 AND last_name = ?2 AND dob = ?3 AND is_active = 1",
            rusqlite::params![first_name, last_name, dob_val],
            |row| row.get::<_, String>(0),
        ) {
            return Some(id);
        }
    }

    // 3. Name-only fallback: match if exactly one active client
    let mut stmt = conn.prepare(
        "SELECT id FROM clients WHERE LOWER(first_name) = LOWER(?1) AND LOWER(last_name) = LOWER(?2) AND is_active = 1"
    ).ok()?;
    let ids: Vec<String> = stmt
        .query_map(rusqlite::params![first_name, last_name], |row| row.get(0))
        .ok()?
        .filter_map(|r| r.ok())
        .collect();
    if ids.len() == 1 {
        Some(ids.into_iter().next().unwrap())
    } else {
        None
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
