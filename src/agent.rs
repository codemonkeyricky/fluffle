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

    /// Discover all tools registered via the plugin inventory.
    /// This method scans the compile-time plugin registry and collects
    /// all available tools from all registered plugins.
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

        // Get initial AI response with tools
        let ai_response = self.ai_provider.complete_with_tools(
            &self.conversation_history,
            &tool_definitions,
        ).await?;

        let tool_calls = ai_response.tool_calls.clone();

        if tool_calls.is_empty() {
            // No tool calls - simple response
            self.conversation_history.push(Message::assistant(&ai_response.content));
            return Ok(ai_response.content);
        }

        // Add assistant message with tool calls to history
        self.conversation_history.push(Message::assistant_with_tool_calls(
            &ai_response.content,
            tool_calls.clone(),
        ));

        // Execute tool calls
        let tool_results = self.execute_tool_calls(&tool_calls).await?;

        // Check if any tool execution failed
        let final_response = if Self::has_tool_execution_failures(&tool_results) {
            // If tools failed, return the initial response with error info
            Self::format_response_with_errors(&ai_response.content, &tool_results)
        } else {
            // All tools succeeded, get final AI response
            self.get_final_ai_response(&tool_definitions).await?
        };

        Ok(final_response)
    }

    /// Execute multiple tool calls and add results to conversation history.
    ///
    /// This method executes each tool call in sequence, creates tool messages
    /// with the results, and adds them to the conversation history.
    /// Returns a vector of tuples containing each tool call and its result.
    async fn execute_tool_calls(&mut self, tool_calls: &[ToolCall]) -> Result<Vec<(ToolCall, ToolResult)>> {
        let mut results = Vec::new();

        for tool_call in tool_calls {
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

            results.push((tool_call.clone(), tool_result));
        }

        Ok(results)
    }

    /// Convert the agent's tools to AI tool definitions.
    ///
    /// This method transforms the internal tool representations into
    /// `ToolDefinition` structs that can be passed to AI providers.
    /// Each definition includes the tool name, description, and parameter schema.
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
    ///
    /// Finds the tool by name, executes it with the provided arguments,
    /// and returns the result. Returns `Error::ToolExecution` if the
    /// tool is not found.
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

    /// Check if any tool execution in the results failed.
    fn has_tool_execution_failures(tool_results: &[(ToolCall, ToolResult)]) -> bool {
        tool_results.iter().any(|(_, result)| !result.is_success())
    }

    /// Format the initial AI response with tool execution error messages.
    fn format_response_with_errors(initial_response: &str, tool_results: &[(ToolCall, ToolResult)]) -> String {
        let error_messages: Vec<String> = tool_results.iter()
            .filter_map(|(tool_call, result)| {
                if !result.is_success() {
                    Some(format!("{}: {}", tool_call.name, result.error_message().unwrap_or("Unknown error")))
                } else {
                    None
                }
            })
            .collect();

        if error_messages.is_empty() {
            initial_response.to_string()
        } else {
            format!("{} (Tool execution errors: {})",
                initial_response,
                error_messages.join(", "))
        }
    }

    /// Get final AI response after successful tool execution.
    async fn get_final_ai_response(&mut self, tool_definitions: &[ToolDefinition]) -> Result<String> {
        let final_ai_response = self.ai_provider.complete_with_tools(
            &self.conversation_history,
            tool_definitions,
        ).await?;

        // Add final assistant response to history
        self.conversation_history.push(Message::assistant(&final_ai_response.content));

        Ok(final_ai_response.content)
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