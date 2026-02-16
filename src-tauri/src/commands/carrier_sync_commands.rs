use tauri::{AppHandle, Manager, State, WebviewWindowBuilder, WebviewUrl};

use crate::carrier_sync;
use crate::db::DbState;
use crate::models::{SyncLogEntry, SyncResult};

/// Open a webview window to the carrier's login portal.
#[tauri::command]
pub async fn open_carrier_login(app: AppHandle, carrier_id: String) -> Result<String, String> {
    let portal = carrier_sync::get_portal(&carrier_id)
        .ok_or_else(|| format!("No portal integration for carrier: {}", carrier_id))?;

    let url = portal.login_url().to_string();

    // Close existing carrier-login window if open
    if let Some(existing) = app.get_webview_window("carrier-login") {
        existing.close().map_err(|e| e.to_string())?;
    }

    WebviewWindowBuilder::new(&app, "carrier-login", WebviewUrl::External(url.parse().unwrap()))
        .title(format!("{} Login", portal.carrier_name()))
        .inner_size(1200.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(portal.login_url().to_string())
}

/// Run the full carrier sync: fetch portal data, compare with local, auto-update.
#[tauri::command]
pub async fn sync_carrier_portal(
    carrier_id: String,
    auth_token: String,
    state: State<'_, DbState>,
) -> Result<SyncResult, String> {
    let portal = carrier_sync::get_portal(&carrier_id)
        .ok_or_else(|| format!("No portal integration for carrier: {}", carrier_id))?;

    let carrier_name = portal.carrier_name().to_string();

    // Fetch members from the carrier portal (async HTTP)
    let portal_members = portal
        .fetch_members(&auth_token)
        .await
        .map_err(|e| e.to_string())?;

    // Compare against local data and update (sync, uses DB)
    let result = state
        .with_conn(|conn| {
            crate::services::carrier_sync_service::run_sync(
                conn,
                &carrier_id,
                &carrier_name,
                &portal_members,
            )
        })
        .map_err(|e| e.to_string())?;

    Ok(result)
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
