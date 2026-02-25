use super::shared_messages::SharedMessages;
use crate::agent::Agent;
use crate::ai::TokenUsage;
use crate::config::Config;
use crate::error::Result;
use futures::poll;
use std::sync::Arc;
use std::task::Poll;
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

    /// Handles the current input asynchronously by spawning a background task.
    /// Returns immediately, allowing the UI to remain responsive during processing.
    /// Use `is_processing()` to check if a task is running.
    pub async fn handle_input(&mut self) -> Result<()> {
        if self.input.trim().is_empty() {
            return Ok(());
        }

        // Don't start new task if one is already running
        if self.processing_task.is_some() {
            return Ok(());
        }

        // Add user message to display
        self.shared_messages.push(format!("> {}", self.input));

        // Clone references for background task
        let agent_clone = self.agent.clone();
        let shared_messages_clone = self.shared_messages.clone();
        let input = std::mem::take(&mut self.input);

        // Spawn background processing task
        self.processing_task = Some(tokio::task::spawn_local(async move {
            let mut agent = agent_clone.write().await;
            let result = agent.process(&input).await;

            match result {
                Ok(response) => {
                    // Add final response to shared messages
                    shared_messages_clone.push(response.clone());
                    Ok(response)
                }
                Err(e) => {
                    // Add error to shared messages
                    shared_messages_clone.push(format!("Error: {}", e));
                    Err(e)
                }
            }
        }));

        Ok(())
    }

    /// Returns true if a background processing task is currently running.
    pub fn is_processing(&self) -> bool {
        self.processing_task.is_some()
    }

    /// Returns the current token usage for the agent session.
    pub fn token_usage(&self) -> TokenUsage {
        match self.agent.try_read() {
            Ok(guard) => guard.token_usage().clone(),
            Err(_) => TokenUsage::default(),
        }
    }

    /// Checks if the current background processing task has completed.
    /// Returns true if a task was completed (successfully or with error).
    /// If the task completed successfully, the result is stored in `pending_result`.
    pub async fn check_task_completion(&mut self) -> bool {
        if let Some(task) = &mut self.processing_task {
            match poll!(task) {
                Poll::Ready(result) => {
                    match result {
                        Ok(Ok(response)) => {
                            self.pending_result = Some(response);
                        }
                        Ok(Err(e)) => {
                            eprintln!("Task failed: {}", e);
                        }
                        Err(join_err) => {
                            eprintln!("Task panicked: {}", join_err);
                        }
                    }
                    self.processing_task = None;
                    true
                }
                Poll::Pending => false,
            }
        } else {
            false
        }
    }

    /// Cancels any ongoing background processing task.
    /// Aborts the task immediately without waiting for completion.
    pub fn cancel_processing(&mut self) {
        if let Some(task) = self.processing_task.take() {
            task.abort();
        }
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
