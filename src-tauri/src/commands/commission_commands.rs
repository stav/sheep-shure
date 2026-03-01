use std::collections::HashMap;
use std::io::{Seek, Write};
use tauri::{AppHandle, Emitter, Listener, Manager, State};

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

    // Try to detect the actual statement month from a CommRunDt column in the CSV data.
    // The Humana CSV has a pipe-delimited CommRunDt column (e.g. "10/1/2025") which is more
    // reliable than the JS-scraped date from the portal page.
    let commission_month = {
        let mut detected = commission_month.clone();
        let delimiter = if csv_content.contains('|') { b'|' } else { b',' };
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .delimiter(delimiter)
            .flexible(true)
            .from_reader(csv_content.as_bytes());
        if let Ok(headers) = rdr.headers() {
            let comm_run_idx = headers.iter().position(|h| h.trim() == "CommRunDt");
            if let Some(idx) = comm_run_idx {
                if let Some(Ok(record)) = rdr.records().next() {
                    if let Some(date_str) = record.get(idx) {
                        let date_str = date_str.trim();
                        // Parse "M/D/YYYY" → "YYYY-MM"
                        let parts: Vec<&str> = date_str.split('/').collect();
                        if parts.len() == 3 {
                            if let (Ok(month_num), Ok(year)) = (parts[0].parse::<u32>(), parts[2].parse::<u32>()) {
                                let csv_month = format!("{:04}-{:02}", year, month_num);
                                if csv_month != detected {
                                    emit_log(&app, "info", "import",
                                        &format!("Month override: {} → {} (from CommRunDt: {})", detected, csv_month, date_str),
                                        None);
                                }
                                detected = csv_month;
                            }
                        }
                    }
                }
            }
        }
        detected
    };

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

/// Phase 1: Set the date range on the live DOM inputs and click Submit.
/// This causes a real page reload — the ASP.NET form submission happens
/// natively so the server's session state stays consistent.
///
/// Placeholders `{{FROM_DATE}}` and `{{THRU_DATE}}` are MM/DD/YYYY.
const COMMISSION_SUBMIT_TEMPLATE: &str = r#"
(function() {
    try {
        console.log('[Compass] ════════════════════════════════════════════');
        console.log('[Compass] Phase 1: Starting form submission');
        console.log('[Compass] Current URL: ' + window.location.href);
        console.log('[Compass] readyState: ' + document.readyState);

        if (window.location.pathname.indexOf('CompensationStatements') === -1) {
            throw new Error(
                'Not on the Compensation Statements page. ' +
                'Navigate to Compensation Statements in the portal, then click Fetch Statements again.'
            );
        }

        var FROM_DATE = '{{FROM_DATE}}';
        var THRU_DATE = '{{THRU_DATE}}';
        console.log('[Compass] Phase 1: Setting date range ' + FROM_DATE + ' – ' + THRU_DATE);

        var fromInput = document.querySelector('input[id*="FromDate"]');
        var thruInput = document.querySelector('input[id*="ThruDate"]');
        console.log('[Compass] Phase 1: fromInput=' + (fromInput ? fromInput.id : 'NOT FOUND') +
                     ', thruInput=' + (thruInput ? thruInput.id : 'NOT FOUND'));
        if (!fromInput || !thruInput) {
            throw new Error(
                'Cannot find date inputs on the Compensation Statements page. ' +
                'Make sure the "Prior Statement(s) by Statement Date" section is visible.'
            );
        }

        // Find the Submit button near the date inputs
        var submitBtn = null;
        for (var el = fromInput.closest('div[id]') || fromInput.parentElement;
             el && !submitBtn; el = el.parentElement) {
            submitBtn = el.querySelector('input[type="submit"], input[type="button"][value*="Submit"]');
        }
        if (!submitBtn) {
            var allInputs = document.querySelectorAll('input[type="submit"]');
            console.log('[Compass] Phase 1: No submit button near date inputs, searching all (' + allInputs.length + ' submit inputs)');
            for (var i = 0; i < allInputs.length; i++) {
                console.log('[Compass] Phase 1: submit input[' + i + ']: name=' + allInputs[i].name + ', value=' + allInputs[i].value);
                if (allInputs[i].value.toLowerCase().indexOf('submit') !== -1) {
                    submitBtn = allInputs[i];
                    break;
                }
            }
        }
        if (!submitBtn) {
            throw new Error('Cannot find the Submit button for the Prior Statements form.');
        }

        console.log('[Compass] Phase 1: Found submit button: name=' + submitBtn.name + ', value=' + submitBtn.value + ', id=' + submitBtn.id);
        console.log('[Compass] Phase 1: Setting fromInput.value = ' + FROM_DATE);
        fromInput.value = FROM_DATE;
        console.log('[Compass] Phase 1: Setting thruInput.value = ' + THRU_DATE);
        thruInput.value = THRU_DATE;
        console.log('[Compass] Phase 1: Clicking submit button NOW');
        submitBtn.click();
        console.log('[Compass] Phase 1: submitBtn.click() returned — page should be reloading');

    } catch (e) {
        console.error('[Compass] Phase 1 ERROR: ' + e.toString());
        window.location.href = 'http://compass-sync.localhost/error?message=' +
            encodeURIComponent('Commission fetch: ' + e.toString());
    }
})();
"#;

