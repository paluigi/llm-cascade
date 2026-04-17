//! Configuration loading and types for `config.toml`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Top-level application configuration.
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// Named provider definitions (keyed by provider name).
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    /// Named cascade definitions (keyed by cascade name).
    #[serde(default)]
    pub cascades: HashMap<String, CascadeConfig>,
    /// SQLite database configuration.
    #[serde(default)]
    pub database: DatabaseConfig,
    /// Failed prompt persistence configuration.
    #[serde(default)]
    pub failure_persistence: FailureConfig,
}

/// Configuration for a single LLM provider endpoint.
#[derive(Debug, Deserialize, Clone)]
pub struct ProviderConfig {
    /// The provider protocol type.
    pub r#type: ProviderType,
    /// Keyring service name for API key lookup.
    #[serde(default)]
    pub api_key_service: Option<String>,
    /// Environment variable name for API key fallback.
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Override the default base URL for this provider.
    #[serde(default)]
    pub base_url: Option<String>,
}

/// Supported provider protocol types.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    /// OpenAI Chat Completions API (and compatible endpoints like Groq, Together, vLLM).
    Openai,
    /// Anthropic Messages API.
    Anthropic,
    /// Google Gemini generateContent API.
    Gemini,
    /// Ollama local inference server.
    Ollama,
}

/// A single entry in a cascade, referencing a provider and model.
#[derive(Debug, Deserialize, Clone)]
pub struct CascadeEntry {
    /// The provider name (must match a key in `[providers]`).
    pub provider: String,
    /// The model identifier to use with this provider.
    pub model: String,
}

/// An ordered list of provider/model entries to try in sequence.
#[derive(Debug, Deserialize)]
pub struct CascadeConfig {
    /// The cascade entries, tried in order until one succeeds.
    pub entries: Vec<CascadeEntry>,
}

/// SQLite database path configuration.
#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file. Tilde (`~`) is expanded.
    #[serde(default = "default_db_path")]
    pub path: String,
}

fn default_db_path() -> String {
    "~/.local/share/llm-cascade/db.sqlite".to_string()
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { path: default_db_path() }
    }
}

/// Failed prompt persistence directory configuration.
#[derive(Debug, Deserialize)]
pub struct FailureConfig {
    /// Directory where failed conversation `.json` files are saved. Tilde is expanded.
    #[serde(default = "default_failure_dir")]
    pub dir: String,
}

fn default_failure_dir() -> String {
    "~/.local/share/llm-cascade/failed_prompts".to_string()
}

impl Default for FailureConfig {
    fn default() -> Self {
        Self { dir: default_failure_dir() }
    }
}

/// Expands a leading `~/` to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs_home()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Reads and parses a TOML configuration file.
pub fn load_config(path: &Path) -> Result<AppConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file '{}': {}", path.display(), e))?;
    let config: AppConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse config file '{}': {}", path.display(), e))?;
    Ok(config)
}

/// Returns the default configuration path (`$XDG_CONFIG_HOME/llm-cascade/config.toml`).
pub fn default_config_path() -> PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| expand_tilde("~/.config"));
    config_dir.join("llm-cascade").join("config.toml")
}
