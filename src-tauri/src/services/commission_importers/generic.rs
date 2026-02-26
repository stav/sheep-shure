use std::collections::HashMap;

use crate::error::AppError;
use crate::services::import_service;

use super::ParsedCommissionRow;

/// Auto-map commission statement column headers to our target fields.
fn auto_map_commission_columns(headers: &[String]) -> HashMap<String, String> {
    let aliases: &[(&str, &[&str])] = &[
        (
            "member_name",
            &[
                "member name", "name", "member", "subscriber name", "subscriber",
                "insured name", "enrollee", "enrollee name",
            ],
        ),
        (
            "member_id",
            &[
                "member id", "member number", "subscriber id", "id", "mbi",
                "hicn", "medicare id",
            ],
        ),
        (
            "first_name",
            &["first name", "first", "fname", "member first name"],
        ),
        (
            "last_name",
            &["last name", "last", "lname", "member last name"],
        ),
        (
            "statement_amount",
            &[
                "amount", "commission", "commission amount", "owed", "amount owed",
                "statement amount", "gross commission", "total commission",
            ],
        ),
        (
            "paid_amount",
            &[
                "paid", "paid amount", "net amount", "net commission",
                "payment amount", "amount paid",
            ],
        ),
        (
            "plan_type",
            &[
                "plan type", "product type", "product", "plan",
                "line of business", "lob", "plan name",
            ],
        ),
    ];

    let mut mapping = HashMap::new();
    for header in headers {
        let normalized = header.trim().to_lowercase().replace(['_', '-'], " ");
        for (target, alias_list) in aliases {
            if alias_list.iter().any(|a| *a == normalized) {
                mapping.insert(header.clone(), target.to_string());
                break;
            }
        }
    }
    mapping
}

/// Parse a generic CSV/XLSX commission statement into ParsedCommissionRows.
pub fn parse(file_path: &str) -> Result<Vec<ParsedCommissionRow>, AppError> {
    let (headers, rows) = import_service::get_all_rows(file_path)?;
    let auto_map = auto_map_commission_columns(&headers);

    // Build column index map: target_field -> column_index
    let col_idx: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| auto_map.get(h).map(|t| (t.clone(), i)))
        .collect();

    let mut result = Vec::with_capacity(rows.len());

    for row in &rows {
        let get_field = |field: &str| -> Option<String> {
            col_idx
                .get(field)
                .and_then(|&idx| row.get(idx))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        // Extract member name — either from combined field or first+last
        let member_name = get_field("member_name").or_else(|| {
            let first = get_field("first_name").unwrap_or_default();
            let last = get_field("last_name").unwrap_or_default();
            if first.is_empty() && last.is_empty() {
                None
            } else {
                Some(format!("{} {}", first, last).trim().to_string())
            }
        });

        let member_id = get_field("member_id");

        let parse_amount = |field: &str| -> Option<f64> {
            get_field(field).and_then(|s| s.replace(['$', ',', ' '], "").parse::<f64>().ok())
        };

        let statement_amount = parse_amount("statement_amount");
        let paid_amount = parse_amount("paid_amount").or(statement_amount);
        let plan_type_code = get_field("plan_type");

        result.push(ParsedCommissionRow {
            member_name,
            member_id,
            member_id_is_mbi: true,
            statement_amount,
            paid_amount,
            plan_type_code,
            is_initial: None,
            effective_date: None,
            notes: None,
        });
    }

    Ok(result)
}
