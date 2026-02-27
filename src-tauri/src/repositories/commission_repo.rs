use rusqlite::{params, Connection};

use crate::error::AppError;
use crate::models::{
    CarrierMonthSummary, CommissionDepositListItem, CommissionEntry,
    CommissionEntryListItem, CommissionFilters, CommissionRate, CommissionRateListItem,
    CreateCommissionDepositInput, CreateCommissionRateInput, ReconciliationRow,
    UpdateCommissionDepositInput, UpdateCommissionEntryInput, UpdateCommissionRateInput,
};

// ============================================================================
// Commission Rates
// ============================================================================

pub fn get_commission_rates(
    conn: &Connection,
    carrier_id: Option<&str>,
    plan_year: Option<i32>,
) -> Result<Vec<CommissionRateListItem>, AppError> {
    let mut sql = String::from(
        "SELECT cr.id, cr.carrier_id, c.name, cr.plan_type_code, cr.plan_year,
                cr.initial_rate, cr.renewal_rate, cr.notes
         FROM commission_rates cr
         JOIN carriers c ON cr.carrier_id = c.id
         WHERE 1=1",
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(cid) = carrier_id {
        sql.push_str(&format!(" AND cr.carrier_id = ?{}", idx));
        param_values.push(Box::new(cid.to_string()));
        idx += 1;
    }
    if let Some(year) = plan_year {
        sql.push_str(&format!(" AND cr.plan_year = ?{}", idx));
        param_values.push(Box::new(year));
    }

    sql.push_str(" ORDER BY c.name, cr.plan_type_code, cr.plan_year DESC");

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(CommissionRateListItem {
                id: row.get(0)?,
                carrier_id: row.get(1)?,
                carrier_name: row.get(2)?,
                plan_type_code: row.get(3)?,
                plan_year: row.get(4)?,
                initial_rate: row.get(5)?,
                renewal_rate: row.get(6)?,
                notes: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn create_commission_rate(
    conn: &Connection,
    id: &str,
    input: &CreateCommissionRateInput,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO commission_rates (id, carrier_id, plan_type_code, plan_year, initial_rate, renewal_rate, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            input.carrier_id,
            input.plan_type_code,
            input.plan_year,
            input.initial_rate,
            input.renewal_rate,
            input.notes
        ],
    )?;
    Ok(())
}

pub fn update_commission_rate(
    conn: &Connection,
    id: &str,
    input: &UpdateCommissionRateInput,
) -> Result<(), AppError> {
    let rows = conn.execute(
        "UPDATE commission_rates SET
            carrier_id = COALESCE(?2, carrier_id),
            plan_type_code = COALESCE(?3, plan_type_code),
            plan_year = COALESCE(?4, plan_year),
            initial_rate = COALESCE(?5, initial_rate),
            renewal_rate = COALESCE(?6, renewal_rate),
            notes = COALESCE(?7, notes)
         WHERE id = ?1",
        params![
            id,
            input.carrier_id,
            input.plan_type_code,
            input.plan_year,
            input.initial_rate,
            input.renewal_rate,
            input.notes
        ],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Commission rate {} not found",
            id
        )));
    }
    Ok(())
}

pub fn delete_commission_rate(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows = conn.execute("DELETE FROM commission_rates WHERE id = ?1", params![id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Commission rate {} not found",
            id
        )));
    }
    Ok(())
}

