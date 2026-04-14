use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub cascades: HashMap<String, CascadeConfig>,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub failure_persistence: FailureConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProviderConfig {
    pub r#type: ProviderType,
    #[serde(default)]
    pub api_key_service: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Openai,
    Anthropic,
    Gemini,
    Ollama,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CascadeEntry {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct CascadeConfig {
    pub entries: Vec<CascadeEntry>,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
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

#[derive(Debug, Deserialize)]
pub struct FailureConfig {
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

pub fn load_config(path: &Path) -> Result<AppConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file '{}': {}", path.display(), e))?;
    let config: AppConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse config file '{}': {}", path.display(), e))?;
    Ok(config)
}

pub fn default_config_path() -> PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| expand_tilde("~/.config"));
    config_dir.join("llm-cascade").join("config.toml")
}
