use std::collections::HashMap;

use calamine::Reader;

use crate::error::AppError;

use super::ParsedCommissionRow;

/// Parse "MM/DD/YYYY" date format to "YYYY-MM-DD".
fn parse_date(date_str: &str) -> Option<String> {
    let parts: Vec<&str> = date_str.trim().split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    let year: u32 = parts[2].parse().ok()?;
    Some(format!("{:04}-{:02}-{:02}", year, month, day))
}

/// Map Devoted's Prior Plan Type to our internal codes.
fn map_plan_type(plan_type: &str) -> Option<String> {
    match plan_type.trim().to_uppercase().as_str() {
        "MAPD" => Some("MAPD".to_string()),
        "PDP" => Some("PDP".to_string()),
        "NONE" | "" => None,
        other => {
            tracing::warn!("Unknown Devoted plan type: '{}', passing through", other);
            Some(other.to_string())
        }
    }
}

/// Parse a Devoted Health XLSX commission statement.
///
/// Devoted files have two sheets:
/// - Sheet 1: Summary (credits/debits/balance)
/// - Sheet 2: Transaction detail with per-member rows
///
/// We target Sheet 2 and use column-name lookup for resilience.
pub fn parse(file_path: &str) -> Result<Vec<ParsedCommissionRow>, AppError> {
    let mut workbook = calamine::open_workbook_auto(file_path)
        .map_err(|e| AppError::Import(format!("Failed to open Devoted workbook: {}", e)))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = sheet_names
        .get(1)
        .ok_or_else(|| {
            AppError::Import(
                "Devoted workbook does not have a second sheet (transaction detail)".to_string(),
            )
        })?
        .clone();

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::Import(format!("Failed to read Devoted sheet: {}", e)))?;

    let mut rows_iter = range.rows();

    // First row is headers
    let headers: Vec<String> = rows_iter
        .next()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .ok_or_else(|| AppError::Import("Devoted sheet 2 has no header row".to_string()))?;

    // Build column index lookup
    let idx = |name: &str| -> Option<usize> { headers.iter().position(|h| h.trim() == name) };

    let first_name_idx = idx("Member First");
    let last_name_idx = idx("Member Last");
    let hicn_idx = idx("Member HICN");
    let total_payment_idx = idx("Total Payment");
    let plan_type_idx = idx("Prior Plan Type");
    let commission_type_idx = idx("Commission Type");
    let effective_date_idx = idx("Effective Date");

    let mut rows = Vec::new();

    for row in rows_iter {
        let cells: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();

        let get = |idx: Option<usize>| -> Option<String> {
            idx.and_then(|i| cells.get(i))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        // Format as "LAST, FIRST"
        let member_name = match (get(last_name_idx), get(first_name_idx)) {
            (Some(last), Some(first)) => Some(format!("{}, {}", last.to_uppercase(), first.to_uppercase())),
            (Some(last), None) => Some(last.to_uppercase()),
            (None, Some(first)) => Some(first.to_uppercase()),
            (None, None) => None,
        };

        let member_id = get(hicn_idx);

        let amount: Option<f64> = get(total_payment_idx)
            .and_then(|s| s.replace(['$', ',', ' '], "").parse::<f64>().ok());

        let plan_type_code = get(plan_type_idx).and_then(|p| map_plan_type(&p));

        // "Renewal" → false, "Initial" → true
        let is_initial = get(commission_type_idx).map(|v| {
            let lower = v.to_lowercase();
            if lower.contains("initial") {
                true
            } else {
                false // "Renewal" or anything else
            }
        });

        let effective_date = get(effective_date_idx).and_then(|d| parse_date(&d));

        // Capture all columns as raw key-value pairs
        let mut raw_fields = HashMap::new();
        for (i, header) in headers.iter().enumerate() {
            if let Some(val) = cells.get(i) {
                let val = val.trim();
                if !val.is_empty() {
                    raw_fields.insert(header.clone(), val.to_string());
                }
            }
        }

        // Skip rows where everything is empty (blank trailing rows)
        if member_name.is_none() && member_id.is_none() && amount.is_none() {
            continue;
        }

        rows.push(ParsedCommissionRow {
            member_name,
            member_id,
            member_id_is_mbi: true,
            statement_amount: amount,
            paid_amount: amount,
            plan_type_code,
            is_initial,
            effective_date,
            notes: None,
            raw_fields: Some(raw_fields),
        });
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        assert_eq!(parse_date("01/15/2025"), Some("2025-01-15".to_string()));
        assert_eq!(parse_date("12/1/2024"), Some("2024-12-01".to_string()));
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("invalid"), None);
    }

    #[test]
    fn test_map_plan_type() {
        assert_eq!(map_plan_type("MAPD"), Some("MAPD".to_string()));
        assert_eq!(map_plan_type("PDP"), Some("PDP".to_string()));
        assert_eq!(map_plan_type("NONE"), None);
        assert_eq!(map_plan_type(""), None);
        assert_eq!(map_plan_type("HMO"), Some("HMO".to_string()));
    }
}
