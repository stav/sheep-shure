use std::collections::HashMap;

use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    CarrierMonthSummary, CommissionDeposit, CommissionDepositListItem, CommissionEntry,
    CommissionEntryListItem, CommissionFilters, CommissionRateListItem,
    CreateCommissionDepositInput, CreateCommissionRateInput, ImportLogEntry, ReconciliationRow,
    StatementImportResult, UpdateCommissionDepositInput, UpdateCommissionEntryInput,
    UpdateCommissionRateInput,
};
use crate::repositories::commission_repo;
use crate::services::commission_importers;

fn make_log(level: &str, phase: &str, message: &str, detail: Option<&str>) -> ImportLogEntry {
    ImportLogEntry {
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        level: level.to_string(),
        phase: phase.to_string(),
        message: message.to_string(),
        detail: detail.map(String::from),
    }
}

// ============================================================================
// Commission Rates
// ============================================================================

pub fn get_commission_rates(
    conn: &Connection,
    carrier_id: Option<&str>,
    plan_year: Option<i32>,
) -> Result<Vec<CommissionRateListItem>, AppError> {
    commission_repo::get_commission_rates(conn, carrier_id, plan_year)
}

pub fn create_commission_rate(
    conn: &Connection,
    input: &CreateCommissionRateInput,
) -> Result<CommissionRateListItem, AppError> {
    let id = Uuid::new_v4().to_string();
    commission_repo::create_commission_rate(conn, &id, input)?;

    // Return the created rate with carrier name
    let rates = commission_repo::get_commission_rates(conn, Some(&input.carrier_id), Some(input.plan_year))?;
    rates
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::NotFound("Created rate not found".to_string()))
}

pub fn update_commission_rate(
    conn: &Connection,
    id: &str,
    input: &UpdateCommissionRateInput,
) -> Result<(), AppError> {
    commission_repo::update_commission_rate(conn, id, input)
}

pub fn delete_commission_rate(conn: &Connection, id: &str) -> Result<(), AppError> {
    commission_repo::delete_commission_rate(conn, id)
}

// ============================================================================
// Commission Entries
// ============================================================================

pub fn get_commission_entries(
    conn: &Connection,
    filters: &CommissionFilters,
) -> Result<Vec<CommissionEntryListItem>, AppError> {
    commission_repo::get_commission_entries(conn, filters)
}

pub fn delete_commission_batch(conn: &Connection, batch_id: &str) -> Result<usize, AppError> {
    commission_repo::delete_entries_by_batch(conn, batch_id)
}

pub fn update_commission_entry(
    conn: &Connection,
    id: &str,
    input: &UpdateCommissionEntryInput,
) -> Result<(), AppError> {
    commission_repo::update_commission_entry(conn, id, input)
}

pub fn delete_commission_entry(conn: &Connection, id: &str) -> Result<(), AppError> {
    commission_repo::delete_commission_entry(conn, id)
}

// ============================================================================
// Reconciliation
// ============================================================================

/// Determine if a client's enrollment is initial (year 1) or renewal for a given month
pub fn determine_initial_or_renewal(
    conn: &Connection,
    client_id: &str,
    carrier_id: &str,
    commission_month: &str,
) -> Result<bool, AppError> {
    // Parse commission_month "YYYY-MM" to get the year
    let commission_year: i32 = commission_month
        .split('-')
        .next()
        .and_then(|y| y.parse().ok())
        .unwrap_or(0);

    // Look up the enrollment effective_date for this client+carrier
    let sql = "SELECT effective_date FROM enrollments
               WHERE client_id = ?1 AND carrier_id = ?2 AND is_active = 1
               ORDER BY effective_date DESC LIMIT 1";

    let effective_date: Option<String> = conn
        .query_row(sql, rusqlite::params![client_id, carrier_id], |row| {
            row.get(0)
        })
        .ok();

    if let Some(ref eff) = effective_date {
        let eff_year: i32 = eff
            .split('-')
            .next()
            .and_then(|y| y.parse().ok())
            .unwrap_or(0);

        // Initial if effective year == commission year
        Ok(eff_year == commission_year)
    } else {
        // No enrollment found - can't determine, default to renewal
        Ok(false)
    }
}

