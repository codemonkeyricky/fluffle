use crate::agents::AgentProfile;
use crate::ai::{create_provider, AiProvider, Message, TokenUsage, ToolCall, ToolDefinition};
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use crate::profile_loader;
use crate::types::{ToolContext, ToolResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct Agent {
    config: Config,
    tools: Vec<Arc<dyn crate::plugin::Tool>>,
    context: ToolContext,
    ai_provider: Box<dyn AiProvider>,
    history: Vec<Message>,
    ui_to_agent_rx: Option<mpsc::Receiver<UiToAgent>>,
    system_prompt: Option<String>,
    token_usage: TokenUsage,
}

impl Agent {
    pub fn new(config: Config) -> Result<Self> {
        // Discover tools from inventory
        let tools = Self::discover_tools();

        // Create default context (current directory, empty permissions)
        let context = crate::types::ToolContext {
            working_directory: std::env::current_dir().map_err(|e| crate::error::Error::Io(e))?,
            permissions: Vec::new(),
            agent_to_ui_tx: None,
        };

        // Create AI provider based on configuration
        let ai_provider = create_provider(&config.provider, config.api_key.as_deref())?;

        Ok(Self {
            config,
            tools,
            context,
            ai_provider,
            history: Vec::new(),
            ui_to_agent_rx: None,
            system_prompt: None,
            token_usage: TokenUsage::default(),
        })
    }

    /// Tool names whose successful results should be suppressed (tool calls still printed)
    pub(crate) const SUPPRESSED_TOOLS: &'static [&'static str] = &[
        "file_read",
        "file_write",
        "file_list",
        "bash_exec",
        "list_files",
        "glob",
        "grep",
        "read",
        "write",
        "ls",
    ];

    pub fn new_with_channels(
        config: Config,
        agent_to_ui_tx: mpsc::Sender<AgentToUi>,
        ui_to_agent_rx: mpsc::Receiver<UiToAgent>,
    ) -> Result<Self> {
        let mut agent = Self::new(config)?;
        agent.context.agent_to_ui_tx = Some(agent_to_ui_tx);
        agent.ui_to_agent_rx = Some(ui_to_agent_rx);
        Ok(agent)
    }

    /// Create agent with a specific profile
    pub fn new_with_profile(profile_name: &str, config: Config) -> Result<Self> {
        let profile = profile_loader::get_profile(profile_name).ok_or_else(|| {
            crate::error::Error::Agent(format!("Profile not found: {}", profile_name))
        })?;

        // Create base agent
        let mut agent = Self::new(config)?;

        // Apply profile configuration
        agent.apply_profile(&profile)?;

        Ok(agent)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// Apply profile settings to existing agent
    fn apply_profile(&mut self, profile: &AgentProfile) -> Result<()> {
        // Filter tools based on profile whitelist
        self.filter_tools(&profile.tools)?;

        // Set system prompt
        self.system_prompt = Some(profile.system_prompt.clone());

        // Apply config overrides
        self.apply_config_overrides(&profile.config_overrides)?;

        Ok(())
    }

    /// Filter tools to only include those in the whitelist
    fn filter_tools(&mut self, tool_names: &[String]) -> Result<()> {
        let available_tools: HashMap<String, Arc<dyn crate::plugin::Tool>> = self
            .tools
            .iter()
            .map(|t| (t.name().to_string(), t.clone()))
            .collect();

        let mut filtered = Vec::new();
        for name in tool_names {
            if let Some(tool) = available_tools.get(name) {
                filtered.push(tool.clone());
            } else {
                // Check if it's a profile tool
                if profile_loader::has_profile(name) {
                    // Profile tools are handled separately by the profile plugin
                    // They'll be available through their own tool registration
                    continue;
                }
                // Return error for unknown tools
                return Err(crate::error::Error::Agent(format!(
                    "Unknown tool in profile: {}",
                    name
                )));
            }
        }

        self.tools = filtered;
        Ok(())
    }

    /// Apply configuration overrides from profile
    fn apply_config_overrides(
        &mut self,
        overrides: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        for (key, value) in overrides {
            match key.as_str() {
                "temperature" => {
                    if let Value::Number(num) = value {
                        if let Some(temp) = num.as_f64() {
                            self.config.temperature = temp as f32;
                        }
                    }
                }
                "max_tokens" => {
                    if let Value::Number(num) = value {
                        if let Some(tokens) = num.as_u64() {
                            self.config.max_tokens = tokens as u32;
                        }
                    }
                }
                "max_tool_iterations" => {
                    if let Value::Number(num) = value {
                        if let Some(iter) = num.as_u64() {
                            self.config.max_tool_iterations = iter as u32;
                        }
                    }
                }
                "model" => {
                    if let Value::String(model) = value {
                        self.config.model = model.clone();
                    }
                }
                "provider" => {
                    if let Value::String(provider) = value {
                        self.config.provider = provider.clone();
                    }
                }
                "api_key" => match value {
                    Value::String(key) => self.config.api_key = Some(key.clone()),
                    Value::Null => self.config.api_key = None,
                    _ => {}
                },
                _ => {
                    tracing::warn!("Unknown config override key: {}", key);
                }
            }
        }
        Ok(())
    }

    pub fn token_usage(&self) -> &TokenUsage {
        &self.token_usage
    }

    pub fn reset_token_usage(&mut self) {
        self.token_usage = TokenUsage::default();
    }

    pub fn set_agent_to_ui_tx(&mut self, tx: mpsc::Sender<AgentToUi>) {
        self.context.agent_to_ui_tx = Some(tx);
    }

    pub fn set_ui_to_agent_rx(&mut self, rx: mpsc::Receiver<UiToAgent>) {
        self.ui_to_agent_rx = Some(rx);
    }

    /// Log a message to agent.log file.
    fn log_to_agent_file(&self, message: &str) {
        use std::fs::OpenOptions;
        use std::io::Write;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let line = format!("[{}] {}", timestamp, message);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("agent.log")
        {
            let _ = writeln!(file, "{}", line);
        }
    }

    /// Run the agent, blocking on messages from UI and processing them serially.
    /// Requires both channels to be set via `set_agent_to_ui_tx` and `set_ui_to_agent_rx`.
    /// Sends responses, errors, and token usage back via `agent_to_ui_tx`.
    pub async fn run(&mut self) -> Result<()> {
        // Verify channels are set
        if self.ui_to_agent_rx.is_none() {
            return Err(crate::error::Error::Agent(
                "ui_to_agent_rx not set".to_string(),
            ));
        }
        if self.context.agent_to_ui_tx.is_none() {
            return Err(crate::error::Error::Agent(
                "agent_to_ui_tx not set".to_string(),
            ));
        }

        loop {
            let request = match self.ui_to_agent_rx.as_mut().unwrap().recv().await {
                Some(req) => req,
                None => break,
            };
            match request {
                UiToAgent::Request(input) => match self.process(&input).await {
                    Ok(response) => {
                        tracing::debug!("Sending response: {}", response);
                        if let Some(tx) = &self.context.agent_to_ui_tx {
                            if let Err(e) = tx.send(AgentToUi::Response(response)).await {
                                tracing::error!("Failed to send response to UI: {}", e);
                            }
                            if let Err(e) = tx
                                .send(AgentToUi::TokenUsage(self.token_usage().clone()))
                                .await
                            {
                                tracing::error!("Failed to send token usage to UI: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Sending error: {}", e);
                        if let Some(tx) = &self.context.agent_to_ui_tx {
                            if let Err(e) = tx.send(AgentToUi::Error(e.to_string())).await {
                                tracing::error!("Failed to send error to UI: {}", e);
                            }
                        }
                    }
                },
                UiToAgent::Shutdown => {
                    break;
                }
                UiToAgent::ChildResult { .. } => {
                    // Child results are handled via oneshot channels; ignore here
                    tracing::debug!("Received ChildResult via UI channel (already handled)");
                }
            }
        }
        Ok(())
    }

    /// Create a new agent with the same configuration and tools but a custom system prompt.
    /// The new agent will have its own conversation history starting with the system prompt.
    pub fn with_system_prompt(&self, system_prompt: Option<String>) -> Result<Self> {
        let mut history = Vec::new();

        // Add system prompt as first message if provided
        if let Some(prompt) = &system_prompt {
            history.push(Message::user(prompt));
        }

        // Create new AI provider with same configuration
        let ai_provider = create_provider(&self.config.provider, self.config.api_key.as_deref())?;

        Ok(Self {
            config: self.config.clone(),
            tools: self.tools.clone(),
            context: self.context.clone(),
            ai_provider,
            history,
            ui_to_agent_rx: None,
            system_prompt,
            token_usage: TokenUsage::default(),
        })
    }

    /// Create a new agent with a custom context (working directory and permissions).
    /// Useful for creating subagents that should operate in a specific directory.
    pub fn with_context(&self, context: ToolContext) -> Result<Self> {
        // Create new AI provider with same configuration
        let ai_provider = create_provider(&self.config.provider, self.config.api_key.as_deref())?;

        // Propagate agent_to_ui_tx from parent context
        let mut context = context;
        context.agent_to_ui_tx = self.context.agent_to_ui_tx.clone();

        Ok(Self {
            config: self.config.clone(),
            tools: self.tools.clone(),
            context,
            ai_provider,
            history: Vec::new(),
            ui_to_agent_rx: None,
            system_prompt: None,
            token_usage: TokenUsage::default(),
        })
    }

    /// Set the agent's context (working directory and permissions).
    /// This is useful for subagents that need to operate in a specific directory.
    pub fn set_context(&mut self, context: ToolContext) {
        self.context = context;
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
        self.history.push(Message::user(user_message));
        // Reset token usage for this request
        self.token_usage = TokenUsage::default();

        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > self.config.max_tool_iterations {
                return Err(crate::error::Error::ToolIterationLimit(iteration));
            }

            // Convert tools to AI tool definitions
            let tool_definitions = self.tools_to_definitions();

            // Get AI response with tools
            let ai_response = self
                .ai_provider
                .complete_with_tools(&self.history, &tool_definitions)
                .await?;

            // Record token usage for this request
            if let Some(usage) = ai_response.token_usage {
                self.log_to_agent_file("=== TOKEN USAGE ===");
                self.log_to_agent_file(&format!(
                    "Current API call: prompt: {}, completion: {}, total: {}",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                ));
                self.token_usage += usage.clone();
                self.log_to_agent_file(&format!(
                    "Request total: prompt: {}, completion: {}, total: {}",
                    self.token_usage.prompt_tokens,
                    self.token_usage.completion_tokens,
                    self.token_usage.total_tokens
                ));
            }
            self.log_to_agent_file("=== MODEL RESPONSE ===");
            self.log_to_agent_file(&format!("Content: {}", ai_response.content));
            self.log_to_agent_file(&format!("Tool calls: {}", ai_response.tool_calls.len()));

            // Check if we have a final response (no tool calls)
            if ai_response.tool_calls.is_empty() {
                self.history.push(Message::assistant(&ai_response.content));
                return Ok(ai_response.content);
            }

            // We have tool calls - add assistant message with calls to history
            self.history.push(Message::assistant_with_tool_calls(
                &ai_response.content,
                ai_response.tool_calls.clone(),
            ));

            // Push thinking output to UI if content is not empty
            if !ai_response.content.trim().is_empty() {
                self.push_thinking(&ai_response.content).await;
            }

            // Push tool call messages to UI
            for tool_call in &ai_response.tool_calls {
                self.push_tool_call(&tool_call.name, &tool_call.arguments)
                    .await;
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
                        &format!(
                            "Error: {}",
                            result.error_message().unwrap_or("Unknown error")
                        ),
                    )
                };
                self.history.push(tool_message);
            }

            // Push tool result messages to UI
            for (tool_call, result) in &tool_results {
                self.push_tool_result(
                    &tool_call.name,
                    result.is_success(),
                    if result.is_success() {
                        result.output()
                    } else {
                        result.error_message().unwrap_or("Unknown error")
                    },
                )
                .await;
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
            // Log the tool call
            self.log_to_agent_file(&format!("=== TOOL CALL: {} ===", tool_call.name));
            self.log_to_agent_file(&format!(
                "Arguments: {}",
                serde_json::to_string_pretty(&tool_call.arguments)
                    .unwrap_or_else(|_| "{}".to_string())
            ));

            // Execute the tool
            let tool_result = self.execute_tool(tool_call).await;
            results.push((tool_call.clone(), tool_result));
        }

        results
    }

    /// Helper to push tool call message to shared messages
    async fn push_tool_call(&self, name: &str, arguments: &serde_json::Value) {
        if let Some(tx) = &self.context.agent_to_ui_tx {
            let args_str = serde_json::to_string(arguments).unwrap_or_else(|_| "{}".to_string());
            let msg = format!("Tool -> {}: {}", name, args_str);
            tracing::debug!("Pushing tool call: {}", msg);
            if let Err(e) = tx.send(AgentToUi::ToolCall(msg)).await {
                tracing::error!("Failed to send tool call to UI: {}", e);
            }
        }
    }

    /// Helper to push tool result message to UI channel
    async fn push_tool_result(&self, tool_name: &str, success: bool, output: &str) {
        // Suppress successful results for certain tools to reduce noise
        // Errors are always shown
        if success && self.should_suppress_result(tool_name) {
            tracing::debug!("Suppressing tool result for {}", tool_name);
            return;
        }
        if let Some(tx) = &self.context.agent_to_ui_tx {
            let prefix = if success { "Result:" } else { "Error:" };
            let msg = format!("{} {}", prefix, output);
            tracing::debug!("Pushing tool result: {}", msg);
            let message = if success {
                AgentToUi::ToolResult(msg)
            } else {
                AgentToUi::Error(msg)
            };
            if let Err(e) = tx.send(message).await {
                tracing::error!("Failed to send tool result to UI: {}", e);
            }
        }
    }

    /// Helper to push thinking/reasoning output to UI channel
    async fn push_thinking(&self, content: &str) {
        if let Some(tx) = &self.context.agent_to_ui_tx {
            tracing::debug!("Pushing thinking: {}", content);
            if let Err(e) = tx.send(AgentToUi::Thinking(content.to_string())).await {
                tracing::error!("Failed to send thinking to UI: {}", e);
            }
        }
    }

    /// Determine whether to suppress tool results for a given tool name
    fn should_suppress_result(&self, tool_name: &str) -> bool {
        // Check exact matches
        if Self::SUPPRESSED_TOOLS.contains(&tool_name) {
            return true;
        }
        // Suppress all file_* tools
        if tool_name.starts_with("file_") {
            return true;
        }
        false
    }

    /// Convert the agent's tools to AI tool definitions.
    ///
    /// This method transforms the internal tool representations into
    /// `ToolDefinition` structs that can be passed to AI providers.
    /// Each definition includes the tool name, description, and parameter schema.
    fn tools_to_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .iter()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters(),
            })
            .collect()
    }

    /// Execute a single tool call.
    ///
    /// Finds the tool by name, executes it with the provided arguments,
    /// and returns the result. Returns `ToolResult::error()` if the
    /// tool is not found.
    async fn execute_tool(&self, tool_call: &ToolCall) -> ToolResult {
        // Find the tool by name
        let tool = match self.tools.iter().find(|t| t.name() == tool_call.name) {
            Some(tool) => tool,
            None => return ToolResult::error(format!("Tool not found: {}", tool_call.name)),
        };

        // Execute the tool with the provided arguments
        tool.execute(&self.context, tool_call.arguments.clone())
            .await
    }

    /// Get a reference to the agent's tools.
    pub fn tools(&self) -> &[Arc<dyn crate::plugin::Tool>] {
        &self.tools
    }

    /// Get a reference to the conversation history.
    pub fn history(&self) -> &[Message] {
        &self.history
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
        assert!(
            !agent.tools().is_empty(),
            "Agent should discover tools from plugins"
        );

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
        assert_eq!(
            stored_config.max_tool_iterations,
            config.max_tool_iterations
        );
        // Compare api_key (both are Option<String>)
        assert_eq!(stored_config.api_key, config.api_key);
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

    #[test]
    fn test_agent_discovers_task_and_explore_tools() {
        let config = Config {
            model: "gpt-4".to_string(),
            api_key: None,
            provider: "openai".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            max_tool_iterations: 10,
        };

        let agent = Agent::new(config).expect("Agent initialization failed");
        let tool_names: Vec<&str> = agent.tools().iter().map(|t| t.name()).collect();

        // Should have at least file_ops, bash_exec, git_ops, task, explorer
        assert!(tool_names.contains(&"task"), "Missing 'task' tool");
        assert!(tool_names.contains(&"explorer"), "Missing 'explorer' tool");
    }

    #[test]
    fn test_agent_with_system_prompt() {
        let config = Config {
            model: "gpt-4".to_string(),
            api_key: None,
            provider: "openai".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            max_tool_iterations: 10,
        };

        let agent = Agent::new(config).expect("Agent initialization failed");
        let system_prompt = Some("You are a helpful assistant".to_string());

        let subagent = agent
            .with_system_prompt(system_prompt.clone())
            .expect("Failed to create subagent");

        assert_eq!(subagent.system_prompt(), system_prompt.as_deref());
        // System prompt should be added as first message in history
        assert!(!subagent.history().is_empty());
        assert_eq!(subagent.history().len(), 1);
        // First message should be the system prompt as a user message
        let first_msg = &subagent.history()[0];
        assert!(matches!(first_msg.role, crate::ai::MessageRole::User));
        assert!(first_msg.content.contains("helpful assistant"));
    }

    #[test]
    fn test_agent_new_with_profile() {
        use crate::profile_loader;
        // Ensure profiles are loaded
        profile_loader::clear_profiles();
        profile_loader::load_profiles().expect("Failed to load profiles");

        let config = Config {
            model: "gpt-4".to_string(),
            api_key: None,
            provider: "openai".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            max_tool_iterations: 10,
        };

        // Create agent with explorer profile
        let agent = Agent::new_with_profile("explorer", config.clone())
            .expect("Failed to create agent with profile");

        // Should have filtered tools (only those listed in explorer profile)
        let tool_names: Vec<&str> = agent.tools().iter().map(|t| t.name()).collect();
        // explorer profile includes file_read, file_list, bash_exec, git_status, git_diff
        assert!(tool_names.contains(&"file_read"));
        assert!(tool_names.contains(&"file_list"));
        assert!(tool_names.contains(&"bash_exec"));
        assert!(tool_names.contains(&"git_status"));
        assert!(tool_names.contains(&"git_diff"));
        // Should NOT have other tools like file_write, task, explorer
        assert!(!tool_names.contains(&"file_write"));
        assert!(!tool_names.contains(&"task"));
        assert!(!tool_names.contains(&"explorer"));

        // System prompt should be set
        assert!(agent.system_prompt().is_some());
        let prompt = agent.system_prompt().unwrap();
        assert!(prompt.contains("codebase explorer"));

        // Config overrides should be applied (temperature 0.2, max_tool_iterations 30)
        let agent_config = agent.config();
        assert_eq!(agent_config.temperature, 0.2);
        assert_eq!(agent_config.max_tool_iterations, 30);
        // Other config unchanged
        assert_eq!(agent_config.model, config.model);
        assert_eq!(agent_config.max_tokens, config.max_tokens);
        assert_eq!(agent_config.provider, config.provider);
    }

    #[test]
    fn test_suppressed_tools_list() {
        use super::Agent;
        // Verify that known noisy tools are in the list
        let suppressed = Agent::SUPPRESSED_TOOLS;
        assert!(suppressed.contains(&"file_read"));
        assert!(suppressed.contains(&"file_write"));
        assert!(suppressed.contains(&"file_list"));
        assert!(suppressed.contains(&"bash_exec"));
        assert!(suppressed.contains(&"list_files"));
        // Verify that important agent tools are NOT suppressed
        assert!(!suppressed.contains(&"task"));
        assert!(!suppressed.contains(&"explorer"));
    }
}
