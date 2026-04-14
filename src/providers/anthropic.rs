use reqwest::Client;
use serde_json::{json, Value};

use crate::error::ProviderError;
use crate::models::{ContentBlock, Conversation, LlmResponse, MessageRole};
use crate::providers::LlmProvider;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, conversation: &Conversation) -> Result<LlmResponse, ProviderError> {
        let mut system_parts = Vec::new();
        let mut anthropic_messages = Vec::new();

        for msg in &conversation.messages {
            match msg.role {
                MessageRole::System => {
                    system_parts.push(msg.content.clone());
                }
                MessageRole::Tool => {
                    let tool_result = json!({
                        "type": "tool_result",
                        "tool_use_id": msg.tool_call_id.as_deref().unwrap_or(""),
                        "content": msg.content,
                    });
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": vec![tool_result],
                    }));
                }
                _ => {
                    let role = match msg.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        _ => continue,
                    };

                    anthropic_messages.push(json!({
                        "role": role,
                        "content": msg.content,
                    }));
                }
            }
        }

        let mut body = json!({
            "model": self.model,
            "messages": anthropic_messages,
            "max_tokens": 4096,
        });

        if !system_parts.is_empty() {
            body["system"] = json!(system_parts.join("\n\n"));
        }

        if let Some(ref tools) = conversation.tools {
            let anthropic_tools: Vec<Value> = tools.iter().map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            }).collect();
            body["tools"] = json!(anthropic_tools);
        }

        let url = format!(
            "{}/v1/messages",
            self.base_url.trim_end_matches('/')
        );

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Http {
                status: status.as_u16(),
                body: error_body,
                retry_after,
            });
        }

        let data: Value = response.json().await.map_err(|e| ProviderError::Parse(e.to_string()))?;

        let mut content_blocks = Vec::new();

        if let Some(content) = data["content"].as_array() {
            for block in content {
                let block_type = block["type"].as_str().unwrap_or("");
                match block_type {
                    "text" => {
                        let text = block["text"].as_str().unwrap_or("").to_string();
                        content_blocks.push(ContentBlock::Text { text });
                    }
                    "tool_use" => {
                        let id = block["id"].as_str().unwrap_or("").to_string();
                        let name = block["name"].as_str().unwrap_or("").to_string();
                        let input = block["input"].to_string();
                        content_blocks.push(ContentBlock::ToolCall { id, name, arguments: input });
                    }
                    _ => {}
                }
            }
        }

        let usage = &data["usage"];
        let input_tokens = usage["input_tokens"].as_u64().map(|v| v as u32);
        let output_tokens = usage["output_tokens"].as_u64().map(|v| v as u32);
        let model = data["model"].as_str().unwrap_or(&self.model).to_string();

        Ok(LlmResponse {
            content: content_blocks,
            input_tokens,
            output_tokens,
            model,
        })
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
