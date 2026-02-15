use tauri::State;

use crate::db::DbState;
use crate::services::auth_service;
use crate::AppDataDir;

#[tauri::command]
pub fn check_first_run(app_data_dir: State<'_, AppDataDir>) -> Result<bool, String> {
    Ok(auth_service::is_first_run(&app_data_dir.0))
}

#[tauri::command]
pub fn create_account(
    password: String,
    app_data_dir: State<'_, AppDataDir>,
    db_state: State<'_, DbState>,
) -> Result<(), String> {
    let conn = auth_service::create_database(&app_data_dir.0, &password)
        .map_err(|e| e.to_string())?;

    db_state.set_connection(conn).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn login(
    password: String,
    app_data_dir: State<'_, AppDataDir>,
    db_state: State<'_, DbState>,
) -> Result<(), String> {
    let conn = auth_service::unlock_database(&app_data_dir.0, &password)
        .map_err(|e| e.to_string())?;

    db_state.set_connection(conn).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn logout(db_state: State<'_, DbState>) -> Result<(), String> {
    db_state.clear_connection().map_err(|e| e.to_string())?;
    Ok(())
}
