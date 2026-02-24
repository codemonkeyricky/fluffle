use crate::agent::Agent;
use crate::config::Config;
use crate::error::Result;

pub struct App {
    pub agent: Agent,
    pub messages: Vec<String>,
    pub tool_output: String,
    pub input: String,
    pub should_quit: bool,
    pub status: StatusInfo,
}

pub struct StatusInfo {
    pub model: String,
    pub provider: String,
    pub plugins_loaded: usize,
}

impl App {
    pub async fn new() -> Result<Self> {
        let config = Config::load().await?;
        let agent = Agent::new(config.clone())?;

        let status = StatusInfo {
            model: config.model,
            provider: config.provider,
            plugins_loaded: agent.tools().len(),
        };

        Ok(Self {
            agent,
            messages: Vec::new(),
            tool_output: String::new(),
            input: String::new(),
            should_quit: false,
            status,
        })
    }

    pub async fn handle_input(&mut self) -> Result<()> {
        if self.input.trim().is_empty() {
            return Ok(());
        }

        // Add user message to display
        self.messages.push(format!("> {}", self.input));

        // Process through agent
        let response = self.agent.process(&self.input).await?;

        // Add response to display
        self.messages.push(response);

        // Clear input
        self.input.clear();

        Ok(())
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}