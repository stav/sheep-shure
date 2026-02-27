use serde::{Deserialize, Serialize};

// ============================================================================
// Commission Rates
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionRate {
    pub id: String,
    pub carrier_id: String,
    pub plan_type_code: String,
    pub plan_year: i32,
    pub initial_rate: f64,
    pub renewal_rate: f64,
    pub notes: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionRateListItem {
    pub id: String,
    pub carrier_id: String,
    pub carrier_name: String,
    pub plan_type_code: String,
    pub plan_year: i32,
    pub initial_rate: f64,
    pub renewal_rate: f64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommissionRateInput {
    pub carrier_id: String,
    pub plan_type_code: String,
    pub plan_year: i32,
    pub initial_rate: f64,
    pub renewal_rate: f64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCommissionRateInput {
    pub carrier_id: Option<String>,
    pub plan_type_code: Option<String>,
    pub plan_year: Option<i32>,
    pub initial_rate: Option<f64>,
    pub renewal_rate: Option<f64>,
    pub notes: Option<String>,
}

// ============================================================================
// Commission Entries
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionEntry {
    pub id: String,
    pub client_id: Option<String>,
    pub enrollment_id: Option<String>,
    pub carrier_id: String,
    pub plan_type_code: Option<String>,
    pub commission_month: String,
    pub statement_amount: Option<f64>,
    pub paid_amount: Option<f64>,
    pub member_name: Option<String>,
    pub member_id: Option<String>,
    pub is_initial: Option<i32>,
    pub expected_rate: Option<f64>,
    pub rate_difference: Option<f64>,
    pub status: Option<String>,
    pub import_batch_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionEntryListItem {
    pub id: String,
    pub client_id: Option<String>,
    pub client_name: Option<String>,
    pub carrier_name: String,
    pub plan_type_code: Option<String>,
    pub commission_month: String,
    pub statement_amount: Option<f64>,
    pub paid_amount: Option<f64>,
    pub member_name: Option<String>,
    pub is_initial: Option<i32>,
    pub expected_rate: Option<f64>,
    pub rate_difference: Option<f64>,
    pub status: Option<String>,
    pub effective_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCommissionEntryInput {
    pub member_name: Option<String>,
    pub plan_type_code: Option<String>,
    pub statement_amount: Option<f64>,
    pub paid_amount: Option<f64>,
    pub is_initial: Option<i32>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionFilters {
    pub carrier_id: Option<String>,
    pub commission_month: Option<String>,
    pub status: Option<String>,
    pub client_id: Option<String>,
    pub import_batch_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementImportResult {
    pub total: usize,
    pub matched: usize,
    pub unmatched: usize,
    pub skipped: usize,
    pub errors: usize,
    pub batch_id: String,
    pub unmatched_names: Vec<String>,
    pub error_messages: Vec<String>,
}

// ============================================================================
// Commission Deposits
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionDeposit {
    pub id: String,
    pub carrier_id: String,
    pub deposit_month: String,
    pub deposit_amount: f64,
    pub deposit_date: Option<String>,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionDepositListItem {
    pub id: String,
    pub carrier_id: String,
    pub carrier_name: String,
    pub deposit_month: String,
    pub deposit_amount: f64,
    pub deposit_date: Option<String>,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub statement_total: f64,
    pub difference: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommissionDepositInput {
    pub carrier_id: String,
    pub deposit_month: String,
    pub deposit_amount: f64,
    pub deposit_date: Option<String>,
    pub reference: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCommissionDepositInput {
    pub carrier_id: Option<String>,
    pub deposit_month: Option<String>,
    pub deposit_amount: Option<f64>,
    pub deposit_date: Option<String>,
    pub reference: Option<String>,
    pub notes: Option<String>,
}

// ============================================================================
// Reconciliation / Summary
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationRow {
    pub id: String,
    pub client_id: Option<String>,
    pub client_name: Option<String>,
    pub carrier_name: String,
    pub plan_type_code: Option<String>,
    pub commission_month: String,
    pub effective_date: Option<String>,
    pub is_initial: Option<i32>,
    pub expected_rate: Option<f64>,
    pub statement_amount: Option<f64>,
    pub paid_amount: Option<f64>,
    pub rate_difference: Option<f64>,
    pub status: Option<String>,
    pub member_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierMonthSummary {
    pub carrier_id: String,
    pub carrier_name: String,
    pub commission_month: String,
    pub total_expected: f64,
    pub total_statement: f64,
    pub total_paid: f64,
    pub deposit_amount: Option<f64>,
    pub deposit_vs_paid: Option<f64>,
    pub entry_count: i64,
    pub ok_count: i64,
    pub issue_count: i64,
}