/// Phase 2: After the page reloads with results, find CSV links on the
/// live DOM and click them sequentially. Each click triggers a real
/// browser download captured by Tauri's on_download handler.
const COMMISSION_DOWNLOAD_SCRIPT: &str = r#"
(async function() {
    try {
        console.log('[Compass] ════════════════════════════════════════════');
        console.log('[Compass] Phase 2: Looking for CSV links on results page');
        console.log('[Compass] Current URL: ' + window.location.href);
        console.log('[Compass] readyState: ' + document.readyState);
        console.log('[Compass] document.title: ' + document.title);
        console.log('[Compass] body length: ' + (document.body ? document.body.innerHTML.length : 'no body'));

        // Wait for DOM to be ready (in case eval'd before page fully parsed)
        if (document.readyState === 'loading') {
            console.log('[Compass] Phase 2: Waiting for DOMContentLoaded...');
            await new Promise(function(r) { document.addEventListener('DOMContentLoaded', r); });
            console.log('[Compass] Phase 2: DOMContentLoaded fired');
        }

        // Count all links for context
        var allLinks = document.querySelectorAll('a');
        var allDoPostBack = document.querySelectorAll('a[href*="__doPostBack"]');
        console.log('[Compass] Phase 2: Total links on page: ' + allLinks.length + ', __doPostBack links: ' + allDoPostBack.length);

        // Log all __doPostBack link texts for debugging
        for (var d = 0; d < allDoPostBack.length; d++) {
            var text = (allDoPostBack[d].textContent || '').trim();
            var href = (allDoPostBack[d].getAttribute('href') || '').substring(0, 80);
            console.log('[Compass] Phase 2: doPostBack[' + d + ']: text="' + text + '", href="' + href + '..."');
        }

        // Poll for CSV links to appear (ASP.NET may render them late)
        var csvTargets = [];
        for (var attempt = 0; attempt < 20; attempt++) {
            var links = document.querySelectorAll('a[href*="__doPostBack"]');
            csvTargets = [];
            for (var i = 0; i < links.length; i++) {
                if ((links[i].textContent || '').trim() !== 'CSV') continue;
                var href = links[i].getAttribute('href') || '';
                var match = href.match(/__doPostBack\('([^']+)'/);
                if (!match) continue;
                var target = match[1];
                // Skip nested repeater detail rows and current statements
                // (current month also appears in prior results — avoid duplicate)
                if (target.indexOf('rpComm') !== -1) {
                    console.log('[Compass] Phase 2: Skipping rpComm target: ' + target);
                    continue;
                }
                if (target.indexOf('gvCurrentStatements') !== -1) {
                    console.log('[Compass] Phase 2: Skipping gvCurrentStatements target: ' + target);
                    continue;
                }

                var row = links[i].closest('tr');
                var rowText = row ? row.textContent : '';
                var dateMatch = rowText.match(/(\d{1,2})\/(\d{1,2})\/(\d{4})/);
                var month = null;
                if (dateMatch) {
                    month = dateMatch[3] + '-' + dateMatch[1].padStart(2, '0');
                }
                console.log('[Compass] Phase 2: CSV target: ' + target + ', month=' + month + ', rowDate=' + (dateMatch ? dateMatch[0] : '?'));
                csvTargets.push({ month: month, link: links[i], dateText: dateMatch ? dateMatch[0] : '?', target: target });
            }
            if (csvTargets.length > 0) break;
            console.log('[Compass] Phase 2: No CSV links yet, retrying... (' + (attempt + 1) + '/20)');
            await new Promise(function(r) { setTimeout(r, 500); });
        }

        console.log('[Compass] Phase 2: Found ' + csvTargets.length + ' CSV link(s) to download');
        if (csvTargets.length === 0) {
            throw new Error(
                'No CSV links found on the results page. ' +
                'The form submission may not have returned results.'
            );
        }

        // Click each CSV link sequentially
        for (var i = 0; i < csvTargets.length; i++) {
            var t = csvTargets[i];
            console.log('[Compass] Phase 2: ── CSV ' + (i + 1) + '/' + csvTargets.length + ' ──');
            console.log('[Compass] Phase 2: month=' + t.month + ', date=' + t.dateText + ', target=' + t.target);

            // Signal the month to the download handler
            console.log('[Compass] Phase 2: Signaling month to download handler...');
            window.location.href = 'http://compass-sync.localhost/commission-month?month=' +
                encodeURIComponent(t.month || '');
            await new Promise(function(r) { setTimeout(r, 300); });

            // Click the real link — triggers __doPostBack and browser download
            console.log('[Compass] Phase 2: Clicking link NOW');
            t.link.click();
            console.log('[Compass] Phase 2: link.click() returned, waiting 4s for download...');

            // Wait for download to complete before next
            await new Promise(function(r) { setTimeout(r, 4000); });
            console.log('[Compass] Phase 2: 4s wait done for CSV ' + (i + 1));
        }

        console.log('[Compass] Phase 2: ═══ All ' + csvTargets.length + ' CSV downloads triggered ═══');

    } catch (e) {
        console.error('[Compass] Phase 2 ERROR:', e);
        window.location.href = 'http://compass-sync.localhost/error?message=' +
            encodeURIComponent('Commission download: ' + e.toString());
    }
})();
"#;

