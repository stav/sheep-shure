use tauri::{AppHandle, Emitter, Manager, State, WebviewWindowBuilder, WebviewUrl};

use crate::carrier_sync;
use crate::db::DbState;
use crate::models::{CarrierSyncInfo, ConfirmDisenrollmentResult, ImportLogEntry, ImportPortalResult, PortalCredentials, PortalMember, SyncLogEntry, SyncResult};

fn emit_log(app: &AppHandle, level: &str, phase: &str, message: &str, detail: Option<&str>) {
    let entry = ImportLogEntry {
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        level: level.to_string(),
        phase: phase.to_string(),
        message: message.to_string(),
        detail: detail.map(String::from),
    };
    let _ = app.emit("commission-import-log", &entry);
}

/// Open a webview window to the carrier's login portal.
/// Sets up a navigation interceptor to catch sync results from injected JS.
/// If saved credentials exist, injects auto-login script.
#[tauri::command]
pub async fn open_carrier_login(
    app: AppHandle,
    carrier_id: String,
    state: State<'_, DbState>,
) -> Result<String, String> {
    let portal = carrier_sync::get_portal(&carrier_id)
        .ok_or_else(|| format!("No portal integration for carrier: {}", carrier_id))?;

    let url = portal.login_url().to_string();
    let login_url: tauri::Url = url.parse().unwrap();
    let login_host = login_url.host_str().unwrap_or("").to_string();

    // Reuse existing webview if it's already on this carrier's domain
    if let Some(existing) = app.get_webview_window("carrier-login") {
        let current_host = existing.url().ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_default();

        if current_host == login_host {
            let _ = existing.set_focus();
            // Re-inject the fetch script so auto-fetch carriers re-sync
            let _ = existing.eval(portal.fetch_script());
            return Ok(url);
        }

        existing.close().map_err(|e| e.to_string())?;
    }

    // Look up saved credentials
    let creds_key = format!("portal_creds_{}", carrier_id);
    let saved_creds: Option<PortalCredentials> = state
        .with_conn(|conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT value FROM app_settings WHERE key = ?1",
                    rusqlite::params![creds_key],
                    |row| row.get(0),
                )
                .ok();
            match result {
                Some(json_str) => {
                    let creds: PortalCredentials = serde_json::from_str(&json_str)
                        .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                    Ok(Some(creds))
                }
                None => Ok(None),
            }
        })
        .map_err(|e| e.to_string())?;

    // Build combined initialization script
    let mut combined_script = String::new();

    // Override window.open — Tauri webviews have no popup/tab support,
    // so redirect window.open() calls to navigate in the current window.
    combined_script.push_str(
        "(function(){window.open=function(url){if(url)window.location.href=url;return window;}})();\n",
    );

    if let Some(ref creds) = saved_creds {
        // Inject credentials as a JS object (JSON-escaped for safety)
        let creds_json = serde_json::json!({
            "username": creds.username,
            "password": creds.password,
        });
        combined_script.push_str(&format!(
            "window.__compass_creds = {};\n",
            creds_json
        ));
    }

    combined_script.push_str(portal.init_script());

    if saved_creds.is_some() {
        let auto_login = portal.auto_login_script();
        if !auto_login.is_empty() {
            combined_script.push('\n');
            combined_script.push_str(auto_login);
        }
    }

    let nav_handle = app.clone();
    let dl_handle = app.clone();
    let dl_log_handle = app.clone();
    // Shared state: the JS sends the detected statement month via a callback
    // before triggering the CSV download.  The download handler reads it.
    let pending_month = std::sync::Arc::new(std::sync::Mutex::new(Option::<String>::None));
    let nav_month = pending_month.clone();
    let dl_month = pending_month.clone();

    WebviewWindowBuilder::new(&app, "carrier-login", WebviewUrl::External(url.parse().unwrap()))
        .title(format!("{} Login", portal.carrier_name()))
        .inner_size(1200.0, 800.0)
        .initialization_script(&combined_script)
        .on_navigation(move |nav_url| {
            let host = nav_url.host_str().unwrap_or("");
            if host == "compass-sync.localhost" {
                let path = nav_url.path();
                if path == "/data" {
                    if let Some(members_val) = nav_url.query_pairs().find(|(k, _)| k == "members") {
                        let _ = nav_handle.emit("carrier-sync-data", members_val.1.to_string());
                    }
                } else if path == "/commission" {
                    if let Some(val) = nav_url.query_pairs().find(|(k, _)| k == "statements") {
                        let _ = nav_handle.emit("carrier-commission-data", val.1.to_string());
                    }
                } else if path == "/commission-month" {
                    // Store the detected month for the next download
                    if let Some(m) = nav_url.query_pairs().find(|(k, _)| k == "month") {
                        let month_str = m.1.to_string();
                        if !month_str.is_empty() {
                            tracing::info!("Commission month detected from page: {}", month_str);
                            emit_log(&nav_handle, "info", "portal", &format!("Detected statement month: {}", month_str), None);
                            *nav_month.lock().unwrap() = Some(month_str);
                        }
                    }
                } else if path == "/error" {
                    if let Some(err_val) = nav_url.query_pairs().find(|(k, _)| k == "message") {
                        emit_log(&nav_handle, "error", "portal", &err_val.1, None);
                        let _ = nav_handle.emit("carrier-sync-error", err_val.1.to_string());
                    }
                }
                return false;
            }
            true
        })
        .on_download(move |_webview, event| {
            use tauri::webview::DownloadEvent;
            match event {
                DownloadEvent::Requested { url, destination } => {
                    let fname = format!("compass-dl-{}.csv", std::process::id());
                    let tmp = std::env::temp_dir().join(fname);
                    tracing::info!("Download requested: url={}, dest={:?}", url, tmp);
                    emit_log(&dl_log_handle, "info", "download", "Download started", None);
                    *destination = tmp;
                    true
                }
                DownloadEvent::Finished { url, path, success } => {
                    tracing::info!("Download finished: url={}, success={}, path={:?}", url, success, path);
                    if success {
                        if let Some(ref path) = path {
                            match std::fs::read_to_string(path) {
                                Ok(content) => {
                                    if content.contains('|') && !content.trim_start().starts_with('<') {
                                        emit_log(&dl_log_handle, "success", "download",
                                            &format!("Download complete: {} bytes, pipe-delimited CSV", content.len()), None);
                                        // Use the month detected from the page, or null for fallback
                                        let month = dl_month.lock().unwrap().take();
                                        let month_val = match month {
                                            Some(m) => serde_json::Value::String(m),
                                            None => serde_json::Value::Null,
                                        };
                                        let payload = serde_json::json!([{
                                            "month": month_val,
                                            "csv": content
                                        }]).to_string();
                                        let _ = dl_handle.emit("carrier-commission-data", payload);
                                        tracing::info!("Commission CSV download captured: {} bytes", content.len());
                                    } else {
                                        emit_log(&dl_log_handle, "warn", "download", "Downloaded file is not CSV data, skipping", None);
                                        tracing::warn!("Downloaded file is not CSV data (first 100 chars: {:?})", &content[..content.len().min(100)]);
                                    }
                                }
                                Err(e) => {
                                    emit_log(&dl_log_handle, "error", "download", &format!("Failed to read downloaded file: {}", e), None);
                                    tracing::error!("Failed to read downloaded file: {}", e);
                                }
                            }
                            let _ = std::fs::remove_file(path);
                        }
                    }
                    true
                }
                _ => true,
            }
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

/// Get sync behaviour info for a carrier (auto_fetch, instruction text).
#[tauri::command]
pub fn get_carrier_sync_info(carrier_id: String) -> Result<CarrierSyncInfo, String> {
    let portal = carrier_sync::get_portal(&carrier_id)
        .ok_or_else(|| format!("No portal integration for carrier: {}", carrier_id))?;

    Ok(CarrierSyncInfo {
        auto_fetch: portal.auto_fetch(),
        sync_instruction: portal.sync_instruction().to_string(),
    })
}

/// Import selected portal members as new clients with enrollments.
#[tauri::command]
pub fn import_portal_members(
    carrier_id: String,
    members_json: String,
    state: State<'_, DbState>,
) -> Result<ImportPortalResult, String> {
    let members: Vec<PortalMember> =
        serde_json::from_str(&members_json).map_err(|e| format!("Failed to parse members: {}", e))?;

    state
        .with_conn(|conn| {
            crate::services::carrier_sync_service::import_portal_members(conn, &carrier_id, &members)
        })
        .map_err(|e| e.to_string())
}

/// Confirm disenrollment for selected enrollment IDs.
#[tauri::command]
pub fn confirm_disenrollments(
    enrollment_ids: Vec<String>,
    state: State<'_, DbState>,
) -> Result<ConfirmDisenrollmentResult, String> {
    state
        .with_conn(|conn| {
            crate::services::carrier_sync_service::confirm_disenrollments(conn, &enrollment_ids)
        })
        .map_err(|e| e.to_string())
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

/// Save portal credentials for a carrier (stored in app_settings).
#[tauri::command]
pub fn save_portal_credentials(
    carrier_id: String,
    username: String,
    password: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let key = format!("portal_creds_{}", carrier_id);
    let value = serde_json::json!({ "username": username, "password": password }).to_string();
    state
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
                rusqlite::params![key, value],
            )
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            Ok(())
        })
        .map_err(|e| e.to_string())
}