pub fn lookup_rate(
    conn: &Connection,
    carrier_id: &str,
    plan_type_code: &str,
    plan_year: i32,
) -> Result<Option<CommissionRate>, AppError> {
    let sql = "SELECT id, carrier_id, plan_type_code, plan_year, initial_rate, renewal_rate, notes, created_at, updated_at
               FROM commission_rates
               WHERE carrier_id = ?1 AND plan_type_code = ?2 AND plan_year = ?3";
    let result = conn.query_row(sql, params![carrier_id, plan_type_code, plan_year], |row| {
        Ok(CommissionRate {
            id: row.get(0)?,
            carrier_id: row.get(1)?,
            plan_type_code: row.get(2)?,
            plan_year: row.get(3)?,
            initial_rate: row.get(4)?,
            renewal_rate: row.get(5)?,
            notes: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    });

    match result {
        Ok(rate) => Ok(Some(rate)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}

// ============================================================================
// Commission Entries
// ============================================================================

pub fn get_commission_entries(
    conn: &Connection,
    filters: &CommissionFilters,
) -> Result<Vec<CommissionEntryListItem>, AppError> {
    let mut sql = String::from(
        "SELECT ce.id, ce.client_id,
                CASE WHEN c.id IS NOT NULL THEN c.first_name || ' ' || c.last_name ELSE ce.member_name END,
                cr.name, ce.plan_type_code, ce.commission_month,
                ce.statement_amount, ce.paid_amount, ce.member_name,
                ce.is_initial, ce.expected_rate, ce.rate_difference, ce.status,
                e.effective_date
         FROM commission_entries ce
         LEFT JOIN clients c ON ce.client_id = c.id
         LEFT JOIN carriers cr ON ce.carrier_id = cr.id
         LEFT JOIN enrollments e ON ce.enrollment_id = e.id
         WHERE 1=1",
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(ref cid) = filters.carrier_id {
        sql.push_str(&format!(" AND ce.carrier_id = ?{}", idx));
        param_values.push(Box::new(cid.clone()));
        idx += 1;
    }
    if let Some(ref month) = filters.commission_month {
        sql.push_str(&format!(" AND ce.commission_month = ?{}", idx));
        param_values.push(Box::new(month.clone()));
        idx += 1;
    }
    if let Some(ref status) = filters.status {
        sql.push_str(&format!(" AND ce.status = ?{}", idx));
        param_values.push(Box::new(status.clone()));
        idx += 1;
    }
    if let Some(ref client_id) = filters.client_id {
        sql.push_str(&format!(" AND ce.client_id = ?{}", idx));
        param_values.push(Box::new(client_id.clone()));
        idx += 1;
    }
    if let Some(ref batch_id) = filters.import_batch_id {
        sql.push_str(&format!(" AND ce.import_batch_id = ?{}", idx));
        param_values.push(Box::new(batch_id.clone()));
    }

    sql.push_str(" ORDER BY ce.commission_month DESC, cr.name, c.last_name, c.first_name");

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(CommissionEntryListItem {
                id: row.get(0)?,
                client_id: row.get(1)?,
                client_name: row.get(2)?,
                carrier_name: row.get(3)?,
                plan_type_code: row.get(4)?,
                commission_month: row.get(5)?,
                statement_amount: row.get(6)?,
                paid_amount: row.get(7)?,
                member_name: row.get(8)?,
                is_initial: row.get(9)?,
                expected_rate: row.get(10)?,
                rate_difference: row.get(11)?,
                status: row.get(12)?,
                effective_date: row.get(13)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn upsert_commission_entry(conn: &Connection, entry: &CommissionEntry) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO commission_entries (id, client_id, enrollment_id, carrier_id, plan_type_code,
            commission_month, statement_amount, paid_amount, member_name, member_id,
            is_initial, expected_rate, rate_difference, status, import_batch_id, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT(carrier_id, client_id, commission_month) WHERE client_id IS NOT NULL
         DO UPDATE SET
            enrollment_id = excluded.enrollment_id,
            plan_type_code = excluded.plan_type_code,
            statement_amount = excluded.statement_amount,
            paid_amount = excluded.paid_amount,
            member_name = excluded.member_name,
            member_id = excluded.member_id,
            is_initial = excluded.is_initial,
            expected_rate = excluded.expected_rate,
            rate_difference = excluded.rate_difference,
            status = excluded.status,
            import_batch_id = excluded.import_batch_id,
            notes = excluded.notes",
        params![
            entry.id,
            entry.client_id,
            entry.enrollment_id,
            entry.carrier_id,
            entry.plan_type_code,
            entry.commission_month,
            entry.statement_amount,
            entry.paid_amount,
            entry.member_name,
            entry.member_id,
            entry.is_initial,
            entry.expected_rate,
            entry.rate_difference,
            entry.status,
            entry.import_batch_id,
            entry.notes,
        ],
    )?;
    Ok(())
}

pub fn delete_entries_by_batch(conn: &Connection, batch_id: &str) -> Result<usize, AppError> {
    let rows = conn.execute(
        "DELETE FROM commission_entries WHERE import_batch_id = ?1",
        params![batch_id],
    )?;
    Ok(rows)
}

pub fn update_entry_status(
    conn: &Connection,
    id: &str,
    status: &str,
    expected_rate: Option<f64>,
    rate_difference: Option<f64>,
    is_initial: Option<i32>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE commission_entries SET status = ?2, expected_rate = ?3, rate_difference = ?4, is_initial = ?5
         WHERE id = ?1",
        params![id, status, expected_rate, rate_difference, is_initial],
    )?;
    Ok(())
}

pub fn update_commission_entry(
    conn: &Connection,
    id: &str,
    input: &UpdateCommissionEntryInput,
) -> Result<(), AppError> {
    let rows = conn.execute(
        "UPDATE commission_entries SET
            member_name = COALESCE(?2, member_name),
            plan_type_code = COALESCE(?3, plan_type_code),
            statement_amount = COALESCE(?4, statement_amount),
            paid_amount = COALESCE(?5, paid_amount),
            is_initial = COALESCE(?6, is_initial),
            status = COALESCE(?7, status),
            notes = COALESCE(?8, notes)
         WHERE id = ?1",
        params![
            id,
            input.member_name,
            input.plan_type_code,
            input.statement_amount,
            input.paid_amount,
            input.is_initial,
            input.status,
            input.notes
        ],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Commission entry {} not found",
            id
        )));
    }
    Ok(())
}

pub fn delete_commission_entry(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows = conn.execute(
        "DELETE FROM commission_entries WHERE id = ?1",
        params![id],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Commission entry {} not found",
            id
        )));
    }
    Ok(())
}

