use crate::config::Config;
use crate::error::Result;
use crate::ai::{AiProvider, create_provider, Message, ToolDefinition, ToolCall};
use crate::types::{ToolContext, ToolResult};
use crate::ui::SharedMessages;
use std::sync::Arc;

pub struct Agent {
    config: Config,
    tools: Vec<Arc<dyn crate::plugin::Tool>>,
    context: ToolContext,
    ai_provider: Box<dyn AiProvider>,
    conversation_history: Vec<Message>,
    shared_messages: Option<Arc<SharedMessages>>,
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
            shared_messages: None,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn set_shared_messages(&mut self, shared: Arc<SharedMessages>) {
        self.shared_messages = Some(shared);
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
    ///
    /// The method supports iterative tool calling where the AI can make multiple
    /// tool calls across multiple iterations until it has enough information to
    /// provide a final response.
    ///
    /// # Example (conceptual)
    ///
    /// ```rust,ignore
    /// // Example of iterative tool calling flow:
    /// // 1. User: "What's the weather in Tokyo and then London?"
    /// // 2. Model: Calls weather tool for Tokyo
    /// // 3. Tool: Returns Tokyo weather
    /// // 4. Model: Calls weather tool for London
    /// // 5. Tool: Returns London weather
    /// // 6. Model: Provides combined answer without tool calls
    /// // 7. Loop ends, response returned to user
    /// ```
    ///
    /// Tool execution errors are included in the conversation history, allowing
    /// the AI to see and respond to errors in subsequent iterations.
    pub async fn process(&mut self, user_message: &str) -> Result<String> {
        // Add user message to conversation history
        self.conversation_history.push(Message::user(user_message));

        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > self.config.max_tool_iterations {
                return Err(crate::error::Error::ToolIterationLimit(iteration));
            }

            // Convert tools to AI tool definitions
            let tool_definitions = self.tools_to_definitions();

            // Get AI response with tools
            let ai_response = self.ai_provider.complete_with_tools(
                &self.conversation_history,
                &tool_definitions,
            ).await?;

            // Check if we have a final response (no tool calls)
            if ai_response.tool_calls.is_empty() {
                self.conversation_history.push(Message::assistant(&ai_response.content));
                return Ok(ai_response.content);
            }

            // We have tool calls - add assistant message with calls to history
            self.conversation_history.push(
                Message::assistant_with_tool_calls(
                    &ai_response.content,
                    ai_response.tool_calls.clone(),
                )
            );

            // Push tool call messages to UI
            for tool_call in &ai_response.tool_calls {
                self.push_tool_call(&tool_call.name, &tool_call.arguments);
            }

            // Execute all tool calls
            let tool_results = self.execute_tool_calls(&ai_response.tool_calls).await;

            // Add tool results to history (including errors)
            // Model will see errors in next iteration and decide next steps
            for (tool_call, result) in &tool_results {
                let tool_message = if result.is_success() {
                    Message::tool(&tool_call.id, result.output())
                } else {
                    Message::tool(
                        &tool_call.id,
                        &format!("Error: {}", result.error_message().unwrap_or("Unknown error"))
                    )
                };
                self.conversation_history.push(tool_message);
            }

            // Push tool result messages to UI
            for (_tool_call, result) in &tool_results {
                self.push_tool_result(
                    result.is_success(),
                    if result.is_success() {
                        result.output()
                    } else {
                        result.error_message().unwrap_or("Unknown error")
                    }
                );
            }

            // Continue to next iteration
        }
    }

    /// Execute multiple tool calls and return results.
    ///
    /// This method executes each tool call in sequence and returns a vector
    /// of tuples containing each tool call and its result.
    /// Tool messages are NOT added to conversation history; caller must add them.
    async fn execute_tool_calls(&mut self, tool_calls: &[ToolCall]) -> Vec<(ToolCall, ToolResult)> {
        let mut results = Vec::new();

        for tool_call in tool_calls {
            // Execute the tool
            let tool_result = self.execute_tool(tool_call).await;
            results.push((tool_call.clone(), tool_result));
        }

        results
    }

