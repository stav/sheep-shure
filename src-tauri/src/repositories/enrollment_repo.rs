use rusqlite::{params, Connection};
use crate::error::AppError;
use crate::models::{Enrollment, EnrollmentListItem, CreateEnrollmentInput, UpdateEnrollmentInput};

/// Get enrollments, optionally filtered by client_id
pub fn get_enrollments(conn: &Connection, client_id: Option<&str>) -> Result<Vec<EnrollmentListItem>, AppError> {
    let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(cid) = client_id {
        (
            "SELECT e.id, c.first_name || ' ' || c.last_name, e.plan_name, cr.name, e.plan_type_code, es.name, e.effective_date, e.termination_date
             FROM enrollments e
             LEFT JOIN clients c ON e.client_id = c.id
             LEFT JOIN carriers cr ON e.carrier_id = cr.id
             LEFT JOIN enrollment_statuses es ON e.status_code = es.code
             WHERE e.client_id = ?1 AND e.is_active = 1
             ORDER BY e.effective_date DESC".to_string(),
            vec![Box::new(cid.to_string()) as Box<dyn rusqlite::types::ToSql>],
        )
    } else {
        (
            "SELECT e.id, c.first_name || ' ' || c.last_name, e.plan_name, cr.name, e.plan_type_code, es.name, e.effective_date, e.termination_date
             FROM enrollments e
             LEFT JOIN clients c ON e.client_id = c.id
             LEFT JOIN carriers cr ON e.carrier_id = cr.id
             LEFT JOIN enrollment_statuses es ON e.status_code = es.code
             WHERE e.is_active = 1
             ORDER BY e.effective_date DESC".to_string(),
            vec![],
        )
    };

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(EnrollmentListItem {
            id: row.get(0)?,
            client_name: row.get(1)?,
            plan_name: row.get(2)?,
            carrier_name: row.get(3)?,
            plan_type: row.get(4)?,
            status: row.get(5)?,
            effective_date: row.get(6)?,
            termination_date: row.get(7)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// Get a single enrollment by ID
pub fn get_enrollment(conn: &Connection, id: &str) -> Result<Enrollment, AppError> {
    let sql = "SELECT id, client_id, plan_id, carrier_id, plan_type_code, plan_name, contract_number,
               pbp_number, effective_date, termination_date, application_date, status_code,
               enrollment_period, disenrollment_reason, premium, confirmation_number, enrollment_source,
               is_active, created_at, updated_at
               FROM enrollments WHERE id = ?1";

    conn.query_row(sql, params![id], |row| {
        Ok(Enrollment {
            id: row.get(0)?,
            client_id: row.get(1)?,
            plan_id: row.get(2)?,
            carrier_id: row.get(3)?,
            plan_type_code: row.get(4)?,
            plan_name: row.get(5)?,
            contract_number: row.get(6)?,
            pbp_number: row.get(7)?,
            effective_date: row.get(8)?,
            termination_date: row.get(9)?,
            application_date: row.get(10)?,
            status_code: row.get(11)?,
            enrollment_period: row.get(12)?,
            disenrollment_reason: row.get(13)?,
            premium: row.get(14)?,
            confirmation_number: row.get(15)?,
            enrollment_source: row.get(16)?,
            is_active: row.get(17)?,
            created_at: row.get(18)?,
            updated_at: row.get(19)?,
        })
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Enrollment {} not found", id)),
        _ => AppError::Database(e.to_string()),
    })
}

/// Check if client already has an active/pending enrollment in the same plan category
pub fn has_active_enrollment_in_category(conn: &Connection, client_id: &str, plan_type_code: &str, exclude_id: Option<&str>) -> Result<bool, AppError> {
    // Determine the category from plan_type_code
    let category = get_plan_category(plan_type_code);

    // Get all plan_type_codes in the same category
    let category_codes = get_codes_for_category(&category);

    if category_codes.is_empty() {
        return Ok(false);
    }

    let placeholders: Vec<String> = category_codes.iter().enumerate().map(|(i, _)| format!("?{}", i + 3)).collect();

    let mut sql = format!(
        "SELECT COUNT(*) FROM enrollments WHERE client_id = ?1 AND status_code IN ('ACTIVE', 'PENDING') AND plan_type_code IN ({}) AND is_active = 1",
        placeholders.join(", ")
    );

    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    param_values.push(Box::new(client_id.to_string()));
    // ?2 reserved for exclude_id
    if let Some(eid) = exclude_id {
        sql.push_str(" AND id != ?2");
        param_values.push(Box::new(eid.to_string()));
    } else {
        param_values.push(Box::new(rusqlite::types::Null));
    }

    for code in &category_codes {
        param_values.push(Box::new(code.to_string()));
    }

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let count: i64 = conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))?;

    Ok(count > 0)
}

fn get_plan_category(plan_type_code: &str) -> String {
    match plan_type_code {
        "MA" | "MAPD" | "DSNP" | "CSNP" | "ISNP" | "MMP" | "PACE" | "MSA" | "PFFS" | "COST" => "ADVANTAGE".to_string(),
        "PDP" => "PRESCRIPTION".to_string(),
        c if c.starts_with("MedSup") => "SUPPLEMENT".to_string(),
        _ => "OTHER".to_string(),
    }
}

fn get_codes_for_category(category: &str) -> Vec<&'static str> {
    match category {
        "ADVANTAGE" => vec!["MA", "MAPD", "DSNP", "CSNP", "ISNP", "MMP", "PACE", "MSA", "PFFS", "COST"],
        "PRESCRIPTION" => vec!["PDP"],
        "SUPPLEMENT" => vec!["MedSupA", "MedSupB", "MedSupC", "MedSupD", "MedSupF", "MedSupG", "MedSupK", "MedSupL", "MedSupM", "MedSupN"],
        _ => vec![],
    }
}

