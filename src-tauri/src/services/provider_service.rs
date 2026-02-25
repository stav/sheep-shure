use rusqlite::Connection;
use uuid::Uuid;
use crate::error::AppError;
use crate::models::{ClientProvider, CreateProviderInput};
use crate::repositories::provider_repo;

pub fn create_provider(conn: &Connection, input: &CreateProviderInput) -> Result<ClientProvider, AppError> {
    let id = Uuid::new_v4().to_string();
    provider_repo::create_provider(conn, &id, input)?;

    // Return the created provider by fetching it from the list
    let providers = provider_repo::get_providers_for_client(conn, &input.client_id)?;
    providers
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| AppError::Database("Failed to fetch created provider".into()))
}

pub fn get_providers_for_client(conn: &Connection, client_id: &str) -> Result<Vec<ClientProvider>, AppError> {
    provider_repo::get_providers_for_client(conn, client_id)
}
