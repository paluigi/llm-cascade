use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP {status}: {body}")]
    Http {
        status: u16,
        body: String,
        retry_after: Option<u64>,
    },

    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Response parse error: {0}")]
    Parse(String),

    #[error("API key not configured for provider '{0}'")]
    MissingApiKey(String),

    #[error("Provider error: {0}")]
    Other(String),
}

impl ProviderError {
    pub fn http_status(&self) -> Option<u16> {
        match self {
            ProviderError::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            ProviderError::Http { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(self, ProviderError::Http { status: 429, .. })
    }
}

#[derive(Debug, Error)]
#[error("All providers in cascade '{cascade_name}' failed: {message}\nFailed prompt persisted to: {}", .failed_prompt_path.display())]
pub struct CascadeError {
    pub cascade_name: String,
    pub message: String,
    pub failed_prompt_path: PathBuf,
}
