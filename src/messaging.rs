//! Messaging system for bidirectional communication between UI and agent threads.
//!
//! This module defines the message types sent between the UI thread and the
//! agent thread, enabling clean separation and queuing of requests/responses.

use crate::ai::TokenUsage;

/// Messages sent from UI to agent thread.
#[derive(Debug)]
pub enum UiToAgent {
    /// User input to process.
    Request(String),
    /// Request graceful shutdown of agent thread.
    Shutdown,
}

/// Messages sent from agent thread to UI.
#[derive(Debug)]
pub enum AgentToUi {
    /// Tool call log for display.
    ToolCall(String),
    /// Tool result log for display.
    ToolResult(String),
    /// Final response from agent.
    Response(String),
    /// Processing error.
    Error(String),
    /// Token usage statistics.
    TokenUsage(TokenUsage),
    /// Thinking/reasoning output from agent.
    Thinking(String),
}
