use tauri::State;

use crate::db::DbState;
use crate::models::report::DashboardStats;
use crate::services::dashboard_service;

#[tauri::command]
pub fn get_dashboard_stats(state: State<'_, DbState>) -> Result<DashboardStats, String> {
    state
        .with_conn(|conn| dashboard_service::get_dashboard_stats(conn))
        .map_err(|e| e.to_string())
}
