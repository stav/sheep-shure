pub mod carrier_sync;
pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod repositories;
pub mod services;

use std::path::PathBuf;
use tauri::Manager;
use db::DbState;

pub struct AppDataDir(pub PathBuf);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let db_state = DbState::new();

    tauri::Builder::default()
        .manage(db_state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            std::fs::create_dir_all(&app_data_dir)
                .expect("Failed to create app data directory");

            tracing::info!("App data directory: {:?}", app_data_dir);

            app.manage(AppDataDir(app_data_dir));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_first_run,
            commands::create_account,
            commands::login,
            commands::logout,
            commands::get_clients,
            commands::get_client,
            commands::create_client,
            commands::update_client,
            commands::delete_client,
            commands::delete_all_clients,
            commands::get_enrollments,
            commands::create_enrollment,
            commands::update_enrollment,
            commands::get_conversations,
            commands::get_conversation,
            commands::create_conversation,
            commands::update_conversation,
            commands::get_conversation_entries,
            commands::create_conversation_entry,
            commands::update_conversation_entry,
            commands::get_client_timeline,
            commands::get_pending_follow_ups,
            commands::get_carriers,
            commands::parse_import_file,
            commands::validate_import,
            commands::execute_import,
            commands::import_call_log,
            commands::get_dashboard_stats,
            commands::get_report,
            commands::export_report_pdf,
            commands::get_settings,
            commands::update_settings,
            commands::get_agent_profile,
            commands::save_agent_profile,
            commands::backup_database,
            commands::get_database_info,
            commands::open_carrier_login,
            commands::trigger_carrier_fetch,
            commands::process_portal_members,
            commands::get_carrier_login_url,
            commands::get_sync_logs,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running SHEEPS application");
}
