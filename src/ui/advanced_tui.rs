use crate::agent_thread::spawn;
use crate::ai::TokenUsage;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use crate::ui::bottom_pane::BottomPane;
use crate::ui::event::{Event, EventHandler};
use crate::ui::history_cell::{HistoryCell, PlainHistoryCell, AgentMessageCell};
use crate::ui::render::Renderable;
use crate::ui::ui_trait::Ui;
use async_trait::async_trait;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, buffer::Buffer, layout::Rect, Terminal};
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

/// Widget that manages the transcript (history cells) and active cell.
struct ChatWidget {
    history_cells: Vec<Box<dyn HistoryCell>>,
    active_cell: Option<Box<dyn HistoryCell>>,
}

impl ChatWidget {
    fn new() -> Self {
        Self {
            history_cells: Vec::new(),
            active_cell: None,
        }
    }

    /// Add a committed history cell.
    fn push_history(&mut self, cell: Box<dyn HistoryCell>) {
        self.history_cells.push(cell);
    }

    /// Set the active (streaming) cell.
    fn set_active_cell(&mut self, cell: Box<dyn HistoryCell>) {
        self.active_cell = Some(cell);
    }

    /// Clear the active cell (when streaming finishes).
    fn clear_active_cell(&mut self) {
        self.active_cell = None;
    }

    /// Convert an agent message to a history cell.
    fn cell_from_agent_message(&self, msg: AgentToUi) -> Box<dyn HistoryCell> {
        match msg {
            AgentToUi::ToolCall(text) => Box::new(PlainHistoryCell::new(format!("Tool call: {}", text))),
            AgentToUi::ToolResult(text) => Box::new(PlainHistoryCell::new(format!("Tool result: {}", text))),
            AgentToUi::Response(text) => Box::new(AgentMessageCell::new(text)),
            AgentToUi::Error(text) => Box::new(PlainHistoryCell::new(format!("Error: {}", text))),
            AgentToUi::TokenUsage(usage) => Box::new(PlainHistoryCell::new(format!("Token usage: {:?}", usage))),
        }
    }
}

impl Renderable for ChatWidget {
    fn required_height(&self, width: u16) -> u16 {
        let mut total = 0;
        for cell in &self.history_cells {
            total += cell.as_renderable(width).required_height(width);
        }
        if let Some(active) = &self.active_cell {
            total += active.as_renderable(width).required_height(width);
        }
        total
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        let width = area.width;
        let max_y = area.y + area.height;

        for cell in &self.history_cells {
            let mut renderable = cell.as_renderable(width);
            let height = renderable.required_height(width);
            if y + height > max_y {
                break;
            }
            let rect = Rect::new(area.x, y, width, height);
            renderable.render(rect, buf);
            y += height;
        }

        if let Some(active) = &self.active_cell {
            let mut renderable = active.as_renderable(width);
            let height = renderable.required_height(width);
            if y + height <= max_y {
                let rect = Rect::new(area.x, y, width, height);
                renderable.render(rect, buf);
            }
        }
    }
}

/// Advanced terminal UI backend using the Codex‑style layout.
pub struct AdvancedTerminalUi {
    /// Terminal guard for cleanup.
    guard: TerminalGuard,
    /// Chat widget managing transcript.
    chat_widget: ChatWidget,
    /// Bottom pane with composer and popups.
    bottom_pane: BottomPane,
    /// Shared channels for UI↔agent communication.
    channels: crate::ui::UiChannels,
    /// Event handler for user input.
    event_handler: EventHandler,
    /// Current model name.
    model: String,
    /// Current provider name.
    provider: String,
    /// Token usage statistics.
    token_usage: TokenUsage,
}

