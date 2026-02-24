//! Shared types for AI communication.
//!
//! This module defines the data structures used to communicate with AI providers,
//! including messages, tool definitions, and responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
        }
    }

    pub fn tool(name: &str, content: &str) -> Self {
        Self {
            role: MessageRole::Tool,
            content: format!("{}: {}", name, content),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug)]
pub struct AiResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}