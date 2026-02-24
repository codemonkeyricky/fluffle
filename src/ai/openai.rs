//! OpenAI provider implementation.
//!
//! This module provides an `AiProvider` implementation using the OpenAI API.

use crate::error::{Error, Result};
use crate::ai::{AiProvider, Message, ToolDefinition, AiResponse, ToolCall};
use async_openai::config::OpenAIConfig;
use async_trait::async_trait;

pub struct OpenAiProvider {
    client: async_openai::Client<OpenAIConfig>,
}

impl OpenAiProvider {
    pub fn new(api_key: Option<&str>) -> Result<Self> {
        let config = if let Some(key) = api_key {
            OpenAIConfig::new().with_api_key(key)
        } else {
            OpenAIConfig::new()
        };

        let client = async_openai::Client::with_config(config);
        Ok(Self { client })
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn complete_with_tools(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolDefinition>,
    ) -> Result<AiResponse> {
        Err(Error::Ai("OpenAI provider not yet implemented".to_string()))
    }
}