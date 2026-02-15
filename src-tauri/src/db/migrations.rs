use rusqlite::Connection;

use crate::error::AppError;

struct Migration {
    version: i32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("migrations/v001_initial.sql"),
}];

/// Run all pending migrations against the database.
/// Uses PRAGMA user_version to track which migrations have been applied.
pub fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    let current_version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| AppError::Database(format!("Failed to read user_version: {}", e)))?;

    tracing::info!("Current database version: {}", current_version);

    for migration in MIGRATIONS {
        if migration.version > current_version {
            tracing::info!("Applying migration V{}...", migration.version);

            conn.execute_batch(migration.sql).map_err(|e| {
                AppError::Database(format!(
                    "Failed to apply migration V{}: {}",
                    migration.version, e
                ))
            })?;

            conn.pragma_update(None, "user_version", migration.version)
                .map_err(|e| {
                    AppError::Database(format!("Failed to update user_version: {}", e))
                })?;

            tracing::info!("Migration V{} applied successfully", migration.version);
        }
    }

    Ok(())
}
