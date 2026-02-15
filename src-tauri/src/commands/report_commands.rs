use tauri::State;

use crate::db::DbState;
use crate::models::report::{DashboardStats, ReportDefinition};
use crate::services::{dashboard_service, report_service};
use crate::AppDataDir;

#[tauri::command]
pub fn get_dashboard_stats(state: State<'_, DbState>) -> Result<DashboardStats, String> {
    state
        .with_conn(|conn| dashboard_service::get_dashboard_stats(conn))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_report(
    definition: ReportDefinition,
    state: State<'_, DbState>,
) -> Result<serde_json::Value, String> {
    state
        .with_conn(|conn| report_service::run_report(conn, &definition))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_report_pdf(
    definition: ReportDefinition,
    app_data_dir: State<'_, AppDataDir>,
    state: State<'_, DbState>,
) -> Result<String, String> {
    state
        .with_conn(|conn| report_service::generate_pdf(conn, &definition, &app_data_dir.0))
        .map_err(|e| e.to_string())
}
