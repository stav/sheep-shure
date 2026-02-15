use rusqlite::Connection;
use crate::error::AppError;
use crate::models::report::ReportDefinition;

/// Execute a report query and return results as JSON
pub fn run_report(conn: &Connection, definition: &ReportDefinition) -> Result<serde_json::Value, AppError> {
    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    let filters = &definition.filters;

    if let Some(ref search) = filters.search {
        if !search.is_empty() {
            conditions.push(format!(
                "c.rowid IN (SELECT rowid FROM clients_fts WHERE clients_fts MATCH ?{})",
                idx
            ));
            params.push(Box::new(format!("{}*", search.replace('"', ""))));
            idx += 1;
        }
    }

    if let Some(ref state) = filters.state {
        conditions.push(format!("c.state = ?{}", idx));
        params.push(Box::new(state.clone()));
        idx += 1;
    }

    if let Some(ref zip) = filters.zip {
        conditions.push(format!("c.zip = ?{}", idx));
        params.push(Box::new(zip.clone()));
        idx += 1;
    }

    if let Some(is_dual) = filters.is_dual_eligible {
        conditions.push(format!("c.is_dual_eligible = ?{}", idx));
        params.push(Box::new(if is_dual { 1i32 } else { 0i32 }));
        idx += 1;
    }

    if let Some(is_active) = filters.is_active {
        conditions.push(format!("c.is_active = ?{}", idx));
        params.push(Box::new(if is_active { 1i32 } else { 0i32 }));
        idx += 1;
    } else {
        conditions.push("c.is_active = 1".to_string());
    }

    if let Some(ref carrier_id) = filters.carrier_id {
        conditions.push(format!(
            "c.id IN (SELECT DISTINCT client_id FROM enrollments WHERE carrier_id = ?{} AND is_active = 1)",
            idx
        ));
        params.push(Box::new(carrier_id.clone()));
        idx += 1;
    }

    if let Some(ref plan_type_code) = filters.plan_type_code {
        conditions.push(format!(
            "c.id IN (SELECT DISTINCT client_id FROM enrollments WHERE plan_type_code = ?{} AND is_active = 1)",
            idx
        ));
        params.push(Box::new(plan_type_code.clone()));
        idx += 1;
    }

    if let Some(ref status_code) = filters.status_code {
        conditions.push(format!(
            "c.id IN (SELECT DISTINCT client_id FROM enrollments WHERE status_code = ?{} AND is_active = 1)",
            idx
        ));
        params.push(Box::new(status_code.clone()));
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Build column list from definition, defaulting to common fields
    let columns = if definition.columns.is_empty() {
        "c.id, c.first_name, c.last_name, c.dob, c.phone, c.email, c.city, c.state, c.zip, c.mbi, c.is_dual_eligible".to_string()
    } else {
        definition
            .columns
            .iter()
            .map(|col| format!("c.{}", col))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let sort = if let Some(ref sort_by) = definition.sort_by {
        let dir = definition.sort_dir.as_deref().unwrap_or("ASC");
        format!("ORDER BY c.{} {}", sort_by, dir)
    } else {
        "ORDER BY c.last_name, c.first_name".to_string()
    };

    let sql = format!("SELECT {} FROM clients c {} {}", columns, where_clause, sort);
    let _ = idx; // suppress unused warning

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;

    let column_count = stmt.column_count();
    let column_names: Vec<String> = (0..column_count)
        .map(|i| stmt.column_name(i).unwrap_or("").to_string())
        .collect();

    let rows: Vec<serde_json::Value> = stmt
        .query_map(params_refs.as_slice(), |row| {
            let mut obj = serde_json::Map::new();
            for (i, name) in column_names.iter().enumerate() {
                let val: Option<String> = row.get(i)?;
                obj.insert(
                    name.clone(),
                    serde_json::Value::String(val.unwrap_or_default()),
                );
            }
            Ok(serde_json::Value::Object(obj))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(serde_json::json!({
        "columns": column_names,
        "data": rows,
        "total": rows.len(),
        "report_name": definition.name,
    }))
}

/// Generate a PDF report and return the path to the generated file
pub fn generate_pdf(
    conn: &Connection,
    definition: &ReportDefinition,
    output_dir: &std::path::Path,
) -> Result<String, AppError> {
    let report_data = run_report(conn, definition)?;
    let data = report_data
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| AppError::Import("No report data".to_string()))?;
    let columns = report_data
        .get("columns")
        .and_then(|c| c.as_array())
        .ok_or_else(|| AppError::Import("No columns".to_string()))?;

    // Try multiple common font paths
    let font_family = genpdf::fonts::from_files("/usr/share/fonts/TTF/", "DejaVuSans", None)
        .or_else(|_| {
            genpdf::fonts::from_files(
                "/usr/share/fonts/truetype/dejavu/",
                "DejaVuSans",
                None,
            )
        })
        .or_else(|_| {
            genpdf::fonts::from_files("/usr/share/fonts/", "DejaVuSans", None)
        })
        .or_else(|_| genpdf::fonts::from_files("", "LiberationSans", None))
        .map_err(|e| {
            AppError::Import(format!(
                "Could not find any fonts for PDF generation: {}",
                e
            ))
        })?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(&definition.name);
    doc.set_minimal_conformance();

    // Add title
    let mut title = genpdf::elements::Paragraph::new(&definition.name);
    title.set_alignment(genpdf::Alignment::Center);
    doc.push(title);
    doc.push(genpdf::elements::Break::new(1));

    // Add summary
    doc.push(genpdf::elements::Paragraph::new(format!(
        "Total records: {}",
        data.len()
    )));
    doc.push(genpdf::elements::Break::new(1));

    // Add table (limit columns for PDF readability)
    let col_count = columns.len().min(6);
    let mut table = genpdf::elements::TableLayout::new(vec![1; col_count]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Header row
    let mut header_row = table.row();
    for col in columns.iter().take(col_count) {
        let col_name = col.as_str().unwrap_or("");
        header_row.push_element(genpdf::elements::Paragraph::new(col_name));
    }
    header_row
        .push()
        .map_err(|_| AppError::Import("PDF table error".to_string()))?;

    // Data rows (limit to 500 rows for PDF)
    for row_val in data.iter().take(500) {
        let mut row = table.row();
        for col in columns.iter().take(col_count) {
            let col_name = col.as_str().unwrap_or("");
            let cell_val = row_val
                .get(col_name)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            row.push_element(genpdf::elements::Paragraph::new(cell_val));
        }
        row.push()
            .map_err(|_| AppError::Import("PDF table error".to_string()))?;
    }

    doc.push(table);

    // Write to file
    let filename = format!(
        "{}.pdf",
        definition.name.replace(' ', "_").to_lowercase()
    );
    let path = output_dir.join(&filename);
    doc.render_to_file(&path)
        .map_err(|e| AppError::Import(format!("Failed to generate PDF: {}", e)))?;

    Ok(path.to_string_lossy().to_string())
}
