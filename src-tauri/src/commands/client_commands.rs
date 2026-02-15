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

#[tauri::command]
pub fn delete_all_clients(state: State<'_, DbState>) -> Result<serde_json::Value, String> {
    state.with_conn(|conn| {
        // Delete related data first (foreign key children)
        conn.execute("DELETE FROM notes WHERE client_id IN (SELECT id FROM clients)", [])?;
        conn.execute("DELETE FROM enrollments WHERE client_id IN (SELECT id FROM clients)", [])?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0))
            .unwrap_or(0);
        conn.execute("DELETE FROM clients", [])?;
        // Rebuild FTS index
        conn.execute("INSERT INTO clients_fts(clients_fts) VALUES('rebuild')", [])?;
        Ok(serde_json::json!({ "deleted": count }))
    }).map_err(|e| e.to_string())
}