/// Reconcile commission entries: look up rates, determine initial/renewal, compute differences
pub fn reconcile_entries(
    conn: &Connection,
    carrier_id: Option<&str>,
    month: Option<&str>,
) -> Result<usize, AppError> {
    // Get entries to reconcile
    let filters = CommissionFilters {
        carrier_id: carrier_id.map(String::from),
        commission_month: month.map(String::from),
        status: None,
        client_id: None,
        import_batch_id: None,
    };
    let entries = commission_repo::get_commission_entries(conn, &filters)?;
    let mut updated = 0;

    // Also get full entry data for each
    for entry in &entries {
        let full_entry_sql = "SELECT id, client_id, enrollment_id, carrier_id, plan_type_code, commission_month, statement_amount
                              FROM commission_entries WHERE id = ?1";
        let row: Result<(String, Option<String>, Option<String>, String, Option<String>, String, Option<f64>), _> =
            conn.query_row(full_entry_sql, rusqlite::params![entry.id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            });

        if let Ok((id, client_id, _enrollment_id, entry_carrier_id, plan_type_code, commission_month, statement_amount)) = row {
            // Determine status
            let (status, expected_rate, rate_diff, is_initial) = if client_id.is_none() {
                ("UNMATCHED".to_string(), None, None, None)
            } else {
                let client_id = client_id.unwrap();
                let is_initial_val = determine_initial_or_renewal(conn, &client_id, &entry_carrier_id, &commission_month)?;

                // Parse year from commission_month
                let year: i32 = commission_month
                    .split('-')
                    .next()
                    .and_then(|y| y.parse().ok())
                    .unwrap_or(0);

                let rate = if let Some(ref ptc) = plan_type_code {
                    commission_repo::lookup_rate(conn, &entry_carrier_id, ptc, year)?
                } else {
                    None
                };

                match rate {
                    Some(r) => {
                        let expected = if is_initial_val { r.initial_rate } else { r.renewal_rate };
                        let stmt_amt = statement_amount.unwrap_or(0.0);
                        let diff = stmt_amt - expected;
                        let status = if expected == 0.0 {
                            "ZERO_RATE"
                        } else if diff.abs() < 0.01 {
                            "OK"
                        } else if diff < 0.0 {
                            "UNDERPAID"
                        } else {
                            "OVERPAID"
                        };
                        (status.to_string(), Some(expected), Some(diff), Some(if is_initial_val { 1 } else { 0 }))
                    }
                    None => {
                        ("ZERO_RATE".to_string(), None, None, Some(if is_initial_val { 1 } else { 0 }))
                    }
                }
            };

            commission_repo::update_entry_status(conn, &id, &status, expected_rate, rate_diff, is_initial)?;
            updated += 1;
        }
    }

    Ok(updated)
}

