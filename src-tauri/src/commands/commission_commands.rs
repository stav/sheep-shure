use std::collections::HashMap;
use tauri::State;

use crate::db::DbState;
use crate::models::{
    CarrierMonthSummary, CommissionDeposit, CommissionDepositListItem, CommissionEntryListItem,
    CommissionFilters, CommissionRateListItem, CreateCommissionDepositInput,
    CreateCommissionRateInput, ReconciliationRow, StatementImportResult,
    UpdateCommissionDepositInput, UpdateCommissionEntryInput, UpdateCommissionRateInput,
};
use crate::services::{commission_service, import_service};

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
