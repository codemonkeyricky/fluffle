//! Agent stack management for nested agent invocation.
//!
//! This module provides a stack-based manager for handling multiple active agents
//! where only the topmost agent receives UI events and sends responses.

use crate::messaging::{AgentToUi, UiToAgent};
use crate::types::ToolResult;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, oneshot};

/// Global counter for assigning unique context IDs (CIDs) to agents.
pub(crate) static NEXT_CID: AtomicU64 = AtomicU64::new(1);

/// Handle to an active agent with its communication channels and metadata.
pub struct AgentHandle {
    /// Agent profile name (e.g., "generalist", "explorer").
    pub name: String,
    /// Context ID (CID) – unique numeric identifier for this agent instance.
    pub cid: u64,
    /// Sender for UI-to-agent messages (UI uses this to send events to agent).
    pub ui_to_agent_tx: mpsc::Sender<UiToAgent>,
    /// Receiver for agent-to-UI messages (UI receives agent responses from this).
    pub agent_to_ui_rx: mpsc::Receiver<AgentToUi>,
    /// Optional sender for returning child result to parent agent.
    /// When this agent is a child spawned by a parent, this channel is used
    /// to send the final result back to the parent when the child completes.
    pub child_result_tx: Option<oneshot::Sender<ToolResult>>,
}

impl AgentHandle {
    /// Create a new agent handle with the given name and channels.
    pub fn new(
        name: String,
        ui_to_agent_tx: mpsc::Sender<UiToAgent>,
        agent_to_ui_rx: mpsc::Receiver<AgentToUi>,
        cid: Option<u64>,
    ) -> Self {
        let cid = cid.unwrap_or_else(|| NEXT_CID.fetch_add(1, Ordering::Relaxed));
        Self {
            name,
            cid,
            ui_to_agent_tx,
            agent_to_ui_rx,
            child_result_tx: None,
        }
    }

    /// Set the child result sender for returning results to parent.
    pub fn set_child_result_tx(&mut self, tx: oneshot::Sender<ToolResult>) {
        self.child_result_tx = Some(tx);
    }

    /// Take the child result sender, leaving None in its place.
    pub fn take_child_result_tx(&mut self) -> Option<oneshot::Sender<ToolResult>> {
        self.child_result_tx.take()
    }
}

/// Stack manager for active agents.
/// Maintains a stack of `AgentHandle` instances where the topmost agent
/// is the one currently receiving UI events and sending responses.
pub struct AgentStack {
    /// Stack of agent handles, with the topmost being the currently active agent.
    stack: Vec<AgentHandle>,
}

impl AgentStack {
    /// Create a new agent stack with a base agent.
    pub fn new(
        base_agent_name: String,
        base_tx: mpsc::Sender<UiToAgent>,
        base_rx: mpsc::Receiver<AgentToUi>,
        cid: Option<u64>,
    ) -> Self {
        let base_handle = AgentHandle::new(base_agent_name, base_tx, base_rx, cid);
        Self {
            stack: vec![base_handle],
        }
    }

    /// Push a new agent onto the stack, making it the active agent.
    /// Returns a oneshot receiver that the parent can use to await the child's result.
    pub fn push(
        &mut self,
        name: String,
        tx: mpsc::Sender<UiToAgent>,
        rx: mpsc::Receiver<AgentToUi>,
        cid: Option<u64>,
    ) -> oneshot::Receiver<ToolResult> {
        let (result_tx, result_rx) = oneshot::channel();
        let mut handle = AgentHandle::new(name, tx, rx, cid);
        handle.set_child_result_tx(result_tx);
        self.stack.push(handle);
        result_rx
    }

