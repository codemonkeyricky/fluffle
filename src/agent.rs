use crate::config::Config;
use crate::error::Result;
use crate::ai::{AiProvider, create_provider};

pub struct Agent {
    config: Config,
    tools: Vec<std::sync::Arc<dyn crate::plugin::Tool>>,
    context: crate::types::ToolContext,
    ai_provider: Box<dyn AiProvider>,
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

        // Create AI provider based on configuration
        let ai_provider = create_provider(&config.provider, config.api_key.as_deref())?;

        Ok(Self {
            config,
            tools,
            context,
            ai_provider,
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