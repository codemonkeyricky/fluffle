//! AI abstraction layer for the nano code agent.
//!
//! This module provides a unified interface to different AI providers
//! (OpenAI, Anthropic) through the `AiProvider` trait. Providers can be
//! created at runtime based on configuration.

mod types;
mod openai;
mod anthropic;

pub use types::*;
pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;

use crate::error::Result;
use async_trait::async_trait;

/// Trait for AI providers that can complete chat conversations with tools.
#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<AiResponse>;
}

pub fn create_provider(provider_type: &str, api_key: Option<&str>) -> Result<Box<dyn AiProvider>> {
    let normalized = provider_type.trim().to_lowercase();
    match normalized.as_str() {
        "openai" => Ok(Box::new(OpenAiProvider::new(api_key)?)),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(api_key)?)),
        _ => Err(crate::error::Error::Ai(format!("Unsupported provider: {}", provider_type))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_send_sync;

    /// Ensures AI provider trait objects can be safely sent across thread boundaries,
    /// which is required for multi-threaded async execution.
    #[test]
    fn test_ai_provider_is_send_and_sync() {
        assert_send_sync!(Box<dyn AiProvider>);
    }
}