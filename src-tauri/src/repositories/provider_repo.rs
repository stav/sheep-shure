use rusqlite::{params, Connection};
use crate::error::AppError;
use crate::models::{ClientProvider, CreateProviderInput};

pub fn create_provider(conn: &Connection, id: &str, input: &CreateProviderInput) -> Result<(), AppError> {
    let sql = "INSERT INTO client_providers (id, client_id, first_name, last_name, npi, specialty, phone, is_pcp, source)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";

    conn.execute(sql, params![
        id, input.client_id, input.first_name, input.last_name, input.npi,
        input.specialty, input.phone, input.is_pcp, input.source
    ])?;

    Ok(())
}

pub fn get_providers_for_client(conn: &Connection, client_id: &str) -> Result<Vec<ClientProvider>, AppError> {
    let sql = "SELECT id, client_id, first_name, last_name, npi, specialty, phone, is_pcp, source, is_active, created_at, updated_at
               FROM client_providers
               WHERE client_id = ?1 AND is_active = 1
               ORDER BY is_pcp DESC, last_name, first_name";

    let mut stmt = conn.prepare(sql)?;
    let items = stmt.query_map(params![client_id], |row| {
        Ok(ClientProvider {
            id: row.get(0)?,
            client_id: row.get(1)?,
            first_name: row.get(2)?,
            last_name: row.get(3)?,
            npi: row.get(4)?,
            specialty: row.get(5)?,
            phone: row.get(6)?,
            is_pcp: row.get(7)?,
            source: row.get(8)?,
            is_active: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}