fn commission_submit_script(from_date: &str, thru_date: &str) -> String {
    COMMISSION_SUBMIT_TEMPLATE
        .replace("{{FROM_DATE}}", from_date)
        .replace("{{THRU_DATE}}", thru_date)
}

/// JS snippet eval'd repeatedly from Rust to detect when the form POST
/// has completed and results are visible.  When CSV links are found it
/// navigates to `compass-sync.localhost/page-ready?csv=N` which the
/// on_navigation handler intercepts and turns into a Tauri event.
const COMMISSION_POLL_SCRIPT: &str = r#"
(function() {
    try {
        var links = document.querySelectorAll('a[href*="__doPostBack"]');
        var csv = 0;
        for (var i = 0; i < links.length; i++) {
            if ((links[i].textContent || '').trim() === 'CSV') csv++;
        }
        console.log('[Compass] Poll: readyState=' + document.readyState +
                     ', CSV links=' + csv +
                     ', url=' + window.location.href);
        if (csv > 0) {
            window.location.href = 'http://compass-sync.localhost/page-ready?csv=' + csv;
        }
    } catch(e) {
        console.log('[Compass] Poll error: ' + e);
    }
})();
"#;

/// Two-phase commission fetch:
/// 1. Eval Phase 1 script (set dates, submit form — causes page reload)
/// 2. Poll for results via repeated eval() until CSV links appear
/// 3. Eval Phase 2 script (find CSV links, click them sequentially)
///
/// We poll instead of using on_page_load because WebKitGTK does not fire
/// page-load events for ASP.NET form POST submissions to the same URL.
#[tauri::command]
pub async fn trigger_commission_fetch(
    app: AppHandle,
    from_date: String,
    thru_date: String,
) -> Result<(), String> {
    tracing::info!("[commission-fetch] ══════════════════════════════════════════");
    tracing::info!("[commission-fetch] Starting: from={}, thru={}", from_date, thru_date);
    emit_log(&app, "info", "fetch", &format!("Starting commission fetch: {} – {}", from_date, thru_date), None);

    let webview = app
        .get_webview_window("carrier-login")
        .ok_or("Carrier login window is not open. Open the Humana portal and log in first.")?;

    let current_url = webview.url().map(|u| u.to_string()).unwrap_or_else(|_| "?".into());
    tracing::info!("[commission-fetch] Webview found, current url={}", current_url);

    // ── Phase 1: submit the date range form ─────────────────────────────
    let submit_script = commission_submit_script(&from_date, &thru_date);
    tracing::info!("[commission-fetch] Phase 1: Evaluating submit script ({} bytes)", submit_script.len());
    emit_log(&app, "info", "fetch", "Phase 1: Submitting date range form", None);

    // Set up a one-shot listener BEFORE submitting so we don't miss the event.
    // on_page_load may fire (for full navigations) or on_navigation may fire
    // (from our poll script finding CSV links).  Either resolves the channel.
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));

    // Listen for BOTH possible signals — whichever fires first wins
    let tx_page = tx.clone();
    let page_lid = app.once("carrier-page-loaded", move |_| {
        tracing::info!("[commission-fetch] carrier-page-loaded received (from on_page_load)");
        if let Some(tx) = tx_page.lock().unwrap().take() {
            let _ = tx.send(());
        }
    });
    let tx_ready = tx.clone();
    let ready_lid = app.once("carrier-page-ready", move |event| {
        tracing::info!("[commission-fetch] carrier-page-ready received (from poll): {:?}", event.payload());
        if let Some(tx) = tx_ready.lock().unwrap().take() {
            let _ = tx.send(());
        }
    });
    tracing::info!("[commission-fetch] Listeners registered: page_lid={:?}, ready_lid={:?}", page_lid, ready_lid);

    webview.eval(&submit_script).map_err(|e| {
        tracing::error!("[commission-fetch] Phase 1 eval FAILED: {}", e);
        e.to_string()
    })?;
    tracing::info!("[commission-fetch] Phase 1: eval() returned OK — form should be submitting");
    emit_log(&app, "info", "fetch", "Form submitted, waiting for results page...", None);

    // ── Poll for results ────────────────────────────────────────────────
    // WebKitGTK doesn't fire on_page_load for ASP.NET form POSTs to the
    // same URL, so we actively poll by eval'ing a check script every 500ms.
    // The script signals back via compass-sync.localhost/page-ready when
    // it finds CSV links.
    let poll_webview = app
        .get_webview_window("carrier-login")
        .ok_or("Webview closed during fetch")?;

    tracing::info!("[commission-fetch] Starting poll loop (500ms intervals, 30s timeout)");
    let poll_app = app.clone();
    let poll_task = tokio::spawn(async move {
        // Initial delay — give the form POST time to start
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        tracing::info!("[commission-fetch] Poll: initial 2s delay done, starting checks");

        for attempt in 1..=56 {  // ~28s of polling after 2s delay = 30s total
            match poll_webview.eval(COMMISSION_POLL_SCRIPT) {
                Ok(_) => {
                    tracing::debug!("[commission-fetch] Poll #{}: eval OK", attempt);
                }
                Err(e) => {
                    tracing::warn!("[commission-fetch] Poll #{}: eval failed (page transitioning?): {}", attempt, e);
                    emit_log(&poll_app, "debug", "fetch",
                        &format!("Poll #{}: page not ready yet ({})", attempt, e), None);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        tracing::warn!("[commission-fetch] Poll loop exhausted (56 attempts)");
    });

    // Wait for either on_page_load or poll to signal (30s timeout)
    let start = std::time::Instant::now();
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), rx).await;
    let elapsed = start.elapsed();
    poll_task.abort();  // stop polling regardless

    // Clean up whichever listener didn't fire
    app.unlisten(page_lid);
    app.unlisten(ready_lid);

    match result {
        Ok(Ok(())) => {
            tracing::info!("[commission-fetch] ✓ Page ready after {:.1}s", elapsed.as_secs_f64());
            emit_log(&app, "success", "fetch",
                &format!("Results page ready after {:.1}s", elapsed.as_secs_f64()), None);
        }
        Ok(Err(_)) => {
            tracing::error!("[commission-fetch] ✗ Channel dropped after {:.1}s", elapsed.as_secs_f64());
            return Err("Page ready signal lost (channel dropped).".to_string());
        }
        Err(_) => {
            tracing::error!("[commission-fetch] ✗ TIMEOUT after {:.1}s", elapsed.as_secs_f64());
            emit_log(&app, "error", "fetch",
                "Timed out after 30s waiting for results page", None);
            return Err(
                "Timed out waiting for results page after form submission.".to_string(),
            );
        }
    }

    // ── Phase 2: click CSV links on the results page ────────────────────
    tracing::info!("[commission-fetch] Phase 2: Evaluating download script");
    emit_log(&app, "info", "fetch", "Phase 2: Downloading CSV files from results page", None);
    webview
        .eval(COMMISSION_DOWNLOAD_SCRIPT)
        .map_err(|e| {
            tracing::error!("[commission-fetch] Phase 2 eval FAILED: {}", e);
            e.to_string()
        })?;
    tracing::info!("[commission-fetch] Phase 2: eval() OK — CSV downloads proceeding asynchronously");

    Ok(())
}
