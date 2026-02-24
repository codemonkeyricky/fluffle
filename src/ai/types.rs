//! Shared types for AI communication.
//!
//! This module defines the data structures used to communicate with AI providers,
//! including messages, tool definitions, and responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::types::ToolParameters;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    /// Optional tool call ID (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Optional tool calls (for assistant messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl Message {
    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool(tool_call_id: &str, content: &str) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.to_string(),
            tool_call_id: Some(tool_call_id.to_string()),
            tool_calls: None,
        }
    }

    pub fn assistant_with_tool_calls(content: &str, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Some(tool_calls),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
}

#[derive(Debug)]
pub struct AiResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}