use std::collections::HashMap;
use tauri::State;

use crate::db::DbState;
use crate::services::import_service;

#[tauri::command]
pub fn parse_import_file(file_path: String) -> Result<serde_json::Value, String> {
    let parsed = import_service::parse_file(&file_path).map_err(|e| e.to_string())?;
    let mapping = import_service::auto_map_columns(&parsed.headers);

    serde_json::to_value(serde_json::json!({
        "headers": parsed.headers,
        "sample_rows": parsed.sample_rows,
        "total_rows": parsed.total_rows,
        "auto_mapping": mapping,
    }))
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_import(
    file_path: String,
    column_mapping: HashMap<String, String>,
) -> Result<serde_json::Value, String> {
    let (headers, all_rows) =
        import_service::get_all_rows(&file_path).map_err(|e| e.to_string())?;

    let result = import_service::validate_rows(&all_rows, &headers, &column_mapping);

    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn execute_import(
    file_path: String,
    column_mapping: HashMap<String, String>,
    constant_values: Option<HashMap<String, String>>,
    state: State<'_, DbState>,
) -> Result<serde_json::Value, String> {
    let constant_values = constant_values.unwrap_or_default();
    let (headers, all_rows) =
        import_service::get_all_rows(&file_path).map_err(|e| e.to_string())?;

    // Only import valid rows
    let validation = import_service::validate_rows(&all_rows, &headers, &column_mapping);

    state
        .with_conn(|conn| {
            let result = import_service::execute_import(
                conn,
                &validation.valid_rows,
                &headers,
                &column_mapping,
                &constant_values,
            )?;

            // Log the import
            let log_id = uuid::Uuid::new_v4().to_string();
            let filename = std::path::Path::new(&file_path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| file_path.clone());
            let file_type = if file_path.to_lowercase().ends_with(".csv") {
                "CSV"
            } else {
                "XLSX"
            };

            conn.execute(
                "INSERT INTO import_logs (id, filename, file_type, total_rows, inserted_rows, updated_rows, skipped_rows, error_rows, column_mapping, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'COMPLETED')",
                rusqlite::params![
                    log_id,
                    filename,
                    file_type,
                    result.total,
                    result.inserted,
                    result.updated,
                    result.skipped,
                    result.errors,
                    serde_json::to_string(&column_mapping).unwrap_or_default()
                ],
            )?;

            // Combine execution error details with validation error details
            let mut all_error_details = result.error_details;
            for err_row in &validation.error_rows {
                all_error_details.push(import_service::ImportRowDetail {
                    label: format!("Row {}", err_row.row_number),
                    detail: err_row.errors.join("; "),
                });
            }

            serde_json::to_value(serde_json::json!({
                "inserted": result.inserted,
                "updated": result.updated,
                "skipped": result.skipped,
                "errors": result.errors + validation.error_rows.len(),
                "total": result.total + validation.error_rows.len(),
                "inserted_details": result.inserted_details,
                "updated_details": result.updated_details,
                "skipped_details": result.skipped_details,
                "errors_details": all_error_details,
            }))
            .map_err(|e| crate::error::AppError::Import(e.to_string()))
        })
        .map_err(|e| e.to_string())
}
