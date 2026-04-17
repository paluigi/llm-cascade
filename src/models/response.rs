//! LLM response types.

use serde::{Deserialize, Serialize};

/// A block of content in an LLM response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text output.
    Text {
        /// The text content.
        text: String,
    },
    /// A tool call requested by the model.
    ToolCall {
        /// Unique identifier for this tool call.
        id: String,
        /// The function name to invoke.
        name: String,
        /// JSON-encoded function arguments.
        arguments: String,
    },
}

impl ContentBlock {
    /// Creates a text content block.
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text { text: text.into() }
    }

    /// Creates a tool call content block.
    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        ContentBlock::ToolCall {
            id: id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }

    /// Returns the text content if this is a `Text` block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ContentBlock::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// The full response from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Ordered content blocks (text, tool calls, etc.).
    pub content: Vec<ContentBlock>,
    /// Number of input tokens consumed, if reported by the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    /// Number of output tokens generated, if reported by the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    /// The model identifier returned by the provider.
    pub model: String,
}

impl LlmResponse {
    /// Concatenates all text blocks into a single string.
    pub fn text_only(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| b.as_text())
            .collect::<Vec<_>>()
            .join("")
    }
}
