mod generic;
mod humana;

use std::collections::HashMap;

use crate::error::AppError;

/// Common intermediate type that all carrier-specific parsers produce.
/// Each importer maps carrier-specific columns into these unified fields.
#[derive(Debug, Clone)]
pub struct ParsedCommissionRow {
    pub member_name: Option<String>,
    pub member_id: Option<String>,
    /// If true, member_id may be an MBI and should be matched against clients.mbi.
    /// If false (e.g. Humana's GrpNbr), skip MBI matching.
    pub member_id_is_mbi: bool,
    pub statement_amount: Option<f64>,
    pub paid_amount: Option<f64>,
    /// Already mapped to our plan type codes (MAPD, PDP, etc.)
    pub plan_type_code: Option<String>,
    /// Some carriers (Humana) tell us directly whether this is initial or renewal.
    /// None means the reconciliation step will determine it from enrollment dates.
    pub is_initial: Option<bool>,
    pub effective_date: Option<String>,
    pub notes: Option<String>,
    /// All original columns from the carrier file, preserved as key-value pairs.
    pub raw_fields: Option<HashMap<String, String>>,
}

/// Dispatch to the correct carrier-specific parser based on `carrier_short_name`.
pub fn parse_statement_rows(
    file_path: &str,
    carrier_short_name: &str,
) -> Result<Vec<ParsedCommissionRow>, AppError> {
    match carrier_short_name.to_lowercase().as_str() {
        "humana" => humana::parse(file_path),
        _ => generic::parse(file_path),
    }
}
