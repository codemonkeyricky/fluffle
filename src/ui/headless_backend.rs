use crate::agent::Agent;
use crate::agent_thread::spawn_with_agent;
use crate::app_name;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use crate::types::ToolResult;
use crate::ui::ui_trait::Ui;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc;

const HEADLESS_SYSTEM_PROMPT: &str = "You are an AI coding assistant with access to tools. Use tools to accomplish tasks when appropriate. When the user asks to explore or analyze a codebase, use the explorer tool.";

/// Headless UI backend that prints to stdout.
pub struct HeadlessUi {
    /// Shared channels for UI↔agent communication.
    channels: crate::ui::UiChannels,
    /// Configuration for spawning child agents.
    config: Config,
    /// Optional initial prompt to send immediately.
    prompt: Option<String>,
    /// Working directory for tool execution.
    workdir: Option<PathBuf>,
}

impl HeadlessUi {
    /// Create a new headless UI backend.
    /// Creates channels and spawns agent thread with system prompt.
    pub fn new(config: Config, prompt: Option<String>, workdir: Option<PathBuf>) -> Result<Self> {

        // Determine profile based on app
        let profile_name = if app_name::get_app_name() == "plan-builder" {
            "task-agent".to_string()
        } else {
            "generalist".to_string()
        };

        // Try to create agent with profile, fall back to generic agent
        let agent = match Agent::new_with_profile(&profile_name, config.clone(), workdir.clone()) {
            Ok(agent) => agent,
            Err(_) => {
                // Profile not found, fall back to generic agent with default system prompt
                let agent = Agent::new(config.clone(), workdir.clone())?;
                agent.with_system_prompt(Some(HEADLESS_SYSTEM_PROMPT.to_string()))?
            }
        };

        // Create channel for agent->UI updates
        let (agent_to_ui_tx, agent_to_ui_rx) = mpsc::channel(100);

        // Spawn agent thread with existing agent
        let ui_to_agent_tx = spawn_with_agent(agent, agent_to_ui_tx);

        let channels = crate::ui::UiChannels {
            agent_to_ui_rx,
            ui_to_agent_tx,
        };

        Ok(Self {
            channels,
            config,
            prompt,
            workdir,
        })
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

    /// Spawn a child agent inline (headless mode).
    async fn spawn_child_inline(
        &self,
        name: String,
        description: String,
        system_prompt: Option<String>,
    ) -> ToolResult {
        // Try to create agent with profile first
        let mut agent = match Agent::new_with_profile(&name, self.config.clone(), self.workdir.clone()) {
            Ok(agent) => agent,
            Err(_) => {
                // Fall back to generic agent
                match Agent::new(self.config.clone(), self.workdir.clone()) {
                    Ok(agent) => agent,
                    Err(e) => return ToolResult::error(format!("Failed to create agent: {}", e)),
                }
            }
        };

        // Apply custom system prompt if provided (overrides profile)
        if let Some(prompt) = system_prompt {
            match agent.with_system_prompt(Some(prompt)) {
                Ok(subagent) => agent = subagent,
                Err(e) => return ToolResult::error(format!("Failed to set system prompt: {}", e)),
            }
        }

        // Run the task
        match agent.process(&description).await {
            Ok(summary) => ToolResult::success(summary),
            Err(e) => ToolResult::error(format!("Child agent failed: {}", e)),
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
            tracing::debug!("Headless UI received message: {:?}", msg);
            match msg {
                AgentToUi::ToolCall(text) => {
                    println!("\x1b[90m{}\x1b[0m", text);
                }
                AgentToUi::ToolResult(text) => {
                    println!("\x1b[90m{}\x1b[0m", text);
                }
                AgentToUi::Thinking(text) => {
                    println!("\x1b[90mThinking: {}\x1b[0m", text);
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
                AgentToUi::SpawnChild {
                    name,
                    description,
                    system_prompt,
                    result_tx,
                } => {
                    println!(
                        "\x1b[90mSpawning child agent {}: {}\x1b[0m",
                        name, description
                    );
                    // Inline execution for headless mode
                    let result = self
                        .spawn_child_inline(name, description, system_prompt)
                        .await;
                    let _ = result_tx.send(result);
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
