//! Anthropic provider implementation.
//!
//! This module provides an `AiProvider` implementation using the Anthropic API.

use crate::ai::types::{AiResponse, Message, ToolDefinition};
use crate::error::Result;
use async_trait::async_trait;
use super::AiProvider;

/// Anthropic provider implementation.
pub struct AnthropicProvider {
    api_key: Option<String>,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Optional Anthropic API key. If not provided, the provider
    ///   will attempt to read it from the environment.
    pub fn new(api_key: Option<&str>) -> Result<Self> {
        // TODO: implement
        Err(crate::error::Error::Ai("Anthropic provider not yet implemented".to_string()))
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    async fn complete_with_tools(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolDefinition>,
    ) -> Result<AiResponse> {
        // TODO: implement
        Err(crate::error::Error::Ai("Anthropic provider not yet implemented".to_string()))
    }
}