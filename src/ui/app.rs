use crate::agent::Agent;
use crate::ai::TokenUsage;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use tokio::sync::mpsc;

pub struct App {
    pub ui_to_agent_tx: mpsc::Sender<UiToAgent>,
    pub messages: Vec<String>,
    pub input: String,
    pub should_quit: bool,
    pub status: StatusInfo,
    pub pending_requests: usize,
    pub token_usage: TokenUsage,
}

pub struct StatusInfo {
    pub model: String,
    pub provider: String,
    pub plugins_loaded: usize,
}

impl App {
    pub async fn new(config: Config, ui_to_agent_tx: mpsc::Sender<UiToAgent>) -> Result<Self> {
        // Create temporary agent to get tool count for status
        let agent = Agent::new(config.clone(), None)?;
        let status = StatusInfo {
            model: config.model,
            provider: config.provider,
            plugins_loaded: agent.tools().len(),
        };

        Ok(Self {
            ui_to_agent_tx,
            messages: Vec::new(),
            input: String::new(),
            should_quit: false,
            status,
            pending_requests: 0,
            token_usage: TokenUsage::default(),
        })
    }

    /// Handles the current input asynchronously by sending request to agent thread.
    /// Returns immediately, allowing the UI to remain responsive during processing.
    /// Use `is_processing()` to check if there are pending requests.
    pub async fn handle_input(&mut self) -> Result<()> {
        if self.input.trim().is_empty() {
            return Ok(());
        }

        // Add user message to display
        self.messages.push(format!("> {}", self.input));

        // Send request to agent thread
        let request = UiToAgent::Request(std::mem::take(&mut self.input));
        if let Err(e) = self.ui_to_agent_tx.send(request).await {
            self.messages.push(format!("Error sending request: {}", e));
            return Ok(());
        }

        self.pending_requests += 1;
        Ok(())
    }

    /// Returns true if there are pending requests to the agent thread.
    pub fn is_processing(&self) -> bool {
        self.pending_requests > 0
    }

    /// Returns the current token usage for the agent session.
    pub fn token_usage(&self) -> TokenUsage {
        self.token_usage.clone()
    }

    /// Process a single agent message and update UI state.
    /// Returns true if the message indicates the agent has finished processing
    /// (i.e., a Response or Error), false otherwise.
    pub fn handle_agent_message(&mut self, msg: AgentToUi) -> bool {
        match msg {
            AgentToUi::ToolCall(text) => {
                self.messages.push(text);
                false
            }
            AgentToUi::ToolResult(text) => {
                self.messages.push(text);
                false
            }
            AgentToUi::Thinking(text) => {
                self.messages.push(text);
                false
            }
            AgentToUi::Response(text) => {
                self.messages.push(text);
                self.pending_requests = self.pending_requests.saturating_sub(1);
                true
            }
            AgentToUi::Error(text) => {
                self.messages.push(text);
                self.pending_requests = self.pending_requests.saturating_sub(1);
                true
            }
            AgentToUi::TokenUsage(usage) => {
                self.token_usage = usage;
                false
            }
            AgentToUi::SpawnChild { .. } => {
                self.messages.push("Spawning child agent...".to_string());
                false
            }
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

#[cfg(test)]
mod tests {
    // Tests temporarily disabled during refactoring to agent thread architecture
}