// ============================================================================
// Commission Deposits
// ============================================================================

pub fn get_commission_deposits(
    conn: &Connection,
    carrier_id: Option<&str>,
    month: Option<&str>,
) -> Result<Vec<CommissionDepositListItem>, AppError> {
    let mut sql = String::from(
        "SELECT cd.id, cd.carrier_id, c.name, cd.deposit_month, cd.deposit_amount,
                cd.deposit_date, cd.reference, cd.notes,
                COALESCE((SELECT SUM(ce.paid_amount) FROM commission_entries ce
                          WHERE ce.carrier_id = cd.carrier_id AND ce.commission_month = cd.deposit_month), 0) as statement_total,
                cd.deposit_amount - COALESCE((SELECT SUM(ce.paid_amount) FROM commission_entries ce
                          WHERE ce.carrier_id = cd.carrier_id AND ce.commission_month = cd.deposit_month), 0) as difference
         FROM commission_deposits cd
         JOIN carriers c ON cd.carrier_id = c.id
         WHERE 1=1",
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(cid) = carrier_id {
        sql.push_str(&format!(" AND cd.carrier_id = ?{}", idx));
        param_values.push(Box::new(cid.to_string()));
        idx += 1;
    }
    if let Some(m) = month {
        sql.push_str(&format!(" AND cd.deposit_month = ?{}", idx));
        param_values.push(Box::new(m.to_string()));
    }

    sql.push_str(" ORDER BY cd.deposit_month DESC, c.name");

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(CommissionDepositListItem {
                id: row.get(0)?,
                carrier_id: row.get(1)?,
                carrier_name: row.get(2)?,
                deposit_month: row.get(3)?,
                deposit_amount: row.get(4)?,
                deposit_date: row.get(5)?,
                reference: row.get(6)?,
                notes: row.get(7)?,
                statement_total: row.get(8)?,
                difference: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn create_commission_deposit(
    conn: &Connection,
    id: &str,
    input: &CreateCommissionDepositInput,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO commission_deposits (id, carrier_id, deposit_month, deposit_amount, deposit_date, reference, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            input.carrier_id,
            input.deposit_month,
            input.deposit_amount,
            input.deposit_date,
            input.reference,
            input.notes
        ],
    )?;
    Ok(())
}

pub fn update_commission_deposit(
    conn: &Connection,
    id: &str,
    input: &UpdateCommissionDepositInput,
) -> Result<(), AppError> {
    let rows = conn.execute(
        "UPDATE commission_deposits SET
            carrier_id = COALESCE(?2, carrier_id),
            deposit_month = COALESCE(?3, deposit_month),
            deposit_amount = COALESCE(?4, deposit_amount),
            deposit_date = COALESCE(?5, deposit_date),
            reference = COALESCE(?6, reference),
            notes = COALESCE(?7, notes)
         WHERE id = ?1",
        params![
            id,
            input.carrier_id,
            input.deposit_month,
            input.deposit_amount,
            input.deposit_date,
            input.reference,
            input.notes
        ],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Commission deposit {} not found",
            id
        )));
    }
    Ok(())
}

