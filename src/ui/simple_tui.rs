use crate::agent_thread::spawn_with_profile;
use crate::ai::TokenUsage;
use crate::app_name;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use crate::types::ToolResult;
use crate::ui::agent_stack::{AgentStack, NEXT_CID};
use crate::ui::bottom_pane::BottomPane;
use crate::ui::event::{Event, EventHandler};
use crate::ui::render::Renderable;
use crate::ui::ui_trait::Ui;
use async_trait::async_trait;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tokio::sync::{mpsc, oneshot};

/// Guard that ensures terminal state is restored when dropped.
struct TerminalGuard {
    terminal: Option<Terminal<CrosstermBackend<io::Stdout>>>,
}

impl TerminalGuard {
    /// Setup terminal and return guard that will restore on drop.
    fn setup() -> Result<Self> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();
        match execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            Ok(_) => {}
            Err(e) => {
                let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
                let _ = disable_raw_mode();
                return Err(e.into());
            }
        }

        let backend = CrosstermBackend::new(stdout);

        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(e) => {
                let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
                let _ = disable_raw_mode();
                return Err(e.into());
            }
        };

        Ok(Self {
            terminal: Some(terminal),
        })
    }

    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        self.terminal.as_mut().expect("terminal should be present")
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        if let Some(terminal) = &mut self.terminal {
            let _ = execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
            let _ = terminal.show_cursor();
        } else {
            let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        }
    }
}

/// Simple terminal UI backend that shows a log of agent messages and an input line.
pub struct SimpleTui {
    /// Terminal guard for cleanup.
    guard: TerminalGuard,
    /// Agent stack for managing nested agents.
    stack: AgentStack,
    /// Configuration for spawning child agents.
    config: Config,
    /// Working directory for tool execution.
    workdir: Option<PathBuf>,
    /// Event handler for user input.
    event_handler: EventHandler,
    /// Bottom pane with input composer.
    bottom_pane: BottomPane,
    /// Log lines to display (styled text).
    log_lines: Vec<Line<'static>>,
    /// Current scroll offset (lines from bottom).
    scroll_offset: usize,
    /// Whether to auto-scroll to bottom when new messages arrive.
    auto_scroll: bool,
    /// Current viewport height for log area (lines).
    viewport_height: u16,
    /// Model name.
    model: String,
    /// Provider name.
    provider: String,
    /// Token usage statistics.
    token_usage: TokenUsage,
    /// Agent stack display string (e.g., "generalist -> explorer").
    /// Computed from stack.
    agent_type: String,
}

impl SimpleTui {
    /// Create a new simple terminal UI backend.
    /// Sets up terminal, creates channels, spawns agent thread.
    pub async fn new(config: Config, workdir: Option<PathBuf>) -> Result<Self> {
        // Setup terminal
        let guard = TerminalGuard::setup()?;

        // Create channel for agent->UI updates
        let (agent_to_ui_tx, agent_to_ui_rx) = mpsc::channel(100);

        // Determine default profile based on app
        let default_profile = "dispatcher".to_string();

        // Clone config for spawning agent thread (spawn takes ownership)
        let config_clone = config.clone();
        // Generate unique CID for base agent
        let cid = NEXT_CID.fetch_add(1, Ordering::Relaxed);
        let ui_to_agent_tx = spawn_with_profile(
            config_clone,
            agent_to_ui_tx,
            workdir.clone(),
            Some(default_profile.clone()),
            Some(cid),
        );
        let ui_to_agent_tx_clone = ui_to_agent_tx.clone();

        // Create agent stack with base agent
        let stack = AgentStack::new(
            default_profile,
            ui_to_agent_tx_clone,
            agent_to_ui_rx,
            Some(cid),
        );
        let agent_type = stack.stack_display();

        let event_handler = EventHandler::new(250);
        let bottom_pane = BottomPane::new();

        // Extract model and provider before moving config into struct
        let model = config.model.clone();
        let provider = config.provider.clone();

        Ok(Self {
            guard,
            stack,
            config,
            workdir,
            event_handler,
            bottom_pane,
            log_lines: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            viewport_height: 0,
            model,
            provider,
            token_usage: TokenUsage::default(),
            agent_type,
        })
    }

