//! Google Gemini generateContent API provider.

use reqwest::Client;
use serde_json::{Value, json};

use crate::error::ProviderError;
use crate::models::{ContentBlock, Conversation, LlmResponse, MessageRole};
use crate::providers::LlmProvider;

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// Provider for the Google Gemini generateContent API.
pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl GeminiProvider {
    /// Creates a new Gemini provider.
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
impl LlmProvider for GeminiProvider {
    async fn complete(&self, conversation: &Conversation) -> Result<LlmResponse, ProviderError> {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for msg in &conversation.messages {
            match msg.role {
                MessageRole::System => {
                    system_instruction = Some(msg.content.clone());
                }
                MessageRole::User | MessageRole::Assistant => {
                    let role = match msg.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "model",
                        _ => unreachable!(),
                    };

                    contents.push(json!({
                        "role": role,
                        "parts": [{ "text": msg.content }],
                    }));
                }
                MessageRole::Tool => {
                    let function_response = json!({
                        "role": "function",
                        "parts": [{
                            "functionResponse": {
                                "name": msg.tool_call_id.as_deref().unwrap_or("unknown"),
                                "response": { "content": msg.content },
                            }
                        }],
                    });
                    contents.push(function_response);
                }
            }
        }

        let mut body = json!({
            "contents": contents,
        });

        if let Some(system) = system_instruction {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system }],
            });
        }

        if let Some(ref tools) = conversation.tools {
            let function_declarations: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    })
                })
                .collect();
            body["tools"] = json!([{ "functionDeclarations": function_declarations }]);
        }

        let base = self.base_url.trim_end_matches('/');
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            base, self.model, self.api_key
        );

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

        let data: Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let mut content_blocks = Vec::new();

        if let Some(candidates) = data["candidates"].as_array()
            && let Some(first) = candidates.first()
            && let Some(parts) = first["content"]["parts"].as_array()
        {
            for part in parts {
                if let Some(text) = part["text"].as_str() {
                    content_blocks.push(ContentBlock::Text {
                        text: text.to_string(),
                    });
                }
                if let Some(fc) = part.get("functionCall") {
                    let name = fc["name"].as_str().unwrap_or("").to_string();
                    let args = fc["args"].to_string();
                    let id = format!(
                        "call_{}",
                        serde_json::to_string(&fc).map(|s| s.len()).unwrap_or(0)
                    );
                    content_blocks.push(ContentBlock::ToolCall {
                        id,
                        name,
                        arguments: args,
                    });
                }
            }
        }

        let usage = &data["usageMetadata"];
        let input_tokens = usage["promptTokenCount"].as_u64().map(|v| v as u32);
        let output_tokens = usage["candidatesTokenCount"].as_u64().map(|v| v as u32);

        Ok(LlmResponse {
            content: content_blocks,
            input_tokens,
            output_tokens,
            model: self.model.clone(),
        })
    }

    fn provider_name(&self) -> &str {
        "gemini"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