pub fn delete_commission_deposit(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows = conn.execute(
        "DELETE FROM commission_deposits WHERE id = ?1",
        params![id],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Commission deposit {} not found",
            id
        )));
    }
    Ok(())
}

// ============================================================================
// Reconciliation / Summary
// ============================================================================

pub fn get_reconciliation_entries(
    conn: &Connection,
    filters: &CommissionFilters,
) -> Result<Vec<ReconciliationRow>, AppError> {
    let mut sql = String::from(
        "SELECT ce.id, ce.client_id,
                CASE WHEN c.id IS NOT NULL THEN c.first_name || ' ' || c.last_name ELSE ce.member_name END,
                cr.name, ce.plan_type_code, ce.commission_month,
                e.effective_date, ce.is_initial, ce.expected_rate,
                ce.statement_amount, ce.paid_amount, ce.rate_difference, ce.status,
                ce.member_name
         FROM commission_entries ce
         LEFT JOIN clients c ON ce.client_id = c.id
         LEFT JOIN carriers cr ON ce.carrier_id = cr.id
         LEFT JOIN enrollments e ON ce.enrollment_id = e.id
         WHERE 1=1",
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(ref cid) = filters.carrier_id {
        sql.push_str(&format!(" AND ce.carrier_id = ?{}", idx));
        param_values.push(Box::new(cid.clone()));
        idx += 1;
    }
    if let Some(ref month) = filters.commission_month {
        sql.push_str(&format!(" AND ce.commission_month = ?{}", idx));
        param_values.push(Box::new(month.clone()));
        idx += 1;
    }
    if let Some(ref status) = filters.status {
        sql.push_str(&format!(" AND ce.status = ?{}", idx));
        param_values.push(Box::new(status.clone()));
    }

    sql.push_str(" ORDER BY ce.commission_month DESC, cr.name, c.last_name, c.first_name");

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(ReconciliationRow {
                id: row.get(0)?,
                client_id: row.get(1)?,
                client_name: row.get(2)?,
                carrier_name: row.get(3)?,
                plan_type_code: row.get(4)?,
                commission_month: row.get(5)?,
                effective_date: row.get(6)?,
                is_initial: row.get(7)?,
                expected_rate: row.get(8)?,
                statement_amount: row.get(9)?,
                paid_amount: row.get(10)?,
                rate_difference: row.get(11)?,
                status: row.get(12)?,
                member_name: row.get(13)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_carrier_month_summaries(
    conn: &Connection,
    month: Option<&str>,
) -> Result<Vec<CarrierMonthSummary>, AppError> {
    let mut sql = String::from(
        "SELECT ce.carrier_id, cr.name, ce.commission_month,
                COALESCE(SUM(ce.expected_rate), 0),
                COALESCE(SUM(ce.statement_amount), 0),
                COALESCE(SUM(ce.paid_amount), 0),
                cd.deposit_amount,
                CASE WHEN cd.deposit_amount IS NOT NULL
                     THEN cd.deposit_amount - COALESCE(SUM(ce.paid_amount), 0)
                     ELSE NULL END,
                COUNT(*),
                SUM(CASE WHEN ce.status = 'OK' THEN 1 ELSE 0 END),
                SUM(CASE WHEN ce.status IN ('UNDERPAID', 'OVERPAID', 'MISSING', 'ZERO_RATE', 'UNMATCHED') THEN 1 ELSE 0 END)
         FROM commission_entries ce
         JOIN carriers cr ON ce.carrier_id = cr.id
         LEFT JOIN commission_deposits cd ON cd.carrier_id = ce.carrier_id AND cd.deposit_month = ce.commission_month
         WHERE 1=1",
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(m) = month {
        sql.push_str(" AND ce.commission_month = ?1");
        param_values.push(Box::new(m.to_string()));
    }

    sql.push_str(" GROUP BY ce.carrier_id, ce.commission_month ORDER BY ce.commission_month DESC, cr.name");

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(CarrierMonthSummary {
                carrier_id: row.get(0)?,
                carrier_name: row.get(1)?,
                commission_month: row.get(2)?,
                total_expected: row.get(3)?,
                total_statement: row.get(4)?,
                total_paid: row.get(5)?,
                deposit_amount: row.get(6)?,
                deposit_vs_paid: row.get(7)?,
                entry_count: row.get(8)?,
                ok_count: row.get(9)?,
                issue_count: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}
