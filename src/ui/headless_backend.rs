use crate::agent::Agent;
use crate::agent_thread::spawn_with_agent;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use crate::ui::ui_trait::Ui;
use async_trait::async_trait;
use tokio::sync::mpsc;

const HEADLESS_SYSTEM_PROMPT: &str = "You are an AI coding assistant with access to tools. Use tools to accomplish tasks when appropriate. When the user asks to explore or analyze a codebase, use the explore tool.";

/// Headless UI backend that prints to stdout.
pub struct HeadlessUi {
    /// Shared channels for UI↔agent communication.
    channels: crate::ui::UiChannels,
    /// Optional initial prompt to send immediately.
    prompt: Option<String>,
}

impl HeadlessUi {
    /// Create a new headless UI backend.
    /// Creates channels and spawns agent thread with system prompt.
    pub fn new(config: Config, prompt: Option<String>) -> Result<Self> {
        // Create agent with system prompt
        let agent = Agent::new(config.clone())?;
        let agent = agent.with_system_prompt(Some(HEADLESS_SYSTEM_PROMPT.to_string()))?;

        // Create channel for agent->UI updates
        let (agent_to_ui_tx, agent_to_ui_rx) = mpsc::channel(100);

        // Spawn agent thread with existing agent
        let ui_to_agent_tx = spawn_with_agent(agent, agent_to_ui_tx);

        let channels = crate::ui::UiChannels {
            agent_to_ui_rx,
            ui_to_agent_tx,
        };

        Ok(Self { channels, prompt })
    }

    /// Read input from stdin if no prompt provided.
    fn read_input() -> Result<String> {
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();
        match lines.next() {
            Some(Ok(line)) => Ok(line),
            Some(Err(e)) => Err(crate::error::Error::Io(e)),
            None => Err(crate::error::Error::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No input provided",
            ))),
        }
    }
}

#[async_trait]
impl Ui for HeadlessUi {
    fn agent_rx(&mut self) -> &mut mpsc::Receiver<AgentToUi> {
        &mut self.channels.agent_to_ui_rx
    }

    fn agent_tx(&mut self) -> &mut mpsc::Sender<UiToAgent> {
        &mut self.channels.ui_to_agent_tx
    }

    async fn run(&mut self) -> Result<()> {
        // Determine input: use provided prompt or read from stdin
        let input = match self.prompt.take() {
            Some(p) => p,
            None => Self::read_input()?,
        };

        if input.trim().is_empty() {
            println!("No input provided. Exiting.");
            return Ok(());
        }

        // Send request to agent
        self.send_to_agent(UiToAgent::Request(input))
            .await
            .map_err(|e| crate::error::Error::Ai(format!("Failed to send request: {}", e)))?;

        // Collect and print responses
        let mut final_response = None;
        while let Some(msg) = self.recv_from_agent().await {
            match msg {
                AgentToUi::ToolCall(text) => {
                    println!("{}", text);
                }
                AgentToUi::ToolResult(text) => {
                    println!("{}", text);
                }
                AgentToUi::Response(text) => {
                    final_response = Some(text);
                    break;
                }
                AgentToUi::Error(text) => {
                    eprintln!("{}", text);
                    final_response = Some(text);
                    break;
                }
                AgentToUi::TokenUsage(usage) => {
                    println!(
                        "Tokens used: prompt: {}, completion: {}, total: {}",
                        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                    );
                }
            }
        }

        // Print final response if any
        if let Some(response) = final_response {
            println!("{}", response);
        }

        // Send shutdown signal
        let _ = self.send_to_agent(UiToAgent::Shutdown).await;

        Ok(())
    }
}
