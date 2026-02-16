use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::Connection;

use crate::db::{migrations, seed};
use crate::error::AppError;

const ARGON2_T_COST: u32 = 3;
const ARGON2_M_COST: u32 = 65536; // 64 MB
const ARGON2_P_COST: u32 = 4;
const KEY_LENGTH: usize = 32;
const SALT_FILE: &str = "sheeps.salt";
const DB_FILE: &str = "sheeps.db";

/// Check if this is a first run (no salt file exists)
pub fn is_first_run(app_data_dir: &Path) -> bool {
    !salt_path(app_data_dir).exists()
}

/// Create a new account: generate salt, derive key, create encrypted DB
pub fn create_database(app_data_dir: &Path, password: &str) -> Result<Connection, AppError> {
    if !is_first_run(app_data_dir) {
        return Err(AppError::Auth(
            "Database already exists. Use login instead.".to_string(),
        ));
    }

    // Generate random salt
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);

    // Derive key
    let key = derive_key(password, &salt)?;

    // Save salt to file
    std::fs::write(salt_path(app_data_dir), &salt)
        .map_err(|e| AppError::Io(format!("Failed to write salt file: {}", e)))?;

    // Open DB with SQLCipher key
    let conn = open_encrypted_db(app_data_dir, &key)?;

    // Run migrations and seed data
    migrations::run_migrations(&conn)?;
    seed::seed_data(&conn)?;

    tracing::info!("New encrypted database created successfully");
    Ok(conn)
}

/// Unlock existing database with password
pub fn unlock_database(app_data_dir: &Path, password: &str) -> Result<Connection, AppError> {
    // Read salt
    let salt = std::fs::read(salt_path(app_data_dir))
        .map_err(|e| AppError::Auth(format!("Failed to read salt file: {}", e)))?;

    // Derive key
    let key = derive_key(password, &salt)?;

    // Try to open DB - if password is wrong, open_encrypted_db returns "Invalid password"
    let conn = open_encrypted_db(app_data_dir, &key)?;

    // Run any pending migrations (for upgrades)
    migrations::run_migrations(&conn)?;

    // Re-run seed data (INSERT OR IGNORE) so new carriers/statuses are added
    seed::seed_data(&conn)?;

    tracing::info!("Database unlocked successfully");
    Ok(conn)
}

/// Change the database password
pub fn change_password(
    conn: &Connection,
    app_data_dir: &Path,
    new_password: &str,
) -> Result<(), AppError> {
    // Generate new salt
    let mut new_salt = [0u8; 32];
    OsRng.fill_bytes(&mut new_salt);

    // Derive new key
    let new_key = derive_key(new_password, &new_salt)?;
    let hex_key = hex_encode(&new_key);

    // Rekey the database
    conn.execute_batch(&format!("PRAGMA rekey = \"x'{}'\";", hex_key))
        .map_err(|e| AppError::Database(format!("Failed to rekey database: {}", e)))?;

    // Save new salt
    std::fs::write(salt_path(app_data_dir), &new_salt)
        .map_err(|e| AppError::Io(format!("Failed to write new salt file: {}", e)))?;

    tracing::info!("Database password changed successfully");
    Ok(())
}

/// Derive a 32-byte key from password and salt using Argon2id
fn derive_key(password: &str, salt: &[u8]) -> Result<Vec<u8>, AppError> {
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(KEY_LENGTH))
        .map_err(|e| AppError::Auth(format!("Invalid Argon2 params: {}", e)))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = vec![0u8; KEY_LENGTH];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| AppError::Auth(format!("Key derivation failed: {}", e)))?;

    Ok(key)
}

/// Open a SQLCipher-encrypted database
fn open_encrypted_db(app_data_dir: &Path, key: &[u8]) -> Result<Connection, AppError> {
    let db_path = db_path(app_data_dir);
    let conn = Connection::open(&db_path)?;

    let hex_key = hex_encode(key);
    conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", hex_key))
        .map_err(|e| AppError::Database(format!("Failed to set encryption key: {}", e)))?;

    // Verify the key is correct before proceeding
    conn.execute_batch("SELECT count(*) FROM sqlite_master;")
        .map_err(|_| AppError::Auth("Invalid password".to_string()))?;

    // Enable WAL mode and foreign keys
    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|e| AppError::Database(format!("Failed to set WAL mode: {}", e)))?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|e| AppError::Database(format!("Failed to enable foreign keys: {}", e)))?;

    Ok(conn)
}

fn salt_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(SALT_FILE)
}

fn db_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(DB_FILE)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