/// Find active enrollments with no commission entry for the given carrier/month and insert MISSING entries
pub fn find_missing_clients(
    conn: &Connection,
    carrier_id: &str,
    month: &str,
) -> Result<usize, AppError> {
    let sql = "SELECT e.client_id, e.id, e.plan_type_code, c.first_name || ' ' || c.last_name
               FROM enrollments e
               JOIN clients c ON e.client_id = c.id
               WHERE e.carrier_id = ?1 AND e.is_active = 1 AND e.status_code = 'ACTIVE'
               AND NOT EXISTS (
                   SELECT 1 FROM commission_entries ce
                   WHERE ce.client_id = e.client_id AND ce.carrier_id = ?1 AND ce.commission_month = ?2
               )";

    let mut stmt = conn.prepare(sql)?;
    let missing: Vec<(String, String, Option<String>, String)> = stmt
        .query_map(rusqlite::params![carrier_id, month], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut count = 0;
    for (client_id, enrollment_id, plan_type_code, _client_name) in &missing {
        let entry = CommissionEntry {
            id: Uuid::new_v4().to_string(),
            client_id: Some(client_id.clone()),
            enrollment_id: Some(enrollment_id.clone()),
            carrier_id: carrier_id.to_string(),
            plan_type_code: plan_type_code.clone(),
            commission_month: month.to_string(),
            statement_amount: None,
            paid_amount: None,
            member_name: None,
            member_id: None,
            is_initial: None,
            expected_rate: None,
            rate_difference: None,
            status: Some("MISSING".to_string()),
            import_batch_id: None,
            notes: None,
            created_at: None,
            updated_at: None,
        };
        commission_repo::upsert_commission_entry(conn, &entry)?;
        count += 1;
    }

    Ok(count)
}

pub fn get_reconciliation_entries(
    conn: &Connection,
    filters: &CommissionFilters,
) -> Result<Vec<ReconciliationRow>, AppError> {
    commission_repo::get_reconciliation_entries(conn, filters)
}

pub fn get_carrier_month_summaries(
    conn: &Connection,
    month: Option<&str>,
) -> Result<Vec<CarrierMonthSummary>, AppError> {
    commission_repo::get_carrier_month_summaries(conn, month)
}

// ============================================================================
// Commission Deposits
// ============================================================================

pub fn get_commission_deposits(
    conn: &Connection,
    carrier_id: Option<&str>,
    month: Option<&str>,
) -> Result<Vec<CommissionDepositListItem>, AppError> {
    commission_repo::get_commission_deposits(conn, carrier_id, month)
}

pub fn create_commission_deposit(
    conn: &Connection,
    input: &CreateCommissionDepositInput,
) -> Result<CommissionDeposit, AppError> {
    let id = Uuid::new_v4().to_string();
    commission_repo::create_commission_deposit(conn, &id, input)?;

    // Return the created deposit
    let sql = "SELECT id, carrier_id, deposit_month, deposit_amount, deposit_date, reference, notes, created_at, updated_at
               FROM commission_deposits WHERE id = ?1";
    conn.query_row(sql, rusqlite::params![id], |row| {
        Ok(CommissionDeposit {
            id: row.get(0)?,
            carrier_id: row.get(1)?,
            deposit_month: row.get(2)?,
            deposit_amount: row.get(3)?,
            deposit_date: row.get(4)?,
            reference: row.get(5)?,
            notes: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })
    .map_err(|e| AppError::Database(e.to_string()))
}

pub fn update_commission_deposit(
    conn: &Connection,
    id: &str,
    input: &UpdateCommissionDepositInput,
) -> Result<(), AppError> {
    commission_repo::update_commission_deposit(conn, id, input)
}

pub fn delete_commission_deposit(conn: &Connection, id: &str) -> Result<(), AppError> {
    commission_repo::delete_commission_deposit(conn, id)
}

// ============================================================================
// Statement Import
// ============================================================================

/// Parse a commission statement file and return headers + sample rows (for preview)
pub fn parse_commission_statement(file_path: &str) -> Result<crate::services::import_service::ParsedFile, AppError> {
    crate::services::import_service::parse_file(file_path)
}

/// Import a commission statement file.
/// Dispatches to the correct carrier-specific parser based on `carrier.short_name`.
/// When `log` is provided, structured log entries are emitted for live activity display.
pub fn import_commission_statement(
    conn: &Connection,
    file_path: &str,
    carrier_id: &str,
    commission_month: &str,
    _column_mapping: &HashMap<String, String>,
    log: Option<&dyn Fn(ImportLogEntry)>,
) -> Result<StatementImportResult, AppError> {
    let emit = |level: &str, phase: &str, msg: &str, detail: Option<&str>| {
        if let Some(log_fn) = log {
            log_fn(make_log(level, phase, msg, detail));
        }
    };

    // Look up carrier short_name for dispatch
    let short_name: String = conn
        .query_row(
            "SELECT COALESCE(short_name, name) FROM carriers WHERE id = ?1",
            rusqlite::params![carrier_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound(format!("Carrier not found: {}", carrier_id)))?;

    emit("info", "parse", &format!("Carrier: {}, using {} importer", short_name, short_name.to_lowercase()), None);

    // Log the first line of the file for diagnostic purposes
    if let Ok(content) = std::fs::read_to_string(file_path) {
        let preview: String = content.lines().take(2).collect::<Vec<_>>().join(" | ");
        tracing::info!(
            "Importing commission statement for carrier '{}' (short_name: '{}'): {}",
            carrier_id, short_name, preview
        );
    }

    let parsed_rows = commission_importers::parse_statement_rows(file_path, &short_name)?;

    // Log parse results with header info
    if let Ok(content) = std::fs::read_to_string(file_path) {
        if let Some(header_line) = content.lines().next() {
            let headers = if header_line.contains('|') {
                header_line.split('|').take(8).collect::<Vec<_>>().join("|")
            } else {
                header_line.chars().take(120).collect::<String>()
            };
            emit("info", "parse",
                &format!("Parsed {} rows", parsed_rows.len()),
                Some(&format!("Headers: {}", headers)));
        }
    }

    // Log sample rows (first 5)
    for (i, row) in parsed_rows.iter().take(5).enumerate() {
        let name = row.member_name.as_deref().unwrap_or("?");
        let amt = row.statement_amount.map(|a| format!("${:.2}", a)).unwrap_or_else(|| "?".to_string());
        let plan = row.plan_type_code.as_deref().unwrap_or("?");
        let ir = match row.is_initial {
            Some(true) => "Initial",
            Some(false) => "Renewal",
            None => "?",
        };
        emit("info", "parse", &format!("Sample {}: {} → {} {} {}", i + 1, name, amt, plan, ir), None);
    }

    let batch_id = Uuid::new_v4().to_string();

    let mut matched = 0;
    let mut unmatched = 0;
    let mut skipped = 0;
    let mut errors = 0;
    let mut unmatched_names = Vec::new();
    let mut error_messages = Vec::new();

    for (row_idx, row) in parsed_rows.iter().enumerate() {
        if row.member_name.is_none() && row.member_id.is_none() {
            if row.statement_amount.is_some() {
                errors += 1;
                let msg = format!("Row {}: No member name or ID", row_idx + 2);
                emit("error", "match", &msg, None);
                error_messages.push(msg);
            } else {
                skipped += 1;
            }
            continue;
        }

        // Try to match to a client
        let (client_id, enrollment_id, enrollment_plan_type) = match_statement_member(
            conn,
            row.member_name.as_deref(),
            row.member_id.as_deref(),
            carrier_id,
            row.member_id_is_mbi,
        );

        // Prefer importer-provided plan_type, fall back to enrollment plan_type
        let plan_type_code = row.plan_type_code.clone().or(enrollment_plan_type);

        let name = row.member_name.as_deref().unwrap_or("?");
        let amt = row.statement_amount.map(|a| format!("${:.2}", a)).unwrap_or_else(|| "?".to_string());
        let plan = plan_type_code.as_deref().unwrap_or("?");
        let ir = match row.is_initial {
            Some(true) => "Initial",
            Some(false) => "Renewal",
            None => "",
        };

        let status = if client_id.is_some() {
            matched += 1;
            emit("success", "match",
                &format!("Row {}: {} → {} {} {} → matched", row_idx + 1, name, amt, plan, ir), None);
            "PENDING"
        } else {
            unmatched += 1;
            if let Some(ref name) = row.member_name {
                if !unmatched_names.contains(name) {
                    unmatched_names.push(name.clone());
                }
            }
            emit("warn", "match",
                &format!("Row {}: {} → {} {} {} → UNMATCHED", row_idx + 1, name, amt, plan, ir), None);
            "UNMATCHED"
        };

        // Use importer-provided is_initial when available
        let is_initial = row.is_initial.map(|b| if b { 1 } else { 0 });

        let entry = CommissionEntry {
            id: Uuid::new_v4().to_string(),
            client_id,
            enrollment_id,
            carrier_id: carrier_id.to_string(),
            plan_type_code,
            commission_month: commission_month.to_string(),
            statement_amount: row.statement_amount,
            paid_amount: row.paid_amount,
            member_name: row.member_name.clone(),
            member_id: row.member_id.clone(),
            is_initial,
            expected_rate: None,
            rate_difference: None,
            status: Some(status.to_string()),
            import_batch_id: Some(batch_id.clone()),
            notes: row.notes.clone(),
            created_at: None,
            updated_at: None,
        };

        if let Err(e) = commission_repo::upsert_commission_entry(conn, &entry) {
            errors += 1;
            let msg = format!("Row {}: {}", row_idx + 2, e);
            emit("error", "import", &msg, None);
            error_messages.push(msg);
        }
    }

    let total = matched + unmatched + skipped + errors;

    emit("success", "import",
        &format!("Import complete: {} matched, {} unmatched, {} skipped, {} errors", matched, unmatched, skipped, errors),
        None);

    Ok(StatementImportResult {
        total,
        matched,
        unmatched,
        skipped,
        errors,
        batch_id,
        unmatched_names,
        error_messages,
    })
}

/// Match a statement member to a client in the database.
/// When `member_id_is_mbi` is true, the member_id is tried as an MBI lookup.
/// When false (e.g. Humana's GrpNbr), the MBI lookup is skipped.
fn match_statement_member(
    conn: &Connection,
    member_name: Option<&str>,
    member_id: Option<&str>,
    carrier_id: &str,
    member_id_is_mbi: bool,
) -> (Option<String>, Option<String>, Option<String>) {
    // Try member_id (MBI) match first — only if the ID is actually an MBI
    if member_id_is_mbi {
        if let Some(mid) = member_id {
            if !mid.is_empty() {
                if let Ok((cid, eid, ptc)) = conn.query_row(
                    "SELECT c.id, e.id, e.plan_type_code
                     FROM clients c
                     LEFT JOIN enrollments e ON e.client_id = c.id AND e.carrier_id = ?2 AND e.is_active = 1
                     WHERE c.mbi = ?1 AND c.is_active = 1
                     LIMIT 1",
                    rusqlite::params![mid, carrier_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?)),
                ) {
                    return (Some(cid), eid, ptc);
                }
            }
        }
    }

    // Parse member name into first/last
    if let Some(name) = member_name {
        let (first, last) = parse_member_name(name);
        if !last.is_empty() {
            // Try exact name match on enrollments for this carrier
            if let Ok((cid, eid, ptc)) = conn.query_row(
                "SELECT c.id, e.id, e.plan_type_code
                 FROM clients c
                 JOIN enrollments e ON e.client_id = c.id AND e.carrier_id = ?3 AND e.is_active = 1
                 WHERE LOWER(c.last_name) = LOWER(?2) AND LOWER(c.first_name) = LOWER(?1) AND c.is_active = 1
                 LIMIT 1",
                rusqlite::params![first, last, carrier_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?)),
            ) {
                return (Some(cid), eid, ptc);
            }

            // Try with first name as prefix (strip middle initial: "Kenneth E" -> "Kenneth")
            let first_base = first.split_whitespace().next().unwrap_or(&first);
            if first_base != first {
                if let Ok((cid, eid, ptc)) = conn.query_row(
                    "SELECT c.id, e.id, e.plan_type_code
                     FROM clients c
                     JOIN enrollments e ON e.client_id = c.id AND e.carrier_id = ?3 AND e.is_active = 1
                     WHERE LOWER(c.last_name) = LOWER(?2) AND LOWER(c.first_name) = LOWER(?1) AND c.is_active = 1
                     LIMIT 1",
                    rusqlite::params![first_base, last, carrier_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?)),
                ) {
                    return (Some(cid), eid, ptc);
                }
            }

            // Try name match without enrollment (broader)
            if let Ok(cid) = conn.query_row(
                "SELECT id FROM clients
                 WHERE LOWER(last_name) = LOWER(?2) AND LOWER(first_name) = LOWER(?1) AND is_active = 1
                 LIMIT 1",
                rusqlite::params![first, last],
                |row| row.get::<_, String>(0),
            ) {
                // Look up enrollment separately
                let enrollment: Option<(String, Option<String>)> = conn
                    .query_row(
                        "SELECT id, plan_type_code FROM enrollments
                         WHERE client_id = ?1 AND carrier_id = ?2 AND is_active = 1
                         LIMIT 1",
                        rusqlite::params![cid, carrier_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .ok();

                return (
                    Some(cid),
                    enrollment.as_ref().map(|(id, _)| id.clone()),
                    enrollment.and_then(|(_, ptc)| ptc),
                );
            }
        }
    }

    (None, None, None)
}

/// Parse a member name into (first, last)
/// Handles: "Last, First", "First Last", "Last, First M", "First M Last"
fn parse_member_name(name: &str) -> (String, String) {
    let trimmed = name.trim();
    if trimmed.contains(',') {
        // "Last, First" or "Last, First M"
        let parts: Vec<&str> = trimmed.splitn(2, ',').collect();
        let last = parts[0].trim().to_string();
        let first = parts.get(1).map(|s| s.trim()).unwrap_or("").to_string();
        (first, last)
    } else {
        // "First Last" or "First M Last"
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        match parts.len() {
            0 => (String::new(), String::new()),
            1 => (String::new(), parts[0].to_string()),
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                // Last word is last name, everything else is first
                let last = parts.last().unwrap().to_string();
                let first = parts[..parts.len() - 1].join(" ");
                (first, last)
            }
        }
    }
}
