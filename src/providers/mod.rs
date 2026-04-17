//! LLM provider implementations.

pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai;

use crate::error::ProviderError;
use crate::models::{Conversation, LlmResponse};

/// Trait implemented by all LLM provider backends.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Sends the conversation to the provider and returns the response.
    async fn complete(&self, conversation: &Conversation) -> Result<LlmResponse, ProviderError>;

    /// Returns the provider name (e.g., `"openai"`, `"anthropic"`).
    fn provider_name(&self) -> &str;

    /// Returns the model identifier (e.g., `"gpt-4o"`, `"claude-sonnet-4-20250514"`).
    fn model_name(&self) -> &str;

    /// Returns the combined `provider/model` key used for cooldown tracking.
    fn entry_key(&self) -> String {
        format!("{}/{}", self.provider_name(), self.model_name())
    }
}