/// Get saved portal credentials for a carrier.
#[tauri::command]
pub fn get_portal_credentials(
    carrier_id: String,
    state: State<'_, DbState>,
) -> Result<Option<PortalCredentials>, String> {
    let key = format!("portal_creds_{}", carrier_id);
    state
        .with_conn(|conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT value FROM app_settings WHERE key = ?1",
                    rusqlite::params![key],
                    |row| row.get(0),
                )
                .ok();
            match result {
                Some(json_str) => {
                    let creds: PortalCredentials = serde_json::from_str(&json_str)
                        .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                    Ok(Some(creds))
                }
                None => Ok(None),
            }
        })
        .map_err(|e| e.to_string())
}

/// Delete saved portal credentials for a carrier.
#[tauri::command]
pub fn delete_portal_credentials(
    carrier_id: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let key = format!("portal_creds_{}", carrier_id);
    state
        .with_conn(|conn| {
            conn.execute(
                "DELETE FROM app_settings WHERE key = ?1",
                rusqlite::params![key],
            )
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            Ok(())
        })
        .map_err(|e| e.to_string())
}

/// Get list of carrier IDs that have saved credentials.
#[tauri::command]
pub fn get_carriers_with_credentials(
    state: State<'_, DbState>,
) -> Result<Vec<String>, String> {
    state
        .with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT key FROM app_settings WHERE key LIKE 'portal_creds_%'")
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            let mut carrier_ids = Vec::new();
            for row in rows {
                let key = row.map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                if let Some(id) = key.strip_prefix("portal_creds_") {
                    carrier_ids.push(id.to_string());
                }
            }
            Ok(carrier_ids)
        })
        .map_err(|e| e.to_string())
}
