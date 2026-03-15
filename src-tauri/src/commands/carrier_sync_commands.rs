use tauri::{AppHandle, Emitter, Manager, State, WebviewWindowBuilder, WebviewUrl, webview::PageLoadEvent};

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
    // Some portals (e.g. Anthem) opt out because their SSO flow uses
    // window.open in ways that cause redirect loops when overridden.
    if portal.override_window_open() {
        combined_script.push_str(
            "(function(){window.open=function(url){if(url)window.location.href=url;return window;}})();\n",
        );
    }

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

    let page_load_handle = app.clone();
    let nav_handle = app.clone();
    let dl_handle = app.clone();
    let dl_log_handle = app.clone();
    // Shared state: the JS sends the detected statement month via a callback
    // before triggering the CSV download.  The download handler reads it.
    let pending_month = std::sync::Arc::new(std::sync::Mutex::new(Option::<String>::None));
    let nav_month = pending_month.clone();
    let dl_month = pending_month.clone();
    let dl_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let nav_file_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    WebviewWindowBuilder::new(&app, "carrier-login", WebviewUrl::External(url.parse().unwrap()))
        .title(format!("{} Login", portal.carrier_name()))
        .inner_size(1200.0, 800.0)
        .initialization_script(&combined_script)
        .on_navigation(move |nav_url| {
            let host = nav_url.host_str().unwrap_or("");
            tracing::debug!("[navigation] url={}, host={}", nav_url, host);
            if host == "compass-sync.localhost" {
                let path = nav_url.path();
                tracing::info!("[navigation] compass-sync intercepted: path={}, full={}", path, nav_url);
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
                } else if path == "/commission-file" {
                    // Receive base64-encoded spreadsheet file from JS fetch
                    let month = nav_url.query_pairs()
                        .find(|(k, _)| k == "month")
                        .map(|(_, v)| v.to_string())
                        .unwrap_or_default();
                    if let Some(data_val) = nav_url.query_pairs().find(|(k, _)| k == "data") {
                        let b64 = data_val.1.to_string();
                        tracing::info!("[navigation] commission-file: month={}, base64 len={}", month, b64.len());
                        use base64::Engine;
                        match base64::engine::general_purpose::STANDARD.decode(&b64) {
                            Ok(bytes) => {
                                // Detect file type from magic bytes
                                let ext = if bytes.starts_with(&[0x50, 0x4b, 0x03, 0x04]) {
                                    "xlsx" // ZIP-based (XLSX/OOXML)
                                } else if bytes.starts_with(&[0xd0, 0xcf, 0x11, 0xe0]) {
                                    "xls" // OLE2 (legacy XLS)
                                } else {
                                    "xlsx" // default to xlsx
                                };
                                let seq = nav_file_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                let fname = format!("compass-commission-{}-{}.{}", std::process::id(), seq, ext);
                                let tmp_path = std::env::temp_dir().join(&fname);
                                match std::fs::write(&tmp_path, &bytes) {
                                    Ok(_) => {
                                        tracing::info!("[navigation] Wrote {} bytes to {:?}", bytes.len(), tmp_path);
                                        emit_log(&nav_handle, "success", "download",
                                            &format!("Commission file: {} bytes, month={}", bytes.len(), month), None);
                                        let month_val = if month.is_empty() {
                                            serde_json::Value::Null
                                        } else {
                                            serde_json::Value::String(month.clone())
                                        };
                                        let payload = serde_json::json!({
                                            "month": month_val,
                                            "filePath": tmp_path.to_string_lossy()
                                        }).to_string();
                                        let _ = nav_handle.emit("carrier-commission-file", &payload);
                                    }
                                    Err(e) => {
                                        tracing::error!("[navigation] Failed to write temp file: {}", e);
                                        emit_log(&nav_handle, "error", "download",
                                            &format!("Failed to write temp file: {}", e), None);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("[navigation] Base64 decode failed: {}", e);
                                emit_log(&nav_handle, "error", "download",
                                    &format!("Base64 decode failed: {}", e), None);
                            }
                        }
                    }
                } else if path == "/page-ready" {
                    // Poll-based readiness signal from commission fetch
                    let csv_count = nav_url.query_pairs()
                        .find(|(k, _)| k == "csv")
                        .map(|(_, v)| v.to_string())
                        .unwrap_or_default();
                    tracing::info!("[navigation] ★ page-ready signal: csv_count={}", csv_count);
                    emit_log(&nav_handle, "info", "fetch",
                        &format!("Results page detected: {} CSV links found", csv_count), None);
                    match nav_handle.emit("carrier-page-ready", csv_count.clone()) {
                        Ok(_) => tracing::info!("[navigation] carrier-page-ready emitted OK (csv={})", csv_count),
                        Err(e) => tracing::error!("[navigation] Failed to emit carrier-page-ready: {}", e),
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
        .on_page_load(move |_window, payload| {
            match payload.event() {
                PageLoadEvent::Started => {
                    tracing::info!("[page-load] STARTED: {}", payload.url());
                }
                PageLoadEvent::Finished => {
                    tracing::info!("[page-load] FINISHED: {}", payload.url());
                    tracing::info!("[page-load] Emitting carrier-page-loaded event");
                    match page_load_handle.emit("carrier-page-loaded", ()) {
                        Ok(_) => tracing::info!("[page-load] carrier-page-loaded emitted OK"),
                        Err(e) => tracing::error!("[page-load] Failed to emit carrier-page-loaded: {}", e),
                    }
                }
                // PageLoadEvent is exhaustive (Started + Finished only)
            }
        })
        .on_download(move |_webview, event| {
            use tauri::webview::DownloadEvent;
            match event {
                DownloadEvent::Requested { url, destination } => {
                    let seq = dl_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    // Determine file extension from URL
                    let url_str = url.as_str();
                    let ext = url_str.rsplit('/').next()
                        .and_then(|segment: &str| {
                            let segment = segment.split('?').next().unwrap_or(segment);
                            let lower = segment.to_lowercase();
                            if lower.ends_with(".xls") { Some("xls") }
                            else if lower.ends_with(".xlsx") { Some("xlsx") }
                            else if lower.ends_with(".csv") { Some("csv") }
                            else if lower.ends_with(".txt") { Some("txt") }
                            else { None }
                        })
                        .unwrap_or("bin");
                    let fname = format!("compass-dl-{}-{}.{}", std::process::id(), seq, ext);
                    let tmp = std::env::temp_dir().join(fname);
                    let pending = dl_month.lock().unwrap().clone();
                    tracing::info!("[download] REQUESTED: url={}", url);
                    tracing::info!("[download]   dest={:?}, pending_month={:?}", tmp, pending);
                    emit_log(&dl_log_handle, "info", "download",
                        &format!("Download started (pending month: {:?})", pending),
                        Some(url.as_str()));
                    *destination = tmp;
                    true
                }
                DownloadEvent::Finished { url, path, success } => {
                    tracing::info!("[download] FINISHED: success={}, path={:?}", success, path);
                    tracing::info!("[download]   url={}", url);
                    emit_log(&dl_log_handle, "info", "download",
                        &format!("Download finished: success={}, path={:?}", success, path), None);
                    if success {
                        if let Some(ref path) = path {
                            let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                            tracing::info!("[download] File on disk: {} bytes at {:?}", file_size, path);

                            // Check if this is a text/CSV file or a binary spreadsheet
                            let path_str = path.to_string_lossy().to_lowercase();
                            let is_spreadsheet = path_str.ends_with(".xls") || path_str.ends_with(".xlsx");

                            if is_spreadsheet {
                                // Binary spreadsheet — emit as file path for the frontend to import
                                let month = dl_month.lock().unwrap().take();
                                tracing::info!("[download] Spreadsheet detected, month={:?}, path={:?}", month, path);
                                emit_log(&dl_log_handle, "success", "download",
                                    &format!("Spreadsheet downloaded: {} bytes, month={:?}", file_size, month), None);
                                let month_val = match month {
                                    Some(m) => serde_json::Value::String(m),
                                    None => serde_json::Value::Null,
                                };
                                let payload = serde_json::json!({
                                    "month": month_val,
                                    "filePath": path.to_string_lossy()
                                }).to_string();
                                tracing::info!("[download] Emitting carrier-commission-file ({} bytes payload)", payload.len());
                                match dl_handle.emit("carrier-commission-file", &payload) {
                                    Ok(_) => tracing::info!("[download] carrier-commission-file emitted OK"),
                                    Err(e) => tracing::error!("[download] Failed to emit: {}", e),
                                }
                                // Don't delete — the import command will read it
                            } else {
                                // Try text/CSV path (existing Humana behavior)
                                match std::fs::read_to_string(path) {
                                    Ok(content) => {
                                        let first_line = content.lines().next().unwrap_or("(empty)");
                                        tracing::info!("[download] Content: {} bytes, first line: {:?}", content.len(), &first_line[..first_line.len().min(120)]);
                                        if content.contains('|') && !content.trim_start().starts_with('<') {
                                            let month = dl_month.lock().unwrap().take();
                                            tracing::info!("[download] Valid CSV detected, month={:?}", month);
                                            emit_log(&dl_log_handle, "success", "download",
                                                &format!("Download complete: {} bytes, pipe-delimited CSV, month={:?}", content.len(), month), None);
                                            let month_val = match month {
                                                Some(m) => serde_json::Value::String(m),
                                                None => serde_json::Value::Null,
                                            };
                                            let payload = serde_json::json!([{
                                                "month": month_val,
                                                "csv": content
                                            }]).to_string();
                                            tracing::info!("[download] Emitting carrier-commission-data ({} bytes payload)", payload.len());
                                            match dl_handle.emit("carrier-commission-data", &payload) {
                                                Ok(_) => tracing::info!("[download] carrier-commission-data emitted OK"),
                                                Err(e) => tracing::error!("[download] Failed to emit: {}", e),
                                            }
                                        } else {
                                            tracing::warn!("[download] NOT CSV: contains_pipe={}, starts_with_html={}", content.contains('|'), content.trim_start().starts_with('<'));
                                            tracing::warn!("[download] First 200 chars: {:?}", &content[..content.len().min(200)]);
                                            emit_log(&dl_log_handle, "warn", "download", "Downloaded file is not CSV data, skipping", None);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("[download] Failed to read file: {}", e);
                                        emit_log(&dl_log_handle, "error", "download", &format!("Failed to read downloaded file: {}", e), None);
                                    }
                                }
                                let _ = std::fs::remove_file(path);
                            }
                        } else {
                            tracing::warn!("[download] Success but no path returned");
                        }
                    } else {
                        tracing::error!("[download] FAILED: url={}", url);
                        emit_log(&dl_log_handle, "error", "download",
                            &format!("Download failed: url={}", url), None);
                    }
                    true
                }
                _ => {
                    tracing::debug!("[download] Unknown event variant");
                    true
                }
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
/// After the local SQLite write succeeds, fires an async push to Convex if configured.
#[tauri::command]
pub async fn import_portal_members(
    carrier_id: String,
    members_json: String,
    state: State<'_, DbState>,
) -> Result<ImportPortalResult, String> {
    let members: Vec<PortalMember> =
        serde_json::from_str(&members_json).map_err(|e| format!("Failed to parse members: {}", e))?;

    let (result, convex_config, carrier_short_name) = state
        .with_conn(|conn| {
            let result = crate::services::carrier_sync_service::import_portal_members(
                conn,
                &carrier_id,
                &members,
            )?;

            let convex_config = crate::services::convex_service::ConvexConfig::from_settings(conn);

            let carrier_short_name: Option<String> = conn
                .query_row(
                    "SELECT short_name FROM carriers WHERE id = ?1",
                    rusqlite::params![carrier_id],
                    |row| row.get(0),
                )
                .ok()
                .flatten();

            Ok((result, convex_config, carrier_short_name))
        })
        .map_err(|e| e.to_string())?;

    // Fire-and-forget: push to Convex without blocking the local result.
    if let (Some(config), Some(short_name)) = (convex_config, carrier_short_name) {
        tokio::spawn(async move {
            if let Err(e) =
                crate::services::convex_service::push_carrier_sync(&config, &short_name, &members)
                    .await
            {
                tracing::warn!("Convex carrier-sync push failed: {}", e);
            } else {
                tracing::info!("Convex carrier-sync push succeeded for {}", short_name);
            }
        });
    }

    Ok(result)
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