    /// Convert an agent message to styled lines and add them to log.
    fn push_agent_message(&mut self, msg: AgentToUi) {
        let (prefix, color) = match &msg {
            AgentToUi::ToolCall(_) => ("", Color::Gray),
            AgentToUi::ToolResult(text) => {
                // Color tool results red if they start with "Error:" prefix
                if text.starts_with("Error:") {
                    ("", Color::Red)
                } else {
                    ("", Color::Gray)
                }
            },
            AgentToUi::Thinking(_) => ("Thinking: ", Color::Blue),
            AgentToUi::Response(_) => ("", Color::Green),
            AgentToUi::Error(_) => ("", Color::Red),
            AgentToUi::SpawnChild { .. } => ("Spawning child agent: ", Color::Cyan),
            AgentToUi::TokenUsage(usage) => {
                self.token_usage = usage.clone();
                let text = format!(
                    "Tokens used: prompt: {}, completion: {}, total: {}",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
                self.push_log(self.indented_line(vec![Span::raw(text)]));
                return;
            }
        };
        let text = match msg {
            AgentToUi::ToolCall(text) => text,
            AgentToUi::ToolResult(text) => text,
            AgentToUi::Thinking(text) => text,
            AgentToUi::Response(text) => text,
            AgentToUi::Error(text) => text,
            AgentToUi::SpawnChild {
                name,
                description,
                system_prompt,
                result_tx: _,
            } => {
                let prompt_info =
                    system_prompt.map_or_else(|| "".to_string(), |p| format!(", prompt: {}", p));
                format!("{}: {}{}", name, description, prompt_info)
            }
            AgentToUi::TokenUsage(_) => unreachable!(),
        };
        // Split by newlines, add prefix only to first line.
        let lines: Vec<&str> = text.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            let spans = if prefix.is_empty() || i > 0 {
                vec![Span::styled(line.to_string(), Style::default().fg(color))]
            } else {
                vec![
                    Span::styled(prefix, Style::default().fg(color)),
                    Span::raw(line.to_string()),
                ]
            };
            self.push_log(self.indented_line(spans));
        }
    }

    /// Add a new log line.
    fn push_log(&mut self, line: Line<'static>) {
        self.log_lines.push(line);
        // Keep last N lines? For now keep all.
        // Auto-scroll to bottom if auto_scroll is enabled.
        if self.auto_scroll {
            self.scroll_offset = 0;
        }
    }

    /// Clamp scroll offset based on current log lines and viewport height.
    /// Called before rendering to ensure offset is valid.
    fn clamp_scroll_offset_with_height(&mut self, viewport_height: usize) {
        let total_lines = self.log_lines.len();
        if total_lines <= viewport_height {
            self.scroll_offset = 0;
            return;
        }
        if self.auto_scroll {
            self.scroll_offset = 0;
            return;
        }
        let max_offset = total_lines - viewport_height;
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
    }

    /// Clamp scroll offset using current viewport height.
    fn clamp_scroll_offset(&mut self) {
        self.clamp_scroll_offset_with_height(self.viewport_height as usize);
    }

    /// Create a line with indentation based on current stack depth.
    fn indented_line(&self, spans: Vec<Span<'static>>) -> Line<'static> {
        let indent = "  ".repeat(self.stack.len().saturating_sub(1));
        let mut all_spans = Vec::new();
        if !indent.is_empty() {
            all_spans.push(Span::raw(indent));
        }
        all_spans.extend(spans);
        Line::from(all_spans)
    }
}

#[async_trait]
impl Ui for SimpleTui {
    fn agent_rx(&mut self) -> &mut mpsc::Receiver<AgentToUi> {
        self.stack.current_rx().expect("agent stack is empty")
    }

    fn agent_tx(&mut self) -> &mut mpsc::Sender<UiToAgent> {
        self.stack.current_tx_mut().expect("agent stack is empty")
    }

    fn current_agent_name(&self) -> Option<&str> {
        self.stack.current_name()
    }

    async fn next_user_event(&mut self) -> Option<Event> {
        self.event_handler.next().await
    }

