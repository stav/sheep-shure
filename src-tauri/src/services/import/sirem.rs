use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use super::file_import::ImportRowDetail;
use super::call_log::ActivityImportResult;
use super::shared::{normalize_date, normalize_mbi, normalize_phone, upsert_client, ImportClientData, UpsertAction};

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
            .map(|v| if v == "t" { Some(true) } else { None })
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

        match upsert_client(conn, &client_data, Some("sirem")) {
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
