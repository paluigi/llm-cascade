use reqwest::Client;
use serde_json::{json, Value};

use crate::error::ProviderError;
use crate::models::{ContentBlock, Conversation, LlmResponse, MessageRole};
use crate::providers::LlmProvider;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(model: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, conversation: &Conversation) -> Result<LlmResponse, ProviderError> {
        let mut messages = Vec::new();

        for msg in &conversation.messages {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };

            let mut message = json!({
                "role": role,
                "content": msg.content,
            });

            if let Some(ref tool_call_id) = msg.tool_call_id {
                message["tool_call_id"] = json!(tool_call_id);
            }

            messages.push(message);
        }

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });

        if let Some(ref tools) = conversation.tools {
            let ollama_tools: Vec<Value> = tools.iter().map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            }).collect();
            body["tools"] = json!(ollama_tools);
        }

        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));

        let response = self.client.post(&url).json(&body).send().await?;

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

        let message = &data["message"];
        let mut content_blocks = Vec::new();

        if let Some(content) = message["content"].as_str()
            && !content.is_empty()
        {
            content_blocks.push(ContentBlock::Text { text: content.to_string() });
        }

        if let Some(tool_calls) = message["tool_calls"].as_array() {
            for tc in tool_calls {
                let function = &tc["function"];
                let name = function["name"].as_str().unwrap_or("").to_string();
                let arguments = function["arguments"].to_string();
                let id = tc["id"].as_str().unwrap_or("").to_string();
                content_blocks.push(ContentBlock::ToolCall { id, name, arguments });
            }
        }

        let eval_count = data.get("eval_count").and_then(|v| v.as_u64()).map(|v| v as u32);
        let prompt_eval_count = data.get("prompt_eval_count").and_then(|v| v.as_u64()).map(|v| v as u32);

        Ok(LlmResponse {
            content: content_blocks,
            input_tokens: prompt_eval_count,
            output_tokens: eval_count,
            model: self.model.clone(),
        })
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
