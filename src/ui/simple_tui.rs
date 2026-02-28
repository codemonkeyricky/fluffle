use crate::agent_thread::spawn;
use crate::ai::TokenUsage;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
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
use tokio::sync::mpsc;

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
    /// Shared channels for UI↔agent communication.
    channels: crate::ui::UiChannels,
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
    /// Active agent type (e.g., "generalist", "explorer").
    agent_type: String,
}

impl SimpleTui {
    /// Create a new simple terminal UI backend.
    /// Sets up terminal, creates channels, spawns agent thread.
    pub async fn new(config: Config) -> Result<Self> {
        // Setup terminal
        let guard = TerminalGuard::setup()?;

        // Create channel for agent->UI updates
        let (agent_to_ui_tx, agent_to_ui_rx) = mpsc::channel(100);

        // Spawn agent thread (returns sender for UI->agent requests)
        let ui_to_agent_tx = spawn(config.clone(), agent_to_ui_tx);
        let ui_to_agent_tx_clone = ui_to_agent_tx.clone();

        let channels = crate::ui::UiChannels {
            agent_to_ui_rx,
            ui_to_agent_tx: ui_to_agent_tx_clone,
        };

        let event_handler = EventHandler::new(250);
        let bottom_pane = BottomPane::new();

        Ok(Self {
            guard,
            channels,
            event_handler,
            bottom_pane,
            log_lines: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            viewport_height: 0,
            model: config.model.clone(),
            provider: config.provider.clone(),
            token_usage: TokenUsage::default(),
            agent_type: "generalist".to_string(),
        })
    }

    /// Convert an agent message to styled lines and add them to log.
    fn push_agent_message(&mut self, msg: AgentToUi) {
        let (prefix, color) = match &msg {
            AgentToUi::ToolCall(_) => ("", Color::Gray),
            AgentToUi::ToolResult(_) => ("", Color::Gray),
            AgentToUi::Thinking(_) => ("Thinking: ", Color::Blue),
            AgentToUi::Response(_) => ("", Color::Green),
            AgentToUi::Error(_) => ("", Color::Red),
            AgentToUi::TokenUsage(usage) => {
                self.token_usage = usage.clone();
                let text = format!(
                    "Tokens used: prompt: {}, completion: {}, total: {}",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
                self.push_log(Line::from(text));
                return;
            }
        };
        let text = match msg {
            AgentToUi::ToolCall(text) => text,
            AgentToUi::ToolResult(text) => text,
            AgentToUi::Thinking(text) => text,
            AgentToUi::Response(text) => text,
            AgentToUi::Error(text) => text,
            AgentToUi::TokenUsage(_) => unreachable!(),
        };
        // Split by newlines, add prefix only to first line.
        let lines: Vec<&str> = text.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            let styled_line = if prefix.is_empty() || i > 0 {
                Line::from(Span::styled(line.to_string(), Style::default().fg(color)))
            } else {
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(color)),
                    Span::raw(line.to_string()),
                ])
            };
            self.push_log(styled_line);
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


}

#[async_trait]
impl Ui for SimpleTui {
    fn agent_rx(&mut self) -> &mut mpsc::Receiver<AgentToUi> {
        &mut self.channels.agent_to_ui_rx
    }

    fn agent_tx(&mut self) -> &mut mpsc::Sender<UiToAgent> {
        &mut self.channels.ui_to_agent_tx
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
            let max_bottom_height = terminal_size.height.saturating_sub(status_height + agent_status_height);
            let bottom_pane_height = bottom_pane_height.min(max_bottom_height);
            let available_for_log = terminal_size
                .height
                .saturating_sub(bottom_pane_height + agent_status_height + status_height);
            let log_height = available_for_log.max(1);
            self.viewport_height = log_height;

            let log_rect = Rect::new(0, 0, terminal_size.width, log_height);
            let agent_status_rect = Rect::new(0, log_height, terminal_size.width, agent_status_height);
            let bottom_pane_rect =
                Rect::new(0, log_height + agent_status_height, terminal_size.width, bottom_pane_height);
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
                    "Agent: {} | Session tokens: {}",
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

            tokio::select! {
                Some(event) = self.event_handler.next() => {
                    if self.handle_event(event).await? {
                        break;
                    }
                }
                Some(msg) = self.channels.agent_to_ui_rx.recv() => {
                    self.handle_agent_message(msg).await?;
                }
                else => break,
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
                            self.push_log(Line::from(vec![
                                Span::styled("> ", Style::default().fg(Color::Cyan)),
                                Span::raw(input.clone()),
                            ]));
                            // Send request to agent
                            if let Err(e) = self.send_to_agent(UiToAgent::Request(input)).await {
                                self.push_log(Line::from(vec![
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

    async fn handle_agent_message(&mut self, msg: AgentToUi) -> Result<()> {
        self.push_agent_message(msg);
        Ok(())
    }
}
