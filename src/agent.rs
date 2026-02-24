use crate::config::Config;
use crate::error::Result;
use anthropic_sdk;

// Type aliases for AI clients
type OpenAiClient = async_openai::Client<async_openai::config::OpenAIConfig>;
type AnthropicClient = anthropic_sdk::Client;

/// Enum representing different AI providers
pub enum AiClient {
    /// OpenAI client using async-openai crate
    OpenAI(OpenAiClient),
    /// Anthropic client using anthropic-sdk crate
    Anthropic(AnthropicClient),
}

impl AiClient {
    /// Create an AI client from configuration
    pub fn from_config(config: &Config) -> Result<Self> {
        match config.provider.as_str() {
            "openai" => {
                // Create OpenAI client
                // If api_key is provided, use it, otherwise rely on environment variable
                let client = async_openai::Client::new();
                Ok(AiClient::OpenAI(client))
            }
            "anthropic" => {
                // Create Anthropic client
                // Client reads API key from environment variable ANTHROPIC_API_KEY
                let client = anthropic_sdk::Client::new();
                Ok(AiClient::Anthropic(client))
            }
            _ => Err(crate::error::Error::Ai(format!("Unsupported provider: {}", config.provider))),
        }
    }
}

pub struct Agent {
    config: Config,
    tools: Vec<std::sync::Arc<dyn crate::plugin::Tool>>,
    context: crate::types::ToolContext,
    ai_client: AiClient,
}

impl Agent {
    pub fn new(config: Config) -> Result<Self> {
        // Discover tools from inventory
        let tools = Self::discover_tools();

        // Create default context (current directory, empty permissions)
        let context = crate::types::ToolContext {
            working_directory: std::env::current_dir()
                .map_err(|e| crate::error::Error::Io(e))?,
            permissions: Vec::new(),
        };

        // Create AI client based on provider
        let ai_client = AiClient::from_config(&config)?;

        Ok(Self {
            config,
            tools,
            context,
            ai_client,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    fn discover_tools() -> Vec<std::sync::Arc<dyn crate::plugin::Tool>> {
        use inventory::iter;
        use std::sync::Arc;

        let mut tools = Vec::new();

        // Iterate over all registered plugins and collect their tools
        for plugin in iter::<&dyn crate::plugin::Plugin> {
            for tool in plugin.tools() {
                tools.push(tool);
            }
        }

        tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_initialization() {
        // Create a minimal config for testing
        let config = Config {
            model: "gpt-4".to_string(),
            api_key: None,
            provider: "openai".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        };

        // This should succeed without panicking
        let agent = Agent::new(config).expect("Agent initialization failed");

        // Verify that tools were discovered (there should be some from plugins)
        assert!(!agent.tools.is_empty(), "Agent should discover tools from plugins");

        // Verify context was created
        assert!(agent.context.working_directory.exists());
    }

    #[test]
    fn test_agent_stores_config() {
        let config = Config {
            model: "gpt-4".to_string(),
            api_key: Some("test-key".to_string()),
            provider: "openai".to_string(),
            max_tokens: 1000,
            temperature: 0.5,
        };

        let agent = Agent::new(config.clone()).expect("Agent initialization failed");

        // Verify config is stored correctly
        let stored_config = agent.config();
        assert_eq!(stored_config.model, config.model);
        assert_eq!(stored_config.provider, config.provider);
        assert_eq!(stored_config.max_tokens, config.max_tokens);
        assert_eq!(stored_config.temperature, config.temperature);
        // Compare api_key (both are Option<String>)
        assert_eq!(stored_config.api_key, config.api_key);
    }
}