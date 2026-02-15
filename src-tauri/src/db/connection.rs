use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::AppError;

pub struct DbState {
    pub conn: Mutex<Option<Connection>>,
}

impl DbState {
    pub fn new() -> Self {
        DbState {
            conn: Mutex::new(None),
        }
    }

    /// Execute a closure with a reference to the database connection.
    /// Returns an error if the database is not initialized or the mutex is poisoned.
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, AppError>,
    {
        let guard = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Failed to acquire database lock: {}", e)))?;

        match guard.as_ref() {
            Some(conn) => f(conn),
            None => Err(AppError::Database(
                "Database connection not initialized".to_string(),
            )),
        }
    }

    /// Set the database connection.
    pub fn set_connection(&self, connection: Connection) -> Result<(), AppError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Failed to acquire database lock: {}", e)))?;

        *guard = Some(connection);
        Ok(())
    }

    /// Clear the database connection (used for logout).
    pub fn clear_connection(&self) -> Result<(), AppError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Failed to acquire database lock: {}", e)))?;

        *guard = None;
        Ok(())
    }
}
