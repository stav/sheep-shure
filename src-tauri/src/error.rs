use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Clone, Serialize)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Import error: {0}")]
    Import(String),

    #[error("IO error: {0}")]
    Io(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

// Note: AppError implements Serialize, so Tauri's blanket impl
// `From<T: Serialize> for InvokeError` automatically handles conversion.
// No manual From<AppError> for InvokeError is needed.