    async fn run(&mut self) -> Result<()> {
        loop {
            let terminal_size = {
                let terminal = self.guard.terminal_mut();
                terminal.size()?
            };

            // Compute layout: status line at bottom, agent status above input, input pane above that, log area fills the rest.
            let status_height = 1;
            let agent_status_height = 1;
            let bottom_pane_height = self.bottom_pane.required_height(terminal_size.width);
            let max_bottom_height = terminal_size
                .height
                .saturating_sub(status_height + agent_status_height);
            let bottom_pane_height = bottom_pane_height.min(max_bottom_height);
            let available_for_log = terminal_size
                .height
                .saturating_sub(bottom_pane_height + agent_status_height + status_height);
            let log_height = available_for_log.max(1);
            self.viewport_height = log_height;

            let log_rect = Rect::new(0, 0, terminal_size.width, log_height);
            let agent_status_rect =
                Rect::new(0, log_height, terminal_size.width, agent_status_height);
            let bottom_pane_rect = Rect::new(
                0,
                log_height + agent_status_height,
                terminal_size.width,
                bottom_pane_height,
            );
            let status_rect = Rect::new(
                0,
                log_height + agent_status_height + bottom_pane_height,
                terminal_size.width,
                status_height,
            );

            // Clamp scroll offset before rendering
            self.clamp_scroll_offset_with_height(log_height as usize);

            // Extract mutable references for use in closure
            let log_lines = &mut self.log_lines;
            let scroll_offset = &mut self.scroll_offset;
            let bottom_pane = &mut self.bottom_pane;
            let model = &self.model;
            let provider = &self.provider;
            let token_usage = &self.token_usage;
            let agent_type = &self.agent_type;

            let terminal = self.guard.terminal_mut();
            terminal.draw(|frame| {
                // Render log lines
                let height = log_height as usize;
                let total_lines = log_lines.len();
                // Determine which lines to render based on scroll offset.
                // scroll_offset = 0 means show bottom-most lines.
                let start = total_lines.saturating_sub(height + *scroll_offset);
                let start = start.max(0);
                let end = (start + height).min(total_lines);

                let mut y = log_rect.y;
                for i in start..end {
                    let line = &log_lines[i];
                    Paragraph::new(line.clone()).render(
                        Rect::new(log_rect.x, y, log_rect.width, 1),
                        frame.buffer_mut(),
                    );
                    y += 1;
                }

                // Render agent status line
                let agent_status_text = format!(
                    "Agent Stack: {} | Session tokens: {}",
                    agent_type, token_usage.total_tokens
                );
                let paragraph = Paragraph::new(Line::from(agent_status_text))
                    .style(Style::default().fg(Color::Cyan));
                paragraph.render(agent_status_rect, frame.buffer_mut());

                // Render bottom pane
                bottom_pane.render(bottom_pane_rect, frame.buffer_mut());

                // Render status line
                let status_text = format!(
                    "nano code | Model: {} | Provider: {} | Tokens: {}",
                    model, provider, token_usage.total_tokens
                );
                let paragraph = Paragraph::new(Line::from(status_text))
                    .style(Style::default().fg(Color::Yellow));
                paragraph.render(status_rect, frame.buffer_mut());
            })?;

            let mut event_result = None;
            let mut msg_result = None;
            {
                let event_handler = &mut self.event_handler;
                let stack = &mut self.stack;
                let agent_rx = stack.current_rx().expect("agent stack is empty");
                tokio::select! {
                    Some(event) = async { event_handler.next().await } => {
                        event_result = Some(event);
                    }
                    Some(msg) = async { agent_rx.recv().await } => {
                        msg_result = Some(msg);
                    }
                    else => break,
                }
            }
            if let Some(event) = event_result {
                if self.handle_event(event).await? {
                    break;
                }
            }
            if let Some(msg) = msg_result {
                self.handle_incoming_message(msg).await?;
            }
        }

        // Send shutdown signal to agent thread
        let _ = self.send_to_agent(UiToAgent::Shutdown).await;
        Ok(())
    }
}

impl SimpleTui {
    async fn handle_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Exit loop
                    return Ok(true);
                }
                KeyCode::Enter => {
                    if let Some(composer) = self.bottom_pane.active_composer_mut() {
                        let input = std::mem::take(composer.input_mut());
                        if !input.is_empty() {
                            // Auto-scroll to see new message and response
                            self.auto_scroll = true;
                            // Add user input to log
                            self.push_log(self.indented_line(vec![
                                Span::styled("> ", Style::default().fg(Color::Cyan)),
                                Span::raw(input.clone()),
                            ]));
                            // Send request to agent
                            if let Err(e) = self.send_to_agent(UiToAgent::Request(input)).await {
                                self.push_log(self.indented_line(vec![
                                    Span::styled(
                                        "error sending request: ",
                                        Style::default().fg(Color::Red),
                                    ),
                                    Span::raw(e.to_string()),
                                ]));
                            }
                        }
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(composer) = self.bottom_pane.active_composer_mut() {
                        composer.input_mut().push(c);
                    }
                }
                KeyCode::Backspace => {
                    if let Some(composer) = self.bottom_pane.active_composer_mut() {
                        composer.input_mut().pop();
                    }
                }
                KeyCode::Up => {
                    // Scroll log up (increase scroll offset)
                    self.auto_scroll = false;
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                    self.clamp_scroll_offset();
                }
                KeyCode::Down => {
                    // Scroll log down (decrease scroll offset)
                    self.auto_scroll = false;
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    self.clamp_scroll_offset();
                }
                KeyCode::PageUp => {
                    self.auto_scroll = false;
                    self.scroll_offset = self.scroll_offset.saturating_add(10);
                    self.clamp_scroll_offset();
                }
                KeyCode::PageDown => {
                    self.auto_scroll = false;
                    self.scroll_offset = self.scroll_offset.saturating_sub(10);
                    self.clamp_scroll_offset();
                }
                KeyCode::Home => {
                    // scroll to top
                    self.auto_scroll = false;
                    self.scroll_offset = usize::MAX;
                    self.clamp_scroll_offset();
                }
                KeyCode::End => {
                    // scroll to bottom
                    self.auto_scroll = true;
                    self.scroll_offset = 0;
                }
                _ => {}
            },
            Event::Tick => {
                // Periodic updates
            }
            Event::TaskCompleted => {}
        }
        Ok(false)
    }

    async fn handle_incoming_message(&mut self, msg: AgentToUi) -> Result<()> {
        match msg {
            AgentToUi::SpawnChild {
                name,
                description,
                system_prompt,
                result_tx,
            } => {
                self.spawn_child_agent(name, description, system_prompt, result_tx)
                    .await?;
            }
            // If this is a final message from a child agent, we need to pop the stack
            // and send the result to parent.
            AgentToUi::Response(ref text) | AgentToUi::Error(ref text) => {
                let is_child = self.stack.len() > 1;
                if is_child {
                    // Child agent has completed
                    let success = matches!(msg, AgentToUi::Response(_));
                    let error = if success { None } else { Some(text.clone()) };
                    let output = text.clone();
                    let result_text = text.clone();
                    let result = if success {
                        ToolResult::success(result_text)
                    } else {
                        ToolResult::error(result_text)
                    };
                    // Pop the child from stack, sending result to parent via oneshot
                    let popped = self.stack.pop(Some(result));
                    // Update UI agent type after pop
                    self.update_agent_type_from_stack();
                    // Also need to send ChildResult to parent agent via its UI channel
                    if let Some(parent_tx) = self.stack.current_tx() {
                        let child_result = UiToAgent::ChildResult {
                            success,
                            output,
                            error,
                        };
                        let _ = parent_tx.send(child_result).await;
                    }
                    // Clean up popped agent (dropped)
                    drop(popped);
                }
                // Always log the response/error
                self.push_agent_message(msg);
            }
            _ => {
                // Regular message, just log it
                self.push_agent_message(msg);
            }
        }
        Ok(())
    }

    /// Update the agent stack display from the current stack state.
    fn update_agent_type_from_stack(&mut self) {
        self.agent_type = self.stack.stack_display();
    }

    async fn spawn_child_agent(
        &mut self,
        name: String,
        description: String,
        system_prompt: Option<String>,
        result_tx: oneshot::Sender<ToolResult>,
    ) -> Result<()> {
        // Generate unique CID for this agent
        let cid = NEXT_CID.fetch_add(1, Ordering::Relaxed);
        // Create agent based on profile name or system prompt
        let mut agent = match crate::Agent::new_with_profile(&name, self.config.clone(), self.workdir.clone()) {
            Ok(profile_agent) => profile_agent,
            Err(_) => {
                // Fall back to generic agent
                match crate::Agent::new(self.config.clone(), self.workdir.clone()) {
                    Ok(mut agent) => {
                        if let Some(prompt) = system_prompt {
                            match agent.with_system_prompt(Some(prompt)) {
                                Ok(subagent) => agent = subagent,
                                Err(e) => {
                                    let _ = result_tx.send(ToolResult::error(e.to_string()));
                                    return Ok(());
                                }
                            }
                        }
                        agent
                    }
                    Err(e) => {
                        let _ = result_tx.send(ToolResult::error(e.to_string()));
                        return Ok(());
                    }
                }
            }
        };
        // Set CID and name on agent
        agent.set_cid(cid);
        agent.set_name(name.clone());

        // Create channel pair for child agent
        let (agent_to_ui_tx, agent_to_ui_rx) = mpsc::channel(100);
        let ui_to_agent_tx = crate::agent_thread::spawn_with_agent(agent, agent_to_ui_tx);

        // Push child onto stack with provided result channel and CID
        self.stack.push_with_result_tx(
            name.clone(),
            ui_to_agent_tx,
            agent_to_ui_rx,
            result_tx,
            Some(cid),
        );

        // Send the task description to the child agent
        if let Some(child_tx) = self.stack.current_tx() {
            if let Err(e) = child_tx.send(UiToAgent::Request(description)).await {
                // Failed to send request to child agent
                // Pop the child we just pushed (since it hasn't started yet)
                if let Some(mut popped_handle) = self.stack.pop(None) {
                    // Get the result channel back from the popped handle
                    if let Some(result_tx) = popped_handle.take_child_result_tx() {
                        let _ = result_tx.send(ToolResult::error(format!(
                            "Failed to send request to child agent: {}",
                            e
                        )));
                    }
                    // Dropping popped_handle will close channels, causing agent thread to exit
                }
                return Ok(());
            }
        }

        // Update agent_type for UI display
        self.update_agent_type_from_stack();

        Ok(())
    }
}
