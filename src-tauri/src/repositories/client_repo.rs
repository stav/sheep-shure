use rusqlite::{params, Connection};
use crate::error::AppError;
use crate::models::{Client, ClientListItem, ClientFilters, CreateClientInput, UpdateClientInput, PaginatedResult};

/// Get paginated, filtered list of clients
pub fn get_clients(
    conn: &Connection,
    filters: &ClientFilters,
    page: i32,
    per_page: i32,
) -> Result<PaginatedResult<ClientListItem>, AppError> {
    let offset = (page - 1) * per_page;
    let mut conditions = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    // If search is provided, use FTS
    if let Some(ref search) = filters.search {
        if !search.is_empty() {
            // Get matching rowids from FTS, then join to clients
            conditions.push("c.rowid IN (SELECT rowid FROM clients_fts WHERE clients_fts MATCH ?1)".to_string());
            // Append * for prefix matching
            let search_term = format!("{}*", search.replace('"', ""));
            param_values.push(Box::new(search_term));
        }
    }

    if let Some(ref state) = filters.state {
        let idx = param_values.len() + 1;
        conditions.push(format!("c.state = ?{}", idx));
        param_values.push(Box::new(state.clone()));
    }

    if let Some(ref zip) = filters.zip {
        let idx = param_values.len() + 1;
        conditions.push(format!("c.zip = ?{}", idx));
        param_values.push(Box::new(zip.clone()));
    }

    if let Some(is_dual) = filters.is_dual_eligible {
        let idx = param_values.len() + 1;
        conditions.push(format!("c.is_dual_eligible = ?{}", idx));
        param_values.push(Box::new(is_dual));
    }

    if let Some(is_active) = filters.is_active {
        let idx = param_values.len() + 1;
        conditions.push(format!("c.is_active = ?{}", idx));
        param_values.push(Box::new(is_active));
    }
    // When is_active is None, no filter is applied — returns all clients

    // Carrier filter: join through enrollments
    if let Some(ref carrier_id) = filters.carrier_id {
        let idx = param_values.len() + 1;
        conditions.push(format!(
            "c.id IN (SELECT DISTINCT client_id FROM enrollments WHERE carrier_id = ?{})",
            idx
        ));
        param_values.push(Box::new(carrier_id.clone()));
    }

    // Plan type filter: join through enrollments
    if let Some(ref plan_type_code) = filters.plan_type_code {
        let idx = param_values.len() + 1;
        conditions.push(format!(
            "c.id IN (SELECT DISTINCT client_id FROM enrollments WHERE plan_type_code = ?{})",
            idx
        ));
        param_values.push(Box::new(plan_type_code.clone()));
    }

    // Status filter
    if let Some(ref status_code) = filters.status_code {
        let idx = param_values.len() + 1;
        conditions.push(format!(
            "c.id IN (SELECT DISTINCT client_id FROM enrollments WHERE status_code = ?{})",
            idx
        ));
        param_values.push(Box::new(status_code.clone()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count total
    let count_sql = format!("SELECT COUNT(*) FROM clients c {}", where_clause);
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let total: i64 = conn.query_row(&count_sql, params_refs.as_slice(), |row| row.get(0))?;

    // Fetch page — join latest active enrollment + carrier for list display
    let limit_idx = param_values.len() + 1;
    let offset_idx = param_values.len() + 2;
    let select_sql = format!(
        "SELECT c.id, c.first_name, c.last_name, c.dob, cr.name, e.plan_name, c.is_active
         FROM clients c
         LEFT JOIN enrollments e ON e.client_id = c.id
           AND e.id = (
             SELECT e2.id FROM enrollments e2
             WHERE e2.client_id = c.id AND e2.is_active = 1
             ORDER BY
               CASE WHEN e2.status_code IN ('ACTIVE', 'PENDING') THEN 0 ELSE 1 END,
               e2.effective_date DESC
             LIMIT 1
           )
         LEFT JOIN carriers cr ON e.carrier_id = cr.id
         {}
         ORDER BY c.last_name, c.first_name
         LIMIT ?{} OFFSET ?{}",
        where_clause, limit_idx, offset_idx
    );

    param_values.push(Box::new(per_page as i64));
    param_values.push(Box::new(offset as i64));
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&select_sql)?;
    let items = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(ClientListItem {
            id: row.get(0)?,
            first_name: row.get(1)?,
            last_name: row.get(2)?,
            dob: row.get(3)?,
            carrier_name: row.get(4)?,
            plan_name: row.get(5)?,
            is_active: row.get(6)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(PaginatedResult {
        items,
        total,
        page,
        per_page,
    })
}

/// Get a single client by ID
pub fn get_client(conn: &Connection, id: &str) -> Result<Client, AppError> {
    let sql = "SELECT id, first_name, last_name, middle_name, dob, gender, phone, phone2, email,
               address_line1, address_line2, city, state, zip, county, mbi, part_a_date, part_b_date,
               orec, is_dual_eligible, dual_status_code, lis_level, medicaid_id,
               lead_source, member_record_locator, is_active, tags, notes,
               created_at, updated_at
               FROM clients WHERE id = ?1";

    conn.query_row(sql, params![id], |row| {
        Ok(Client {
            id: row.get(0)?,
            first_name: row.get(1)?,
            last_name: row.get(2)?,
            middle_name: row.get(3)?,
            dob: row.get(4)?,
            gender: row.get(5)?,
            phone: row.get(6)?,
            phone2: row.get(7)?,
            email: row.get(8)?,
            address_line1: row.get(9)?,
            address_line2: row.get(10)?,
            city: row.get(11)?,
            state: row.get(12)?,
            zip: row.get(13)?,
            county: row.get(14)?,
            mbi: row.get(15)?,
            part_a_date: row.get(16)?,
            part_b_date: row.get(17)?,
            orec: row.get(18)?,
            is_dual_eligible: row.get(19)?,
            dual_status_code: row.get(20)?,
            lis_level: row.get(21)?,
            medicaid_id: row.get(22)?,
            lead_source: row.get(23)?,
            member_record_locator: row.get(24)?,
            is_active: row.get(25)?,
            tags: row.get(26)?,
            notes: row.get(27)?,
            created_at: row.get(28)?,
            updated_at: row.get(29)?,
        })
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Client {} not found", id)),
        _ => AppError::Database(e.to_string()),
    })
}

/// Create a new client
pub fn create_client(conn: &Connection, id: &str, input: &CreateClientInput) -> Result<(), AppError> {
    let sql = "INSERT INTO clients (id, first_name, last_name, middle_name, dob, gender, phone, phone2, email,
               address_line1, address_line2, city, state, zip, county, mbi, part_a_date, part_b_date,
               orec, is_dual_eligible, dual_status_code, lis_level, medicaid_id,
               lead_source, member_record_locator, tags, notes)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
               ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)";

    conn.execute(sql, params![
        id, input.first_name, input.last_name, input.middle_name, input.dob, input.gender,
        input.phone, input.phone2, input.email, input.address_line1, input.address_line2,
        input.city, input.state, input.zip, input.county, input.mbi, input.part_a_date,
        input.part_b_date, input.orec, input.is_dual_eligible,
        input.dual_status_code, input.lis_level, input.medicaid_id, input.lead_source,
        input.member_record_locator, input.tags, input.notes
    ])?;

    Ok(())
}

/// Update a client (only updates provided fields)
pub fn update_client(conn: &Connection, id: &str, input: &UpdateClientInput) -> Result<(), AppError> {
    // Build dynamic UPDATE query - only set fields that are Some
    let mut sets = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    macro_rules! maybe_set {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = input.$field {
                sets.push(format!("{} = ?{}", $col, idx));
                param_values.push(Box::new(val.clone()));
                idx += 1;
            }
        };
    }

    maybe_set!(first_name, "first_name");
    maybe_set!(last_name, "last_name");
    maybe_set!(middle_name, "middle_name");
    maybe_set!(dob, "dob");
    maybe_set!(gender, "gender");
    maybe_set!(phone, "phone");
    maybe_set!(phone2, "phone2");
    maybe_set!(email, "email");
    maybe_set!(address_line1, "address_line1");
    maybe_set!(address_line2, "address_line2");
    maybe_set!(city, "city");
    maybe_set!(state, "state");
    maybe_set!(zip, "zip");
    maybe_set!(county, "county");
    maybe_set!(mbi, "mbi");
    maybe_set!(part_a_date, "part_a_date");
    maybe_set!(part_b_date, "part_b_date");
    maybe_set!(orec, "orec");
    maybe_set!(is_dual_eligible, "is_dual_eligible");
    maybe_set!(dual_status_code, "dual_status_code");
    maybe_set!(lis_level, "lis_level");
    maybe_set!(medicaid_id, "medicaid_id");
    maybe_set!(lead_source, "lead_source");
    maybe_set!(member_record_locator, "member_record_locator");
    maybe_set!(is_active, "is_active");
    maybe_set!(tags, "tags");
    maybe_set!(notes, "notes");

    if sets.is_empty() {
        return Ok(()); // Nothing to update
    }

    let sql = format!("UPDATE clients SET {} WHERE id = ?{}", sets.join(", "), idx);
    param_values.push(Box::new(id.to_string()));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let rows = conn.execute(&sql, params_refs.as_slice())?;

    if rows == 0 {
        return Err(AppError::NotFound(format!("Client {} not found", id)));
    }

    Ok(())
}

/// Soft-delete a client
pub fn delete_client(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows = conn.execute("UPDATE clients SET is_active = 0 WHERE id = ?1", params![id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Client {} not found", id)));
    }
    Ok(())
}

/// Hard-delete a client and all related records (enrollments, conversations, etc.)
pub fn hard_delete_client(conn: &Connection, id: &str) -> Result<(), AppError> {
    // Verify the client exists
    let exists: bool = conn
        .query_row("SELECT 1 FROM clients WHERE id = ?1", params![id], |_| Ok(()))
        .is_ok();
    if !exists {
        return Err(AppError::NotFound(format!("Client {} not found", id)));
    }

    // Delete children in dependency order
    conn.execute("DELETE FROM conversation_entries WHERE client_id = ?1", params![id])?;
    conn.execute("DELETE FROM conversations WHERE client_id = ?1", params![id])?;
    conn.execute("DELETE FROM enrollments WHERE client_id = ?1", params![id])?;
    conn.execute("DELETE FROM client_providers WHERE client_id = ?1", params![id])?;
    conn.execute("DELETE FROM clients WHERE id = ?1", params![id])?;

    // Rebuild FTS index
    conn.execute("INSERT INTO clients_fts(clients_fts) VALUES('rebuild')", [])?;

    Ok(())
}
