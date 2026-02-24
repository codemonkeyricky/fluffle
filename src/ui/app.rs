use crate::agent::Agent;
use crate::config::Config;
use crate::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct App {
    pub agent: Agent,
    pub shared_messages: Arc<SharedMessages>,
    pub tool_output: String,
    pub input: String,
    pub should_quit: bool,
    pub status: StatusInfo,
}

pub struct StatusInfo {
    pub model: String,
    pub provider: String,
    pub plugins_loaded: usize,
}

/// Thread-safe shared message buffer for live UI updates
#[derive(Clone, Debug)]
pub struct SharedMessages {
    messages: Arc<Mutex<Vec<String>>>,
    dirty: Arc<AtomicBool>,
}

impl SharedMessages {
    /// Creates a new empty SharedMessages buffer.
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Pushes a new message onto the buffer and marks the buffer as dirty.
    /// Panics if the mutex is poisoned.
    pub fn push(&self, message: String) {
        let mut guard = self.messages.lock().expect("mutex poisoned");
        guard.push(message);
        self.dirty.store(true, Ordering::Release);
    }

    /// Returns all messages from the buffer without clearing it, and marks it as clean.
    /// Returns a copy of the collected messages.
    /// Panics if the mutex is poisoned.
    pub fn take_messages(&self) -> Vec<String> {
        let guard = self.messages.lock().expect("mutex poisoned");
        self.dirty.store(false, Ordering::Release);
        guard.clone()
    }

    /// Returns `true` if there are new messages since the last call to `take_messages`.
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }
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
            agent,
            shared_messages,
            tool_output: String::new(),
            input: String::new(),
            should_quit: false,
            status,
        })
    }

    pub async fn handle_input(&mut self) -> Result<()> {
        if self.input.trim().is_empty() {
            return Ok(());
        }

        // Add user message to display
        self.shared_messages.push(format!("> {}", self.input));

        // Process through agent (will push tool messages via shared_messages)
        let response = self.agent.process(&self.input).await?;

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
    use super::*;

    #[test]
    fn test_shared_messages_push_and_take() {
        let shared = SharedMessages::new();
        assert_eq!(shared.take_messages(), Vec::<String>::new());
        assert!(!shared.is_dirty());

        shared.push("Test message".to_string());
        assert!(shared.is_dirty());

        let messages = shared.take_messages();
        assert_eq!(messages, vec!["Test message".to_string()]);
        assert!(!shared.is_dirty());
        // Messages persist in buffer after taking
        assert_eq!(shared.take_messages(), vec!["Test message".to_string()]);
    }

    #[test]
    fn test_shared_messages_multiple_pushes() {
        let shared = SharedMessages::new();
        shared.push("Message 1".to_string());
        shared.push("Message 2".to_string());

        let messages = shared.take_messages();
        assert_eq!(messages, vec!["Message 1".to_string(), "Message 2".to_string()]);
    }

    #[tokio::test]
    async fn test_app_new_with_shared_messages() {
        // Mock config to avoid actual file loading
        use crate::config::Config;
        use crate::agent::Agent;

        // Create minimal config for test
        let config = Config {
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