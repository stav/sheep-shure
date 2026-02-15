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
