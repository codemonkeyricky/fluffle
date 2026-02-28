//! Messaging system for bidirectional communication between UI and agent threads.
//!
//! This module defines the message types sent between the UI thread and the
//! agent thread, enabling clean separation and queuing of requests/responses.

use crate::ai::TokenUsage;
use tokio::sync::oneshot;

/// Messages sent from UI to agent thread.
#[derive(Debug)]
pub enum UiToAgent {
    /// User input to process.
    Request(String),
    /// Request graceful shutdown of agent thread.
    Shutdown,
    /// Result from a child agent that has completed.
    ChildResult {
        /// Whether the child agent succeeded.
        success: bool,
        /// Output from the child agent (or error message).
        output: String,
        /// Detailed error information if any.
        error: Option<String>,
    },
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
    /// Request to spawn a child agent.
    SpawnChild {
        /// Agent profile name (e.g., "generalist", "explorer").
        name: String,
        /// Task description for the child agent.
        description: String,
        /// Optional custom system prompt for the child agent.
        system_prompt: Option<String>,
        /// Channel to send the child's result back to the parent agent.
        result_tx: oneshot::Sender<crate::types::ToolResult>,
    },
}
