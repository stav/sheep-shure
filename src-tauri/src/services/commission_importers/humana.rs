use crate::error::AppError;

use super::ParsedCommissionRow;

/// Map Humana's ProdCode to our internal plan type codes.
fn map_product_code(prod_code: &str) -> String {
    match prod_code.trim() {
        "MEP" => "MAPD".to_string(),  // Medicare Choice PPO
        "MRO" => "MAPD".to_string(),  // Medicare Gold Plus POS
        other => {
            if !other.is_empty() {
                tracing::warn!("Unknown Humana product code: '{}', passing through", other);
            }
            other.to_string()
        }
    }
}

/// Parse Humana's "M/D/YYYY" date format to "YYYY-MM-DD".
fn parse_humana_date(date_str: &str) -> Option<String> {
    let parts: Vec<&str> = date_str.trim().split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    let year: u32 = parts[2].parse().ok()?;
    Some(format!("{:04}-{:02}-{:02}", year, month, day))
}

/// Reformat Humana's "LAST FIRST MIDDLE" into "LAST, FIRST MIDDLE"
/// so the existing `parse_member_name()` can handle it as "Last, First".
fn reformat_humana_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        _ => {
            // First token is last name, rest is first + middle
            let last = parts[0];
            let rest = parts[1..].join(" ");
            format!("{}, {}", last, rest)
        }
    }
}

/// Parse a Humana pipe-delimited commission statement.
pub fn parse(file_path: &str) -> Result<Vec<ParsedCommissionRow>, AppError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b'|')
        .flexible(true)
        .from_path(file_path)
        .map_err(|e| AppError::Import(format!("Failed to read Humana file: {}", e)))?;

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| AppError::Import(format!("Failed to read Humana headers: {}", e)))?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();

    // Build header index lookup
    let idx = |name: &str| -> Option<usize> {
        headers.iter().position(|h| h == name)
    };

    let grp_name_idx = idx("GrpName");
    let grp_nbr_idx = idx("GrpNbr");
    let paid_amount_idx = idx("PaidAmount");
    let prod_code_idx = idx("ProdCode");
    let frstyr_rnwl_idx = idx("FrstYrRnwl");
    let eff_date_idx = idx("EffDate");
    let comment_idx = idx("Comment");

    let mut rows = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| AppError::Import(format!("Humana parse error: {}", e)))?;

        let get = |idx: Option<usize>| -> Option<String> {
            idx.and_then(|i| record.get(i))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        let member_name = get(grp_name_idx).map(|n| reformat_humana_name(&n));
        let member_id = get(grp_nbr_idx);

        let paid_amount: Option<f64> = get(paid_amount_idx)
            .and_then(|s| s.replace(['$', ',', ' '], "").parse::<f64>().ok());

        let prod_code = get(prod_code_idx);
        let plan_type_code = prod_code.map(|c| map_product_code(&c));

        // FrstYrRnwl: "R" = renewal, "N" = initial (new)
        let is_initial = get(frstyr_rnwl_idx).map(|v| {
            match v.to_uppercase().as_str() {
                "N" => true,
                _ => false, // "R" or anything else = renewal
            }
        });

        let effective_date = get(eff_date_idx).and_then(|d| parse_humana_date(&d));
        let notes = get(comment_idx);

        rows.push(ParsedCommissionRow {
            member_name,
            member_id,
            member_id_is_mbi: false, // Humana's GrpNbr is NOT an MBI
            statement_amount: paid_amount, // Humana only has PaidAmount
            paid_amount,
            plan_type_code,
            is_initial,
            effective_date,
            notes,
        });
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reformat_humana_name() {
        assert_eq!(reformat_humana_name("RIETH JACQUELINE L"), "RIETH, JACQUELINE L");
        assert_eq!(reformat_humana_name("ORTIZ ADELINA"), "ORTIZ, ADELINA");
        assert_eq!(reformat_humana_name("SMITH"), "SMITH");
        assert_eq!(reformat_humana_name(""), "");
    }

    #[test]
    fn test_parse_humana_date() {
        assert_eq!(parse_humana_date("3/1/2025"), Some("2025-03-01".to_string()));
        assert_eq!(parse_humana_date("12/15/2024"), Some("2024-12-15".to_string()));
        assert_eq!(parse_humana_date("1/1/2025"), Some("2025-01-01".to_string()));
        assert_eq!(parse_humana_date(""), None);
        assert_eq!(parse_humana_date("invalid"), None);
    }

    #[test]
    fn test_map_product_code() {
        assert_eq!(map_product_code("MEP"), "MAPD");
        assert_eq!(map_product_code("MRO"), "MAPD");
        assert_eq!(map_product_code("XYZ"), "XYZ");
        assert_eq!(map_product_code(""), "");
    }
}