    /// Push a new agent onto the stack with a pre-existing result channel.
    /// The result_tx will be used to send the child's result back to the parent.
    /// No receiver is returned because the caller already has the sender.
    pub fn push_with_result_tx(
        &mut self,
        name: String,
        tx: mpsc::Sender<UiToAgent>,
        rx: mpsc::Receiver<AgentToUi>,
        result_tx: oneshot::Sender<ToolResult>,
        cid: Option<u64>,
    ) {
        let mut handle = AgentHandle::new(name, tx, rx, cid);
        handle.set_child_result_tx(result_tx);
        self.stack.push(handle);
    }

    /// Pop the top agent from the stack, returning it to the previous agent.
    /// If a result is provided, it will be sent to the parent via the child result channel.
    /// Returns the popped agent handle (if any) for cleanup.
    pub fn pop(&mut self, result: Option<ToolResult>) -> Option<AgentHandle> {
        let mut popped = self.stack.pop()?;

        // If we have a result and there's a parent to receive it, send it
        if let Some(result) = result {
            if let Some(result_tx) = popped.take_child_result_tx() {
                let _ = result_tx.send(result); // Ignore errors if parent dropped
            }
        }

        Some(popped)
    }

    /// Get a reference to the sender for the currently active agent.
    pub fn current_tx(&self) -> Option<&mpsc::Sender<UiToAgent>> {
        self.stack.last().map(|handle| &handle.ui_to_agent_tx)
    }

    /// Get a mutable reference to the sender for the currently active agent.
    pub fn current_tx_mut(&mut self) -> Option<&mut mpsc::Sender<UiToAgent>> {
        self.stack
            .last_mut()
            .map(|handle| &mut handle.ui_to_agent_tx)
    }

    /// Get a mutable reference to the receiver for the currently active agent.
    pub fn current_rx(&mut self) -> Option<&mut mpsc::Receiver<AgentToUi>> {
        self.stack
            .last_mut()
            .map(|handle| &mut handle.agent_to_ui_rx)
    }

    /// Get the name of the currently active agent.
    pub fn current_name(&self) -> Option<&str> {
        self.stack.last().map(|handle| handle.name.as_str())
    }

    /// Check if the stack is empty (should never happen in normal operation).
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get the number of agents in the stack.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Get the stack representation as a string with " -> " separator.
    pub fn stack_display(&self) -> String {
        self.stack
            .iter()
            .map(|handle| format!("{} (cid {})", handle.name, handle.cid))
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// Get a reference to the base agent (bottom of stack).
    pub fn base_handle(&self) -> Option<&AgentHandle> {
        self.stack.first()
    }

    /// Check if the current agent is the base agent.
    pub fn is_base_agent_active(&self) -> bool {
        self.stack.len() == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::{AgentToUi, UiToAgent};
    use tokio::sync::mpsc;

    #[test]
    fn test_stack_display() {
        NEXT_CID.store(1, Ordering::Relaxed);
        let (tx1, _) = mpsc::channel::<UiToAgent>(1);
        let (_, rx1) = mpsc::channel::<AgentToUi>(1);
        let mut stack = AgentStack::new("generalist".to_string(), tx1, rx1, None);
        assert_eq!(stack.stack_display(), "generalist (cid 1)");

        let (tx2, _) = mpsc::channel::<UiToAgent>(1);
        let (_, rx2) = mpsc::channel::<AgentToUi>(1);
        let _rx = stack.push("explorer".to_string(), tx2, rx2, None);
        assert_eq!(
            stack.stack_display(),
            "generalist (cid 1) -> explorer (cid 2)"
        );

        let (tx3, _) = mpsc::channel::<UiToAgent>(1);
        let (_, rx3) = mpsc::channel::<AgentToUi>(1);
        let _rx = stack.push("specialist".to_string(), tx3, rx3, None);
        assert_eq!(
            stack.stack_display(),
            "generalist (cid 1) -> explorer (cid 2) -> specialist (cid 3)"
        );

        stack.pop(None);
        assert_eq!(
            stack.stack_display(),
            "generalist (cid 1) -> explorer (cid 2)"
        );

        stack.pop(None);
        assert_eq!(stack.stack_display(), "generalist (cid 1)");
    }
}
