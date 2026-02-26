//! Shared channel pair for UI↔agent communication.

use crate::messaging::{AgentToUi, UiToAgent};
use tokio::sync::mpsc;

/// Shared channel pair for UI↔agent communication.
pub struct UiChannels {
    /// Receiver for messages from agent thread.
    pub agent_to_ui_rx: mpsc::Receiver<AgentToUi>,
    /// Sender for requests to agent thread.
    pub ui_to_agent_tx: mpsc::Sender<UiToAgent>,
}

impl UiChannels {
    /// Create a new channel pair with given buffer size.
    /// Returns the sender for agent→UI messages and the channels struct.
    pub fn new(buffer: usize) -> (mpsc::Sender<AgentToUi>, Self) {
        let (agent_to_ui_tx, agent_to_ui_rx) = mpsc::channel(buffer);
        let (ui_to_agent_tx, _ui_to_agent_rx) = mpsc::channel(buffer);
        // Note: ui_to_agent_rx is passed to agent thread elsewhere
        let channels = Self {
            agent_to_ui_rx,
            ui_to_agent_tx,
        };
        (agent_to_ui_tx, channels)
    }
}
