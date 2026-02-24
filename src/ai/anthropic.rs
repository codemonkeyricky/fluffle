//! Anthropic provider implementation.
//!
//! This module provides an `AiProvider` implementation using the Anthropic API.

use crate::error::{Error, Result};
use crate::ai::{AiProvider, Message, ToolDefinition, AiResponse, ToolCall};
use async_trait::async_trait;

pub struct AnthropicProvider {
    api_key: Option<String>,
}

impl AnthropicProvider {
    pub fn new(api_key: Option<&str>) -> Result<Self> {
        Ok(Self {
            api_key: api_key.map(|s| s.to_string()),
        })
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    async fn complete_with_tools(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolDefinition>,
    ) -> Result<AiResponse> {
        Err(Error::Ai("Anthropic provider not yet implemented".to_string()))
    }
}