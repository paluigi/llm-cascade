pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai;

use crate::error::ProviderError;
use crate::models::{Conversation, LlmResponse};

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, conversation: &Conversation) -> Result<LlmResponse, ProviderError>;

    fn provider_name(&self) -> &str;

    fn model_name(&self) -> &str;

    fn entry_key(&self) -> String {
        format!("{}/{}", self.provider_name(), self.model_name())
    }
}
