//! Agent thread implementation for separate thread execution.
//!
//! This module provides functions to spawn agent threads communicating via
//! channels with the UI thread.

use crate::agent::Agent;
use crate::config::Config;
use crate::messaging::{AgentToUi, UiToAgent};
use tokio::sync::mpsc;

/// Create a new agent thread with the given configuration and channels.
/// Spawns the agent task and returns a handle to send requests.
pub fn spawn(
    config: Config,
    ui_tx: mpsc::Sender<AgentToUi>,
) -> mpsc::Sender<UiToAgent> {
    let (agent_tx, agent_rx) = mpsc::channel(100);

    // Clone ui_tx for error reporting before moving into agent
    let ui_tx_clone = ui_tx.clone();
    tokio::spawn(async move {
        // Create agent with both channels
        let mut agent = match Agent::new_with_channels(config, ui_tx, agent_rx) {
            Ok(agent) => agent,
            Err(e) => {
                let _ = ui_tx_clone.send(AgentToUi::Error(e.to_string())).await;
                return;
            }
        };

        // Run agent; this will block until shutdown or channel closed
        if let Err(e) = agent.run().await {
            let _ = ui_tx_clone.send(AgentToUi::Error(e.to_string())).await;
        }
    });

    agent_tx
}

/// Spawn an agent thread with an existing Agent instance.
/// The agent's channel will be set to send updates to UI.
pub fn spawn_with_agent(
    mut agent: Agent,
    ui_tx: mpsc::Sender<AgentToUi>,
) -> mpsc::Sender<UiToAgent> {
    let (agent_tx, agent_rx) = mpsc::channel(100);
    
    // Set channels on agent
    agent.set_agent_to_ui_tx(ui_tx.clone());
    agent.set_ui_to_agent_rx(agent_rx);
    
    let ui_tx_clone = ui_tx.clone();
    tokio::spawn(async move {
        // Run agent; this will block until shutdown or channel closed
        if let Err(e) = agent.run().await {
            let _ = ui_tx_clone.send(AgentToUi::Error(e.to_string())).await;
        }
    });
    
    agent_tx
}