use tauri::{AppHandle, Emitter, Manager, State, WebviewWindowBuilder, WebviewUrl};

use crate::carrier_sync;
use crate::db::DbState;
use crate::models::{PortalMember, SyncLogEntry, SyncResult};

/// Open a webview window to the carrier's login portal.
/// Sets up a navigation interceptor to catch sync results from injected JS.
#[tauri::command]
pub async fn open_carrier_login(app: AppHandle, carrier_id: String) -> Result<String, String> {
    let portal = carrier_sync::get_portal(&carrier_id)
        .ok_or_else(|| format!("No portal integration for carrier: {}", carrier_id))?;

    let url = portal.login_url().to_string();

    // Close existing carrier-login window if open
    if let Some(existing) = app.get_webview_window("carrier-login") {
        existing.close().map_err(|e| e.to_string())?;
    }

    let init_script = portal.init_script().to_string();
    let app_handle = app.clone();
    WebviewWindowBuilder::new(&app, "carrier-login", WebviewUrl::External(url.parse().unwrap()))
        .title(format!("{} Login", portal.carrier_name()))
        .inner_size(1200.0, 800.0)
        .initialization_script(&init_script)
        .on_navigation(move |nav_url| {
            let host = nav_url.host_str().unwrap_or("");
            if host == "sheeps-sync.localhost" {
                let path = nav_url.path();
                if path == "/data" {
                    // Extract the members JSON from the query string
                    if let Some(members_val) = nav_url.query_pairs().find(|(k, _)| k == "members") {
                        let _ = app_handle.emit("carrier-sync-data", members_val.1.to_string());
                    }
                } else if path == "/error" {
                    if let Some(err_val) = nav_url.query_pairs().find(|(k, _)| k == "message") {
                        let _ = app_handle.emit("carrier-sync-error", err_val.1.to_string());
                    }
                }
                return false; // block navigation to the fake URL
            }
            true // allow all other navigation
        })
        .build()
        .map_err(|e| e.to_string())?;

    Ok(portal.login_url().to_string())
}

/// Inject the fetch script into the carrier login webview.
/// The script fetches member data using the browser's cookies and navigates
/// to a callback URL that on_navigation intercepts.
#[tauri::command]
pub async fn trigger_carrier_fetch(app: AppHandle, carrier_id: String) -> Result<(), String> {
    let portal = carrier_sync::get_portal(&carrier_id)
        .ok_or_else(|| format!("No portal integration for carrier: {}", carrier_id))?;

    let webview = app
        .get_webview_window("carrier-login")
        .ok_or("Carrier login window is not open. Open the portal and log in first.")?;

    webview
        .eval(portal.fetch_script())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Process portal member data that was fetched by the webview JS.
/// Compares against local enrollments and auto-updates disenrolled records.
#[tauri::command]
pub fn process_portal_members(
    carrier_id: String,
    members_json: String,
    state: State<'_, DbState>,
) -> Result<SyncResult, String> {
    let portal = carrier_sync::get_portal(&carrier_id)
        .ok_or_else(|| format!("No portal integration for carrier: {}", carrier_id))?;

    let portal_members: Vec<PortalMember> =
        serde_json::from_str(&members_json).map_err(|e| format!("Failed to parse member data: {}", e))?;

    let carrier_name = portal.carrier_name().to_string();

    state
        .with_conn(|conn| {
            crate::services::carrier_sync_service::run_sync(
                conn,
                &carrier_id,
                &carrier_name,
                &portal_members,
            )
        })
        .map_err(|e| e.to_string())
}

/// Get the login URL for a carrier portal.
#[tauri::command]
pub fn get_carrier_login_url(carrier_id: String) -> Result<String, String> {
    let portal = carrier_sync::get_portal(&carrier_id)
        .ok_or_else(|| format!("No portal integration for carrier: {}", carrier_id))?;

    Ok(portal.login_url().to_string())
}

/// Get sync log history.
#[tauri::command]
pub fn get_sync_logs(
    carrier_id: Option<String>,
    state: State<'_, DbState>,
) -> Result<Vec<SyncLogEntry>, String> {
    state
        .with_conn(|conn| {
            crate::services::carrier_sync_service::get_sync_logs(conn, carrier_id.as_deref())
        })
        .map_err(|e| e.to_string())
}
