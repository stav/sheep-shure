use tauri::State;
use crate::db::DbState;
use crate::models::Carrier;
use crate::repositories::carrier_repo;

#[tauri::command]
pub fn get_carriers(state: State<'_, DbState>) -> Result<Vec<Carrier>, String> {
    state.with_conn(|conn| {
        carrier_repo::get_carriers(conn)
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_carrier_expected_active(
    state: State<'_, DbState>,
    carrier_id: String,
    expected_active: i32,
) -> Result<(), String> {
    state.with_conn(|conn| {
        carrier_repo::update_expected_active(conn, &carrier_id, expected_active)
    }).map_err(|e| e.to_string())
}
