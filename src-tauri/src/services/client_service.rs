use rusqlite::Connection;
use uuid::Uuid;
use crate::error::AppError;
use crate::models::{Client, ClientFilters, ClientListItem, CreateClientInput, UpdateClientInput, PaginatedResult};
use crate::repositories::client_repo;

/// Validate MBI format: 11 characters, specific pattern
fn validate_mbi(mbi: &str) -> Result<(), AppError> {
    if mbi.is_empty() {
        return Ok(());
    }
    // MBI format: [1-9][AC-HJKMNP-RT][0-9AC-HJKMNP-RT][0-9]-[AC-HJKMNP-RT][AC-HJKMNP-RT0-9][0-9]-[AC-HJKMNP-RT][AC-HJKMNP-RT0-9][0-9][0-9]
    // Simplified: 11 alphanumeric characters (no S, L, O, I, B, Z)
    if mbi.len() != 11 {
        return Err(AppError::Validation(format!("MBI must be 11 characters, got {}", mbi.len())));
    }
    let valid = mbi.chars().all(|c| c.is_ascii_alphanumeric());
    if !valid {
        return Err(AppError::Validation("MBI must contain only letters and numbers".to_string()));
    }
    Ok(())
}

pub fn get_clients(conn: &Connection, filters: &ClientFilters, page: i32, per_page: i32) -> Result<PaginatedResult<ClientListItem>, AppError> {
    let page = if page < 1 { 1 } else { page };
    let per_page = per_page.clamp(1, 100);
    client_repo::get_clients(conn, filters, page, per_page)
}

pub fn get_client(conn: &Connection, id: &str) -> Result<Client, AppError> {
    client_repo::get_client(conn, id)
}

pub fn create_client(conn: &Connection, input: &CreateClientInput) -> Result<Client, AppError> {
    // Validate MBI if provided
    if let Some(ref mbi) = input.mbi {
        validate_mbi(mbi)?;
    }

    let id = Uuid::new_v4().to_string();
    client_repo::create_client(conn, &id, input)?;
    client_repo::get_client(conn, &id)
}

pub fn update_client(conn: &Connection, id: &str, input: &UpdateClientInput) -> Result<Client, AppError> {
    if let Some(ref mbi) = input.mbi {
        validate_mbi(mbi)?;
    }
    client_repo::update_client(conn, id, input)?;
    client_repo::get_client(conn, id)
}

pub fn delete_client(conn: &Connection, id: &str) -> Result<(), AppError> {
    client_repo::delete_client(conn, id)
}
