//! Tool definition for function calling.

use serde::{Deserialize, Serialize};

/// Describes a tool (function) available for the model to call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The function name.
    pub name: String,
    /// A description of what the function does.
    pub description: String,
    /// JSON Schema object describing the function parameters.
    pub parameters: serde_json::Value,
}
