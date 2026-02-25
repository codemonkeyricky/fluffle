use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
        assert_eq!(
            messages,
            vec!["Message 1".to_string(), "Message 2".to_string()]
        );
    }
}
