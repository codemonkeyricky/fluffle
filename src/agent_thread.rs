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
    let (agent_tx, mut agent_rx) = mpsc::channel(100);

    tokio::spawn(async move {
        // Create agent with channel for updates
        let mut agent = match Agent::new_with_channel(config, ui_tx.clone()) {
            Ok(agent) => agent,
            Err(e) => {
                let _ = ui_tx.send(AgentToUi::Error(e.to_string())).await;
                return;
            }
        };

        while let Some(request) = agent_rx.recv().await {
            match request {
                UiToAgent::Request(input) => {
                    // Process the request
                    match agent.process(&input).await {
                        Ok(response) => {
                            let _ = ui_tx.send(AgentToUi::Response(response)).await;
                            let _ = ui_tx.send(AgentToUi::TokenUsage(agent.token_usage().clone())).await;
                        }
                        Err(e) => {
                            let _ = ui_tx.send(AgentToUi::Error(e.to_string())).await;
                        }
                    }
                }
                UiToAgent::Shutdown => {
                    // Graceful shutdown
                    break;
                }
            }
        }
        // Channel closed, UI disconnected
    });

    agent_tx
}

/// Spawn an agent thread with an existing Agent instance.
/// The agent's channel will be set to send updates to UI.
pub fn spawn_with_agent(
    mut agent: Agent,
    ui_tx: mpsc::Sender<AgentToUi>,
) -> mpsc::Sender<UiToAgent> {
    let (agent_tx, mut agent_rx) = mpsc::channel(100);
    
    // Set channel on agent
    agent.set_channel_tx(ui_tx.clone());
    
    tokio::spawn(async move {
        while let Some(request) = agent_rx.recv().await {
            match request {
                UiToAgent::Request(input) => {
                    match agent.process(&input).await {
                        Ok(response) => {
                            let _ = ui_tx.send(AgentToUi::Response(response)).await;
                            let _ = ui_tx.send(AgentToUi::TokenUsage(agent.token_usage().clone())).await;
                        }
                        Err(e) => {
                            let _ = ui_tx.send(AgentToUi::Error(e.to_string())).await;
                        }
                    }
                }
                UiToAgent::Shutdown => {
                    break;
                }
            }
        }
    });
    
    agent_tx
}