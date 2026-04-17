//! SQLite-backed persistence for attempt logs and cooldown state.

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::config::expand_tilde;

/// Opens (or creates) the SQLite database and ensures the schema exists.
///
/// Expands `~` in the path and creates parent directories if needed.
pub fn init_db(path: &str) -> Result<Connection, String> {
    let expanded = expand_tilde(path);
    if let Some(parent) = expanded.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create database directory '{}': {}",
                parent.display(),
                e
            )
        })?;
    }

    let conn = Connection::open(&expanded)
        .map_err(|e| format!("Failed to open database '{}': {}", expanded.display(), e))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS attempt_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            cascade_name TEXT NOT NULL,
            provider_model TEXT NOT NULL,
            http_status INTEGER,
            latency_ms INTEGER NOT NULL,
            input_tokens INTEGER,
            output_tokens INTEGER
        );

        CREATE TABLE IF NOT EXISTS cooldown (
            provider_model TEXT PRIMARY KEY,
            cooldown_until TEXT NOT NULL
        );",
    )
    .map_err(|e| format!("Failed to initialize database schema: {}", e))?;

    Ok(conn)
}

/// Inserts a row into the `attempt_log` table.
pub fn log_attempt(
    conn: &Connection,
    cascade_name: &str,
    provider_model: &str,
    http_status: Option<u16>,
    latency_ms: u64,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
) {
    let timestamp = Utc::now().to_rfc3339();
    let result = conn.execute(
        "INSERT INTO attempt_log (timestamp, cascade_name, provider_model, http_status, latency_ms, input_tokens, output_tokens)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            timestamp,
            cascade_name,
            provider_model,
            http_status.map(|s| s as i64),
            latency_ms as i64,
            input_tokens.map(|t| t as i64),
            output_tokens.map(|t| t as i64),
        ],
    );

    if let Err(e) = result {
        tracing::warn!("Failed to log attempt to database: {}", e);
    }
}

/// Returns `true` if the given provider/model entry is currently on cooldown.
pub fn is_on_cooldown(conn: &Connection, provider_model: &str) -> bool {
    let now = Utc::now().to_rfc3339();
    let result = conn.query_row(
        "SELECT cooldown_until FROM cooldown WHERE provider_model = ?1 AND cooldown_until > ?2",
        params![provider_model, now],
        |row| row.get::<_, String>(0),
    );

    match result {
        Ok(_) => true,
        Err(rusqlite::Error::QueryReturnedNoRows) => false,
        Err(e) => {
            tracing::warn!("Failed to check cooldown for '{}': {}", provider_model, e);
            false
        }
    }
}

/// Sets or updates the cooldown for a provider/model entry until the given RFC 3339 timestamp.
pub fn set_cooldown(conn: &Connection, provider_model: &str, cooldown_until: &str) {
    let result = conn.execute(
        "INSERT INTO cooldown (provider_model, cooldown_until) VALUES (?1, ?2)
         ON CONFLICT(provider_model) DO UPDATE SET cooldown_until = ?2",
        params![provider_model, cooldown_until],
    );

    if let Err(e) = result {
        tracing::warn!("Failed to set cooldown for '{}': {}", provider_model, e);
    }
}