impl AdvancedTerminalUi {
    /// Create a new advanced terminal UI backend.
    /// Sets up terminal, creates channels, spawns agent thread.
    pub async fn new(config: Config) -> Result<Self> {
        // Setup terminal
        let guard = TerminalGuard::setup()?;
        
        // Create channel for agent->UI updates
        let (agent_to_ui_tx, agent_to_ui_rx) = mpsc::channel(100);
        
        // Spawn agent thread (returns sender for UI->agent requests)
        let ui_to_agent_tx = spawn(config.clone(), agent_to_ui_tx);
        let ui_to_agent_tx_clone = ui_to_agent_tx.clone();
        
        // Create UI widgets
        let chat_widget = ChatWidget::new();
        let bottom_pane = BottomPane::new();
        
        let channels = crate::ui::UiChannels {
            agent_to_ui_rx,
            ui_to_agent_tx: ui_to_agent_tx_clone,
        };
        
        let event_handler = EventHandler::new(250);
        
        Ok(Self {
            guard,
            chat_widget,
            bottom_pane,
            channels,
            event_handler,
            model: config.model.clone(),
            provider: config.provider.clone(),
            token_usage: TokenUsage::default(),
        })
    }
    
    fn render_status_line(area: Rect, buf: &mut Buffer, model: &str, provider: &str, token_usage: &TokenUsage) {
        use ratatui::{style::{Style, Color}, text::Line, widgets::{Paragraph, Widget}};
        let status_text = format!("nano code | Model: {} | Provider: {} | Tokens: {}", 
            model, provider, token_usage.total_tokens);
        let paragraph = Paragraph::new(Line::from(status_text))
            .style(Style::default().fg(Color::Yellow));
        paragraph.render(area, buf);
    }
}

#[async_trait]
impl Ui for AdvancedTerminalUi {
    async fn run(&mut self) -> Result<()> {
        loop {
            let terminal = self.guard.terminal_mut();
            let terminal_size = terminal.size()?;
            
            // Compute layout
            let status_height = 1;
            let bottom_pane_height = self.bottom_pane.required_height(terminal_size.width);
            let max_bottom_height = terminal_size.height.saturating_sub(status_height);
            let bottom_pane_height = bottom_pane_height.min(max_bottom_height);
            let available_for_viewport = terminal_size.height.saturating_sub(bottom_pane_height + status_height);
            let viewport_height = available_for_viewport.max(1);
            
            let viewport_rect = Rect::new(0, 0, terminal_size.width, viewport_height);
            let bottom_pane_rect = Rect::new(0, viewport_height, terminal_size.width, bottom_pane_height);
            let status_rect = Rect::new(0, viewport_height + bottom_pane_height, terminal_size.width, status_height);
            
            // Extract mutable references to widgets for use in closure
            let chat_widget = &mut self.chat_widget;
            let bottom_pane = &mut self.bottom_pane;
            let model = &self.model;
            let provider = &self.provider;
            let token_usage = &self.token_usage;
            
            // Draw
            terminal.draw(|frame| {
                // Render chat widget into viewport
                chat_widget.render(viewport_rect, frame.buffer_mut());
                // Render bottom pane
                bottom_pane.render(bottom_pane_rect, frame.buffer_mut());
                // Render status line
                Self::render_status_line(status_rect, frame.buffer_mut(), model, provider, token_usage);
            })?;
            
            tokio::select! {
                Some(event) = self.event_handler.next() => {
                    match event {
                        Event::Key(key) => match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            KeyCode::Enter => {
                                if let Some(composer) = self.bottom_pane.active_composer_mut() {
                                    let input = std::mem::take(composer.input_mut());
                                    if !input.is_empty() {
                                        // Add user message to chat widget
                                        let user_cell = PlainHistoryCell::new(format!("> {}", input));
                                        self.chat_widget.push_history(Box::new(user_cell));
                                        // Send request to agent
                                        if let Err(e) = self.channels.ui_to_agent_tx.send(UiToAgent::Request(input)).await {
                                            let error_cell = PlainHistoryCell::new(format!("Error sending request: {}", e));
                                            self.chat_widget.push_history(Box::new(error_cell));
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
                            _ => {}
                        },
                        Event::Tick => {
                            // Periodic updates
                        }
                        Event::TaskCompleted => {}
                    }
                }
                Some(msg) = self.channels.agent_to_ui_rx.recv() => {
                    match msg {
                        AgentToUi::TokenUsage(usage) => {
                            self.token_usage = usage;
                        }
                        _ => {
                            let cell = self.chat_widget.cell_from_agent_message(msg);
                            self.chat_widget.push_history(cell);
                        }
                    }
                }
                else => break,
            }
        }
        
        // Send shutdown signal to agent thread
        let _ = self.channels.ui_to_agent_tx.send(UiToAgent::Shutdown).await;
        Ok(())
    }
}