use tauri::State;
use crate::db::DbState;
use crate::models::{Client, ClientFilters, ClientListItem, CreateClientInput, PaginatedResult, UpdateClientInput};
use crate::services::client_service;

#[tauri::command]
pub fn get_clients(
    filters: ClientFilters,
    page: i32,
    per_page: i32,
    state: State<'_, DbState>,
) -> Result<PaginatedResult<ClientListItem>, String> {
    state.with_conn(|conn| {
        client_service::get_clients(conn, &filters, page, per_page)
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_client(id: String, state: State<'_, DbState>) -> Result<Client, String> {
    state.with_conn(|conn| {
        client_service::get_client(conn, &id)
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_client(input: CreateClientInput, state: State<'_, DbState>) -> Result<Client, String> {
    state.with_conn(|conn| {
        client_service::create_client(conn, &input)
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_client(id: String, input: UpdateClientInput, state: State<'_, DbState>) -> Result<Client, String> {
    state.with_conn(|conn| {
        client_service::update_client(conn, &id, &input)
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_client(id: String, state: State<'_, DbState>) -> Result<(), String> {
    state.with_conn(|conn| {
        client_service::delete_client(conn, &id)
    }).map_err(|e| e.to_string())
}
