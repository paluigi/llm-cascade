//! Conversation and message types.

use serde::{Deserialize, Serialize};

use crate::models::ToolDefinition;

/// The role of a message sender.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System instructions that set context for the assistant.
    System,
    /// User input.
    User,
    /// Assistant response.
    Assistant,
    /// Tool execution result.
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The sender's role.
    pub role: MessageRole,
    /// Text content of the message.
    pub content: String,
    /// Associates a tool result with its originating tool call. Only set when `role` is `Tool`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Creates a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_call_id: None,
        }
    }

    /// Creates a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_call_id: None,
        }
    }

    /// Creates an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_call_id: None,
        }
    }

    /// Creates a tool result message, associated with a tool call ID.
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// An ordered list of messages, optionally with tool definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// The conversation messages.
    pub messages: Vec<Message>,
    /// Optional tool definitions to pass to the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

impl Conversation {
    /// Creates a new conversation from a list of messages.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            tools: None,
        }
    }

    /// Attaches tool definitions to the conversation.
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Convenience constructor for a single-user-prompt conversation.
    pub fn single_user_prompt(prompt: impl Into<String>) -> Self {
        Self {
            messages: vec![Message::user(prompt.into())],
            tools: None,
        }
    }
}
