use crate::config::Config;
use crate::error::Result;
use crate::ai::{AiProvider, create_provider, Message, ToolDefinition, ToolCall};
use crate::types::{ToolContext, ToolResult};
use std::sync::Arc;

pub struct Agent {
    config: Config,
    tools: Vec<Arc<dyn crate::plugin::Tool>>,
    context: ToolContext,
    ai_provider: Box<dyn AiProvider>,
    conversation_history: Vec<Message>,
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
            conversation_history: Vec::new(),
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    fn discover_tools() -> Vec<Arc<dyn crate::plugin::Tool>> {
        use inventory::iter;

        let mut tools = Vec::new();

        // Iterate over all registered plugins and collect their tools
        for plugin in iter::<&dyn crate::plugin::Plugin> {
            for tool in plugin.tools() {
                tools.push(tool);
            }
        }

        tools
    }

    /// Process a user message and return the AI's response.
    /// This handles the full tool calling flow: adding user message to history,
    /// getting AI response with tools, executing tool calls, and updating history.
    pub async fn process(&mut self, user_message: &str) -> Result<String> {
        // Add user message to conversation history
        self.conversation_history.push(Message::user(user_message));

        // Convert tools to AI tool definitions
        let tool_definitions = self.tools_to_definitions();

        // Get AI response with tools
        let ai_response = self.ai_provider.complete_with_tools(
            self.conversation_history.clone(),
            tool_definitions,
        ).await?;

        // Execute any tool calls
        let mut final_response = ai_response.content.clone();
        let tool_calls = ai_response.tool_calls.clone(); // Clone to avoid move issues

        if !tool_calls.is_empty() {
            // Execute each tool call
            for tool_call in &tool_calls {
                // Execute the tool
                let tool_result = self.execute_tool(tool_call).await?;

                // Create tool message with result
                let tool_message = if tool_result.is_success() {
                    Message::tool(&tool_call.id, tool_result.output())
                } else {
                    Message::tool(&tool_call.id, &format!("Error: {}", tool_result.error_message().unwrap_or("Unknown error")))
                };

                // Add tool message to history
                self.conversation_history.push(tool_message);

                // If tool execution failed, update final response
                if !tool_result.is_success() {
                    final_response = format!("{} (Tool execution failed: {})",
                        final_response,
                        tool_result.error_message().unwrap_or("Unknown error"));
                }
            }

            // After tool execution, get final AI response
            // Note: We might want to get another AI response here with the tool results
            // For simplicity, we'll just return the initial response for now
        }

        // Add AI response to history
        if tool_calls.is_empty() {
            // Simple assistant response without tool calls
            self.conversation_history.push(Message::assistant(&final_response));
        } else {
            // Assistant response with tool calls
            self.conversation_history.push(Message::assistant_with_tool_calls(
                &final_response,
                tool_calls,
            ));
        }

        Ok(final_response)
    }

    /// Convert the agent's tools to AI tool definitions.
    fn tools_to_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|tool| {
            ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters(),
            }
        }).collect()
    }

    /// Execute a single tool call.
    async fn execute_tool(&self, tool_call: &ToolCall) -> Result<ToolResult> {
        // Find the tool by name
        let tool = self.tools.iter()
            .find(|t| t.name() == tool_call.name)
            .ok_or_else(|| {
                crate::error::Error::ToolExecution(format!("Tool not found: {}", tool_call.name))
            })?;

        // Execute the tool with the provided arguments
        let result = tool.execute(&self.context, tool_call.arguments.clone()).await;
        Ok(result)
    }

    /// Get a reference to the agent's tools.
    pub fn tools(&self) -> &[Arc<dyn crate::plugin::Tool>] {
        &self.tools
    }

    /// Get a reference to the conversation history.
    pub fn conversation_history(&self) -> &[Message] {
        &self.conversation_history
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
        assert!(!agent.tools().is_empty(), "Agent should discover tools from plugins");

        // Verify context was created (we need to check through tools or add a context() method)
        // For now, just verify agent was created successfully
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