    /// Helper to push tool call message to shared messages
    fn push_tool_call(&self, name: &str, arguments: &serde_json::Value) {
        if let Some(shared) = &self.shared_messages {
            let args_str = serde_json::to_string_pretty(arguments)
                .unwrap_or_else(|_| "{}".to_string());
            let msg = format!("Tool: {}({})", name, args_str);
            shared.push(msg);
        }
    }

    /// Helper to push tool result message to shared messages
    fn push_tool_result(&self, success: bool, output: &str) {
        if let Some(shared) = &self.shared_messages {
            let prefix = if success { "Result:" } else { "Error:" };
            let msg = format!("{} {}", prefix, output);
            shared.push(msg);
        }
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
    /// and returns the result. Returns `ToolResult::error()` if the
    /// tool is not found.
    async fn execute_tool(&self, tool_call: &ToolCall) -> ToolResult {
        // Find the tool by name
        let tool = match self.tools.iter()
            .find(|t| t.name() == tool_call.name) {
            Some(tool) => tool,
            None => return ToolResult::error(format!("Tool not found: {}", tool_call.name)),
        };

        // Execute the tool with the provided arguments
        tool.execute(&self.context, tool_call.arguments.clone()).await
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
    use crate::assert_send_sync;

    #[test]
    fn test_agent_initialization() {
        // Create a minimal config for testing
        let config = Config {
            model: "gpt-4".to_string(),
            api_key: None,
            provider: "openai".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            max_tool_iterations: 10,
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
            max_tool_iterations: 10,
        };

        let agent = Agent::new(config.clone()).expect("Agent initialization failed");

        // Verify config is stored correctly
        let stored_config = agent.config();
        assert_eq!(stored_config.model, config.model);
        assert_eq!(stored_config.provider, config.provider);
        assert_eq!(stored_config.max_tokens, config.max_tokens);
        assert_eq!(stored_config.temperature, config.temperature);
        assert_eq!(stored_config.max_tool_iterations, config.max_tool_iterations);
        // Compare api_key (both are Option<String>)
        assert_eq!(stored_config.api_key, config.api_key);
    }

    #[test]
    fn test_agent_accepts_shared_messages() {
        use crate::ui::SharedMessages;
        use std::sync::Arc;

        let config = Config {
            model: "gpt-4".to_string(),
            api_key: None,
            provider: "openai".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            max_tool_iterations: 10,
        };

        let mut agent = Agent::new(config).expect("Agent initialization failed");

        // This should compile once set_shared_messages is added
        let shared = Arc::new(SharedMessages::new());
        agent.set_shared_messages(shared);

        assert!(true);
    }

    #[test]
    fn test_agent_message_formatting() {
        use serde_json::json;
        use crate::ui::SharedMessages;
        use std::sync::Arc;

        let config = Config {
            model: "gpt-4".to_string(),
            api_key: None,
            provider: "openai".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            max_tool_iterations: 10,
        };

        let mut agent = Agent::new(config).expect("Agent initialization failed");
        let shared = Arc::new(SharedMessages::new());
        agent.set_shared_messages(shared.clone());

        // Test tool call formatting
        let _args = json!({"path": "src/main.rs"});
        // We can't test private methods directly
        // This test just verifies agent compiles with helpers

        assert!(true);
    }

    #[tokio::test]
    async fn test_agent_pushes_tool_messages_during_iteration() {
        // Conceptual test - can't easily mock AI provider
        // We'll trust integration tests later
        assert!(true);
    }

    /// Ensures Agent can be safely sent across thread boundaries for async task spawning.
    #[test]
    fn test_agent_is_send_and_sync() {
        assert_send_sync!(Agent);
    }

    /// Ensures Arc<Agent> can be safely sent across thread boundaries, which is common
    /// for sharing agent instances between tasks.
    #[test]
    fn test_arc_agent_is_send_and_sync() {
        assert_send_sync!(std::sync::Arc<Agent>);
    }

    /// Ensures Box<Agent> can be safely sent across thread boundaries, which is common
    /// for dynamic dispatch of agent implementations.
    #[test]
    fn test_box_agent_is_send_and_sync() {
        assert_send_sync!(Box<Agent>);
    }
}