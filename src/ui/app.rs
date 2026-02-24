use crate::agent::Agent;
use crate::config::Config;
use crate::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct App {
    pub agent: Agent,
    pub messages: Vec<String>,
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

    /// Takes all messages from the buffer, clearing it and marking it as clean.
    /// Returns the collected messages.
    /// Panics if the mutex is poisoned.
    pub fn take_messages(&self) -> Vec<String> {
        let mut guard = self.messages.lock().expect("mutex poisoned");
        let messages = std::mem::take(&mut *guard);
        self.dirty.store(false, Ordering::Release);
        messages
    }

    /// Returns `true` if there are new messages since the last call to `take_messages`.
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }
}

impl App {
    pub async fn new() -> Result<Self> {
        let config = Config::load().await?;
        let agent = Agent::new(config.clone())?;

        let status = StatusInfo {
            model: config.model,
            provider: config.provider,
            plugins_loaded: agent.tools().len(),
        };

        Ok(Self {
            agent,
            messages: Vec::new(),
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
        self.messages.push(format!("> {}", self.input));

        // Process through agent
        let response = self.agent.process(&self.input).await?;

        // Add response to display
        self.messages.push(response);

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
        assert_eq!(shared.take_messages(), Vec::<String>::new());
    }

    #[test]
    fn test_shared_messages_multiple_pushes() {
        let shared = SharedMessages::new();
        shared.push("Message 1".to_string());
        shared.push("Message 2".to_string());

        let messages = shared.take_messages();
        assert_eq!(messages, vec!["Message 1".to_string(), "Message 2".to_string()]);
    }
}