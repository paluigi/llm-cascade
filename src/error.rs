//! Error types for provider and cascade operations.

use std::path::PathBuf;
use thiserror::Error;

/// Errors from a single LLM provider request.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// The provider returned a non-success HTTP status.
    #[error("HTTP {status}: {body}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Response body.
        body: String,
        /// Seconds to wait before retrying, if provided by the server.
        retry_after: Option<u64>,
    },

    /// The HTTP request itself failed (network, DNS, TLS, etc.).
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// The response body could not be parsed into the expected format.
    #[error("Response parse error: {0}")]
    Parse(String),

    /// No API key was found for this provider.
    #[error("API key not configured for provider '{0}'")]
    MissingApiKey(String),

    /// A catch-all for other provider-specific errors.
    #[error("Provider error: {0}")]
    Other(String),
}

impl ProviderError {
    /// Returns the HTTP status code, if this was an HTTP error.
    pub fn http_status(&self) -> Option<u16> {
        match self {
            ProviderError::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Returns the `Retry-After` header value in seconds, if present.
    pub fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            ProviderError::Http { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Returns `true` if the provider returned HTTP 429 (rate limited).
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, ProviderError::Http { status: 429, .. })
    }
}

/// All providers in a cascade failed.
#[derive(Debug, Error)]
#[error("All providers in cascade '{cascade_name}' failed: {message}\nFailed prompt persisted to: {}", .failed_prompt_path.display())]
pub struct CascadeError {
    /// Name of the cascade that failed.
    pub cascade_name: String,
    /// Combined error messages from each failed entry.
    pub message: String,
    /// Absolute path to the persisted `.json` file containing the failed conversation.
    pub failed_prompt_path: PathBuf,
}