/// Create a new enrollment
pub fn create_enrollment(conn: &Connection, id: &str, input: &CreateEnrollmentInput) -> Result<(), AppError> {
    let sql = "INSERT INTO enrollments (id, client_id, plan_id, carrier_id, plan_type_code, plan_name,
               contract_number, pbp_number, effective_date, termination_date, application_date,
               status_code, enrollment_period, disenrollment_reason, premium, confirmation_number, enrollment_source)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)";

    conn.execute(sql, params![
        id, input.client_id, input.plan_id, input.carrier_id, input.plan_type_code, input.plan_name,
        input.contract_number, input.pbp_number, input.effective_date, input.termination_date,
        input.application_date, input.status_code, input.enrollment_period, input.disenrollment_reason,
        input.premium, input.confirmation_number, input.enrollment_source
    ])?;

    Ok(())
}

/// Update an enrollment
pub fn update_enrollment(conn: &Connection, id: &str, input: &UpdateEnrollmentInput) -> Result<(), AppError> {
    let sql = "UPDATE enrollments SET plan_id = COALESCE(?2, plan_id), carrier_id = COALESCE(?3, carrier_id),
               plan_type_code = COALESCE(?4, plan_type_code), plan_name = COALESCE(?5, plan_name),
               contract_number = COALESCE(?6, contract_number), pbp_number = COALESCE(?7, pbp_number),
               effective_date = COALESCE(?8, effective_date), termination_date = COALESCE(?9, termination_date),
               application_date = COALESCE(?10, application_date), status_code = COALESCE(?11, status_code),
               enrollment_period = COALESCE(?12, enrollment_period), disenrollment_reason = COALESCE(?13, disenrollment_reason),
               premium = COALESCE(?14, premium), confirmation_number = COALESCE(?15, confirmation_number),
               enrollment_source = COALESCE(?16, enrollment_source), is_active = COALESCE(?17, is_active)
               WHERE id = ?1";

    let rows = conn.execute(sql, params![
        id, input.plan_id, input.carrier_id, input.plan_type_code, input.plan_name,
        input.contract_number, input.pbp_number, input.effective_date, input.termination_date,
        input.application_date, input.status_code, input.enrollment_period, input.disenrollment_reason,
        input.premium, input.confirmation_number, input.enrollment_source, input.is_active
    ])?;

    if rows == 0 {
        return Err(AppError::NotFound(format!("Enrollment {} not found", id)));
    }

    Ok(())
}
