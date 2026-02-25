use rusqlite::Connection;
use crate::error::AppError;
use crate::models::Carrier;

pub fn get_carriers(conn: &Connection) -> Result<Vec<Carrier>, AppError> {
    let sql = "SELECT id, name, short_name, is_active, expected_active FROM carriers WHERE is_active = 1 ORDER BY name";
    let mut stmt = conn.prepare(sql)?;
    let items = stmt.query_map([], |row| {
        Ok(Carrier {
            id: row.get(0)?,
            name: row.get(1)?,
            short_name: row.get(2)?,
            is_active: row.get(3)?,
            expected_active: row.get(4)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

pub fn update_expected_active(conn: &Connection, carrier_id: &str, count: i32) -> Result<(), AppError> {
    conn.execute(
        "UPDATE carriers SET expected_active = ?1 WHERE id = ?2",
        rusqlite::params![count, carrier_id],
    )?;
    Ok(())
}
