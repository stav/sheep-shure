use std::collections::HashMap;
use std::io::{Seek, Write};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::DbState;
use crate::models::{
    CarrierMonthSummary, CommissionDeposit, CommissionDepositListItem, CommissionEntryListItem,
    CommissionFilters, CommissionRateListItem, CreateCommissionDepositInput,
    CreateCommissionRateInput, ImportLogEntry, ReconciliationRow, StatementImportResult,
    UpdateCommissionDepositInput, UpdateCommissionEntryInput, UpdateCommissionRateInput,
};
use crate::services::{commission_service, import_service};

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

// ============================================================================
// Commission Rates
// ============================================================================

#[tauri::command]
pub fn get_commission_rates(
    carrier_id: Option<String>,
    plan_year: Option<i32>,
    state: State<'_, DbState>,
) -> Result<Vec<CommissionRateListItem>, String> {
    state
        .with_conn(|conn| {
            commission_service::get_commission_rates(conn, carrier_id.as_deref(), plan_year)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_commission_rate(
    input: CreateCommissionRateInput,
    state: State<'_, DbState>,
) -> Result<CommissionRateListItem, String> {
    state
        .with_conn(|conn| commission_service::create_commission_rate(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_commission_rate(
    id: String,
    input: UpdateCommissionRateInput,
    state: State<'_, DbState>,
) -> Result<(), String> {
    state
        .with_conn(|conn| commission_service::update_commission_rate(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_commission_rate(id: String, state: State<'_, DbState>) -> Result<(), String> {
    state
        .with_conn(|conn| commission_service::delete_commission_rate(conn, &id))
        .map_err(|e| e.to_string())
}

// ============================================================================
// Commission Entries
// ============================================================================

#[tauri::command]
pub fn get_commission_entries(
    filters: CommissionFilters,
    state: State<'_, DbState>,
) -> Result<Vec<CommissionEntryListItem>, String> {
    state
        .with_conn(|conn| commission_service::get_commission_entries(conn, &filters))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_commission_batch(
    batch_id: String,
    state: State<'_, DbState>,
) -> Result<usize, String> {
    state
        .with_conn(|conn| commission_service::delete_commission_batch(conn, &batch_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_commission_entry(
    id: String,
    input: UpdateCommissionEntryInput,
    state: State<'_, DbState>,
) -> Result<(), String> {
    state
        .with_conn(|conn| commission_service::update_commission_entry(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_commission_entry(id: String, state: State<'_, DbState>) -> Result<(), String> {
    state
        .with_conn(|conn| commission_service::delete_commission_entry(conn, &id))
        .map_err(|e| e.to_string())
}

// ============================================================================
// Reconciliation
// ============================================================================

#[tauri::command]
pub fn reconcile_commissions(
    carrier_id: Option<String>,
    month: Option<String>,
    state: State<'_, DbState>,
) -> Result<usize, String> {
    state
        .with_conn(|conn| {
            commission_service::reconcile_entries(conn, carrier_id.as_deref(), month.as_deref())
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_missing_commissions(
    carrier_id: String,
    month: String,
    state: State<'_, DbState>,
) -> Result<usize, String> {
    state
        .with_conn(|conn| commission_service::find_missing_clients(conn, &carrier_id, &month))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_reconciliation_entries(
    filters: CommissionFilters,
    state: State<'_, DbState>,
) -> Result<Vec<ReconciliationRow>, String> {
    state
        .with_conn(|conn| commission_service::get_reconciliation_entries(conn, &filters))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_commission_summary(
    month: Option<String>,
    state: State<'_, DbState>,
) -> Result<Vec<CarrierMonthSummary>, String> {
    state
        .with_conn(|conn| {
            commission_service::get_carrier_month_summaries(conn, month.as_deref())
        })
        .map_err(|e| e.to_string())
}

// ============================================================================
// Statement Import
// ============================================================================

#[tauri::command]
pub fn parse_commission_statement(
    file_path: String,
) -> Result<import_service::ParsedFile, String> {
    commission_service::parse_commission_statement(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_commission_statement(
    file_path: String,
    carrier_id: String,
    commission_month: String,
    column_mapping: HashMap<String, String>,
    state: State<'_, DbState>,
) -> Result<StatementImportResult, String> {
    state
        .with_conn(|conn| {
            commission_service::import_commission_statement(
                conn,
                &file_path,
                &carrier_id,
                &commission_month,
                &column_mapping,
                None,
            )
        })
        .map_err(|e| e.to_string())
}

/// Import commission CSV content received from a webview fetch.
/// Writes the CSV string to a temp file, then delegates to the existing
/// file-based import_commission_statement pipeline.
#[tauri::command]
pub fn import_commission_csv(
    app: AppHandle,
    carrier_id: String,
    commission_month: String,
    csv_content: String,
    state: State<'_, DbState>,
) -> Result<StatementImportResult, String> {
    // Log first 2 lines as preview
    let preview: String = csv_content.lines().take(2).collect::<Vec<_>>().join("\n");
    emit_log(&app, "info", "import",
        &format!("Received CSV for carrier={}, month={}, {} bytes", carrier_id, commission_month, csv_content.len()),
        Some(&preview));

    // Write CSV content to a temp file
    let mut tmp = tempfile::Builder::new()
        .suffix(".txt")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    tmp.write_all(csv_content.as_bytes())
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    tmp.flush().map_err(|e| format!("Failed to flush temp file: {}", e))?;
    tmp.seek(std::io::SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek temp file: {}", e))?;
    let tmp_path = tmp.path().to_string_lossy().to_string();

    let log = |entry: ImportLogEntry| {
        let _ = app.emit("commission-import-log", &entry);
    };

    state
        .with_conn(|conn| {
            commission_service::import_commission_statement(
                conn,
                &tmp_path,
                &carrier_id,
                &commission_month,
                &HashMap::new(),
                Some(&log),
            )
        })
        .map_err(|e| e.to_string())
}

// ============================================================================
// Commission Deposits
// ============================================================================

#[tauri::command]
pub fn get_commission_deposits(
    carrier_id: Option<String>,
    month: Option<String>,
    state: State<'_, DbState>,
) -> Result<Vec<CommissionDepositListItem>, String> {
    state
        .with_conn(|conn| {
            commission_service::get_commission_deposits(
                conn,
                carrier_id.as_deref(),
                month.as_deref(),
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_commission_deposit(
    input: CreateCommissionDepositInput,
    state: State<'_, DbState>,
) -> Result<CommissionDeposit, String> {
    state
        .with_conn(|conn| commission_service::create_commission_deposit(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_commission_deposit(
    id: String,
    input: UpdateCommissionDepositInput,
    state: State<'_, DbState>,
) -> Result<(), String> {
    state
        .with_conn(|conn| commission_service::update_commission_deposit(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_commission_deposit(id: String, state: State<'_, DbState>) -> Result<(), String> {
    state
        .with_conn(|conn| commission_service::delete_commission_deposit(conn, &id))
        .map_err(|e| e.to_string())
}

// ============================================================================
// Humana Commission Fetch (via webview)
// ============================================================================

/// JS script eval'd into the carrier-login webview when the user clicks
/// "Fetch Statements".  The user must already be on the Humana Compensation
/// Statements page (producer.humana.com).  The CSV links are ASP.NET
/// `__doPostBack(...)` links — we click them to trigger a real form
/// submission, and Tauri's `on_download` handler captures the file.
///
/// Because `__doPostBack` is a synchronous form submission (only one at a
/// time), we click only the FIRST CSV link found in the "Current Statement"
/// section.  The user can fetch prior statements by expanding that section
/// on the portal and clicking Fetch again.
const COMMISSION_FETCH_SCRIPT: &str = r#"
(function() {
    try {
        if (window.location.pathname.indexOf('CompensationStatements') === -1) {
            throw new Error(
                'Not on the Compensation Statements page. ' +
                'Navigate to Compensation Statements in the portal, then click Fetch Statements again.'
            );
        }

        // Find <a> links whose visible text is exactly "CSV"
        var links = document.querySelectorAll('a[href]');
        var csvLinks = [];
        for (var i = 0; i < links.length; i++) {
            if ((links[i].textContent || '').trim() === 'CSV') {
                csvLinks.push(links[i]);
            }
        }

        console.log('[Compass] Found ' + csvLinks.length + ' CSV link(s)');

        if (csvLinks.length === 0) {
            throw new Error('No CSV download links found on the Compensation Statements page.');
        }

        // Extract the statement date from the row context for month detection.
        // The table row typically contains: SAN | date (MM/DD/YYYY) | PDF Excel CSV
        var row = csvLinks[0].closest('tr');
        var rowText = row ? row.textContent : '';
        var dateMatch = rowText.match(/(\d{1,2})\/(\d{1,2})\/(\d{4})/);
        var month = null;
        if (dateMatch) {
            month = dateMatch[3] + '-' + dateMatch[1].padStart(2, '0');
            console.log('[Compass] Detected statement month: ' + month + ' from date ' + dateMatch[0]);
        } else {
            console.log('[Compass] Could not detect month from row: ' + rowText.substring(0, 200));
        }

        // Store the detected month so the frontend can use it
        // (sent via a callback before the download triggers)
        window.location.href = 'http://compass-sync.localhost/commission-month?month=' +
            encodeURIComponent(month || '');

        // Click the first CSV link after a short delay to let the callback process
        var link = csvLinks[0];
        setTimeout(function() {
            console.log('[Compass] Clicking CSV link');
            link.click();
        }, 100);

    } catch (e) {
        window.location.href = 'http://compass-sync.localhost/error?message=' +
            encodeURIComponent('Commission fetch: ' + e.toString());
    }
})();
"#;

/// Inject the commission fetch script into the carrier login webview.
/// The user must already be on the Compensation Statements page at
/// producer.humana.com — the script scrapes the live DOM for CSV links.
#[tauri::command]
pub async fn trigger_commission_fetch(app: AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview_window("carrier-login")
        .ok_or("Carrier login window is not open. Open the Humana portal and log in first.")?;

    webview
        .eval(COMMISSION_FETCH_SCRIPT)
        .map_err(|e| e.to_string())?;

    Ok(())
}
