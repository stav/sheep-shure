mod shared;
mod file_import;
mod call_log;
mod integrity;
mod sirem;
mod leadsmaster;

// Re-export all public types and functions at the module level
// so existing `import_service::` paths continue to work.
pub use file_import::{
    parse_file, auto_map_columns, validate_rows, execute_import, get_all_rows,
    ParsedFile, ValidationResult, ErrorRow, ImportRowDetail, ImportResult,
};
pub use call_log::{import_call_log_from_db, ActivityImportResult};
pub use integrity::import_integrity_from_json;
pub use sirem::import_sirem_from_dump;
pub use leadsmaster::enrich_from_leadsmaster;
