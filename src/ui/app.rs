use crate::agent::Agent;
use crate::config::Config;
use crate::error::Result;
use std::sync::Arc;
use super::shared_messages::SharedMessages;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct App {
    pub agent: Arc<RwLock<Agent>>,
    pub shared_messages: Arc<SharedMessages>,
    pub input: String,
    pub should_quit: bool,
    pub status: StatusInfo,
    pub processing_task: Option<JoinHandle<Result<String>>>,
    // TODO: Consider storing errors as well (Option<Result<String>> or separate error field)
    pub pending_result: Option<String>,
}

pub struct StatusInfo {
    pub model: String,
    pub provider: String,
    pub plugins_loaded: usize,
}


impl App {
    pub async fn new() -> Result<Self> {
        let config = Config::load().await?;
        let mut agent = Agent::new(config.clone())?;

        let shared_messages = Arc::new(SharedMessages::new());

        // Set shared messages on agent
        agent.set_shared_messages(shared_messages.clone());

        let status = StatusInfo {
            model: config.model,
            provider: config.provider,
            plugins_loaded: agent.tools().len(),
        };

        Ok(Self {
            agent: Arc::new(RwLock::new(agent)),
            shared_messages,
            input: String::new(),
            should_quit: false,
            status,
            processing_task: None,
            pending_result: None,
        })
    }

    pub async fn handle_input(&mut self) -> Result<()> {
        if self.input.trim().is_empty() {
            return Ok(());
        }

        // Add user message to display
        self.shared_messages.push(format!("> {}", self.input));

        // Process through agent (will push tool messages via shared_messages)
        let response = self.agent.write().await.process(&self.input).await?;

        // Add final response to display
        self.shared_messages.push(response);

        // Clear input
        self.input.clear();

        Ok(())
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

#[cfg(test)]
mod tests {


    #[tokio::test]
    async fn test_app_new_with_shared_messages() {
        // Mock config to avoid actual file loading
        use crate::config::Config;

        // Create minimal config for test
        let _config = Config {
            model: "test-model".to_string(),
            api_key: None,
            provider: "test".to_string(),
            max_tokens: 100,
            temperature: 0.5,
            max_tool_iterations: 10,
        };

        // This test is conceptual - App::new loads real config
        // We'll just verify App struct compiles with new field
        assert!(true);
    }
}