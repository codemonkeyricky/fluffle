//! OpenAI provider implementation.
//!
//! This module provides an `AiProvider` implementation using the OpenAI API.

use crate::ai::types::{AiResponse, Message, ToolDefinition};
use crate::error::Result;
use async_trait::async_trait;
use super::AiProvider;

/// OpenAI provider implementation.
pub struct OpenAiProvider {
    api_key: Option<String>,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Optional OpenAI API key. If not provided, the provider
    ///   will attempt to read it from the environment.
    pub fn new(api_key: Option<&str>) -> Result<Self> {
        // TODO: implement
        Err(crate::error::Error::Ai("OpenAI provider not yet implemented".to_string()))
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn complete_with_tools(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolDefinition>,
    ) -> Result<AiResponse> {
        // TODO: implement
        Err(crate::error::Error::Ai("OpenAI provider not yet implemented".to_string()))
    }
}