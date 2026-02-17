use serde::Serialize;
use tauri::State;

use crate::db::DbState;
use crate::AppDataDir;

#[derive(Serialize)]
pub struct DatabaseInfo {
    pub db_path: String,
    pub db_size_bytes: u64,
    pub client_count: i64,
    pub enrollment_count: i64,
    pub last_backup: Option<String>,
}

#[tauri::command]
pub fn get_database_info(
    db_state: State<'_, DbState>,
    app_data_dir: State<'_, AppDataDir>,
) -> Result<DatabaseInfo, String> {
    let db_path = app_data_dir.0.join("sheeps.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let db_size_bytes = std::fs::metadata(&db_path)
        .map(|m| m.len())
        .unwrap_or(0);

    db_state
        .with_conn(|conn| {
            let client_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM clients WHERE is_active = 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let enrollment_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM enrollments WHERE is_active = 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let last_backup: Option<String> = conn
                .query_row(
                    "SELECT value FROM app_settings WHERE key = 'last_backup_at'",
                    [],
                    |row| row.get(0),
                )
                .ok();

            Ok(DatabaseInfo {
                db_path: db_path_str,
                db_size_bytes,
                client_count,
                enrollment_count,
                last_backup,
            })
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<'_, DbState>) -> Result<serde_json::Value, String> {
    state
        .with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT key, value FROM app_settings")
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                })
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let mut settings = serde_json::Map::new();
            for row in rows {
                let (key, value) =
                    row.map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                settings.insert(
                    key,
                    serde_json::Value::String(value.unwrap_or_default()),
                );
            }
            Ok(serde_json::Value::Object(settings))
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings(
    settings: serde_json::Value,
    state: State<'_, DbState>,
) -> Result<(), String> {
    state
        .with_conn(|conn| {
            if let Some(obj) = settings.as_object() {
                for (key, value) in obj {
                    let val_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Null => String::new(),
                        other => other.to_string(),
                    };
                    conn.execute(
                        "INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
                        rusqlite::params![key, val_str],
                    )
                    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                }
            }
            Ok(())
        })
        .map_err(|e| e.to_string())
}

/// Get agent profile
#[tauri::command]
pub fn get_agent_profile(state: State<'_, DbState>) -> Result<serde_json::Value, String> {
    state
        .with_conn(|conn| {
            let result = conn.query_row(
                "SELECT id, first_name, last_name, email, phone, npn, agency_name, license_state FROM agent_profile LIMIT 1",
                [],
                |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, Option<String>>(0)?,
                        "first_name": row.get::<_, Option<String>>(1)?,
                        "last_name": row.get::<_, Option<String>>(2)?,
                        "email": row.get::<_, Option<String>>(3)?,
                        "phone": row.get::<_, Option<String>>(4)?,
                        "npn": row.get::<_, Option<String>>(5)?,
                        "agency_name": row.get::<_, Option<String>>(6)?,
                        "license_state": row.get::<_, Option<String>>(7)?,
                    }))
                },
            );
            match result {
                Ok(profile) => Ok(profile),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(serde_json::json!(null)),
                Err(e) => Err(crate::error::AppError::Database(e.to_string())),
            }
        })
        .map_err(|e| e.to_string())
}

/// Save or update agent profile
#[tauri::command]
pub fn save_agent_profile(
    profile: serde_json::Value,
    state: State<'_, DbState>,
) -> Result<(), String> {
    state
        .with_conn(|conn| {
            let id = profile
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let first_name = profile
                .get("first_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let last_name = profile
                .get("last_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let email = profile
                .get("email")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let phone = profile
                .get("phone")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let npn = profile
                .get("npn")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let agency_name = profile
                .get("agency_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let license_state = profile
                .get("license_state")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if id.is_empty() {
                let new_id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO agent_profile (id, first_name, last_name, email, phone, npn, agency_name, license_state) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![new_id, first_name, last_name, email, phone, npn, agency_name, license_state],
                )
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            } else {
                conn.execute(
                    "UPDATE agent_profile SET first_name = ?2, last_name = ?3, email = ?4, phone = ?5, npn = ?6, agency_name = ?7, license_state = ?8 WHERE id = ?1",
                    rusqlite::params![id, first_name, last_name, email, phone, npn, agency_name, license_state],
                )
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            }
            Ok(())
        })
        .map_err(|e| e.to_string())
}

/// Backup database to a user-selected location
#[tauri::command]
pub fn backup_database(
    destination: String,
    app_data_dir: State<'_, AppDataDir>,
    db_state: State<'_, DbState>,
) -> Result<(), String> {
    let db_path = app_data_dir.0.join("sheeps.db");
    std::fs::copy(&db_path, &destination).map_err(|e| format!("Backup failed: {}", e))?;

    // Record the backup timestamp
    db_state
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO app_settings (key, value) VALUES ('last_backup_at', datetime('now')) ON CONFLICT(key) DO UPDATE SET value = datetime('now'), updated_at = datetime('now')",
                [],
            )
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            Ok(())
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}
