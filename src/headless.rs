use crate::agent::Agent;
use crate::agent_thread::spawn_with_agent;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use tokio::sync::mpsc;

const HEADLESS_SYSTEM_PROMPT: &str = "You are an AI coding assistant with access to tools. Use tools to accomplish tasks when appropriate. When the user asks to explore or analyze a codebase, use the explore tool.";

pub async fn run(config: Config, prompt: Option<String>) -> Result<()> {
    let input = match prompt {
        Some(p) => p,
        None => read_input()?,
    };

    if input.trim().is_empty() {
        println!("No input provided. Exiting.");
        return Ok(());
    }

    // Create agent with system prompt
    let agent = Agent::new(config.clone())?;
    let agent = agent.with_system_prompt(Some(HEADLESS_SYSTEM_PROMPT.to_string()))?;

    // Create channel for agent->headless updates
    let (agent_to_ui_tx, mut agent_to_ui_rx) = mpsc::channel(100);
    
    // Spawn agent thread with existing agent
    let ui_to_agent_tx = spawn_with_agent(agent, agent_to_ui_tx);
    
    // Send request
    ui_to_agent_tx
        .send(UiToAgent::Request(input))
        .await
        .map_err(|e| crate::error::Error::Ai(format!("Failed to send request: {}", e)))?;
    
    // Collect and print responses
    let mut final_response = None;
    while let Some(msg) = agent_to_ui_rx.recv().await {
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
    
    // Send shutdown
    let _ = ui_to_agent_tx.send(UiToAgent::Shutdown).await;
    
    Ok(())
}

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
