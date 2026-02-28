use crate::agent_thread::spawn;
use crate::ai::TokenUsage;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use crate::ui::bottom_pane::BottomPane;
use crate::ui::event::{Event, EventHandler};
use crate::ui::history_cell::{AgentMessageCell, HistoryCell, PlainHistoryCell};
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
    scroll_offset: usize,
    auto_scroll: bool,
}

impl ChatWidget {
    fn new() -> Self {
        Self {
            history_cells: Vec::new(),
            active_cell: None,
            scroll_offset: 0,
            auto_scroll: true,
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
            AgentToUi::ToolCall(text) => Box::new(PlainHistoryCell::new(text)),
            AgentToUi::ToolResult(text) => Box::new(PlainHistoryCell::new(text)),
            AgentToUi::Response(text) => Box::new(AgentMessageCell::new(text)),
            AgentToUi::Error(text) => Box::new(PlainHistoryCell::new(format!("Error: {}", text))),
            AgentToUi::TokenUsage(usage) => {
                Box::new(PlainHistoryCell::new(format!("Token usage: {:?}", usage)))
            }
            AgentToUi::Thinking(text) => {
                Box::new(PlainHistoryCell::new(format!("Thinking: {}", text)))
            }
        }
    }

    /// Adjust scroll offset by delta (positive = scroll down toward newer, negative = scroll up toward older).
    fn scroll_by(&mut self, delta: isize) {
        let new_offset = self.scroll_offset as isize + delta;
        self.scroll_offset = new_offset.max(0) as usize;
        // Clamp to max possible offset (cannot scroll past last cell)
        // The actual clamping depends on viewport height; we'll clamp later in render.
        // For now, just ensure it doesn't exceed cell count.
        let max_offset = self.history_cells.len().saturating_sub(1);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
        // When user scrolls manually, disable auto-scroll
        self.auto_scroll = false;
    }

    /// Scroll to bottom (show newest cells).
    fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
    }

    /// Scroll to top (show oldest cells).
    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    /// Update scroll offset based on viewport height and cell heights.
    /// Called from render to ensure offset is valid.
    fn clamp_scroll_offset(&mut self, viewport_height: u16, cell_heights: &[u16]) {
        if self.auto_scroll {
            // Compute start_index as the oldest cell that fits when showing newest cells
            let mut history_height = 0;
            let mut start_index = self.history_cells.len();
            for (i, &height) in cell_heights.iter().enumerate().rev() {
                if history_height + height > viewport_height {
                    break;
                }
                history_height += height;
                start_index = i;
            }
            self.scroll_offset = start_index;
        } else {
            // Ensure scroll_offset is within bounds
            let max_offset = self.history_cells.len().saturating_sub(1);
            if self.scroll_offset > max_offset {
                self.scroll_offset = max_offset;
            }
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
        let width = area.width;
        let max_y = area.y + area.height;
        let available_height = area.height;

        // Compute heights for all history cells
        let mut cell_heights: Vec<u16> = Vec::with_capacity(self.history_cells.len());
        for cell in &self.history_cells {
            let renderable = cell.as_renderable(width);
            cell_heights.push(renderable.required_height(width));
        }

        // Compute active cell height if present
        let active_height = self.active_cell.as_ref().map(|cell| {
            let renderable = cell.as_renderable(width);
            renderable.required_height(width)
        });

        // Reserve space for active cell if it fits
        let mut remaining_height = available_height;
        let include_active = active_height.map_or(false, |h| h <= remaining_height);
        if include_active {
            remaining_height -= active_height.unwrap();
        }

        // Update scroll offset based on remaining height and cell heights
        self.clamp_scroll_offset(remaining_height, &cell_heights);

        // Determine which history cells to render (starting from scroll_offset)
        let mut history_height = 0;
        let mut end_index = self.scroll_offset; // exclusive
        for (i, &height) in cell_heights.iter().enumerate().skip(self.scroll_offset) {
            if history_height + height > remaining_height {
                break;
            }
            history_height += height;
            end_index = i + 1;
        }

        // Render history cells top-down
        let mut y = area.y;
        for i in self.scroll_offset..end_index {
            let cell = &self.history_cells[i];
            let mut renderable = cell.as_renderable(width);
            let height = cell_heights[i];
            let rect = Rect::new(area.x, y, width, height);
            renderable.render(rect, buf);
            y += height;
        }

        // Render active cell below history cells if it fits
        if include_active {
            if let Some(active) = &self.active_cell {
                let mut renderable = active.as_renderable(width);
                let height = active_height.unwrap();
                if y + height <= max_y {
                    let rect = Rect::new(area.x, y, width, height);
                    renderable.render(rect, buf);
                }
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

    fn render_status_line(
        area: Rect,
        buf: &mut Buffer,
        model: &str,
        provider: &str,
        token_usage: &TokenUsage,
    ) {
        use ratatui::{
            style::{Color, Style},
            text::Line,
            widgets::{Paragraph, Widget},
        };
        let status_text = format!(
            "nano code | Model: {} | Provider: {} | Tokens: {}",
            model, provider, token_usage.total_tokens
        );
        let paragraph =
            Paragraph::new(Line::from(status_text)).style(Style::default().fg(Color::Yellow));
        paragraph.render(area, buf);
    }
}

#[async_trait]
impl Ui for AdvancedTerminalUi {
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
            let terminal = self.guard.terminal_mut();
            let terminal_size = terminal.size()?;

            // Compute layout
            let status_height = 1;
            let bottom_pane_height = self.bottom_pane.required_height(terminal_size.width);
            let max_bottom_height = terminal_size.height.saturating_sub(status_height);
            let bottom_pane_height = bottom_pane_height.min(max_bottom_height);
            let available_for_viewport = terminal_size
                .height
                .saturating_sub(bottom_pane_height + status_height);
            let viewport_height = available_for_viewport.max(1);

            let viewport_rect = Rect::new(0, 0, terminal_size.width, viewport_height);
            let bottom_pane_rect =
                Rect::new(0, viewport_height, terminal_size.width, bottom_pane_height);
            let status_rect = Rect::new(
                0,
                viewport_height + bottom_pane_height,
                terminal_size.width,
                status_height,
            );

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
                Self::render_status_line(
                    status_rect,
                    frame.buffer_mut(),
                    model,
                    provider,
                    token_usage,
                );
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
                                        if let Err(e) = self.send_to_agent(UiToAgent::Request(input)).await {
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
                            KeyCode::Up => {
                                self.chat_widget.scroll_by(-1);
                            }
                            KeyCode::Down => {
                                self.chat_widget.scroll_by(1);
                            }
                            KeyCode::PageUp => {
                                self.chat_widget.scroll_by(-5);
                            }
                            KeyCode::PageDown => {
                                self.chat_widget.scroll_by(5);
                            }
                            KeyCode::Home => {
                                self.chat_widget.scroll_to_top();
                            }
                            KeyCode::End => {
                                self.chat_widget.scroll_to_bottom();
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
        let _ = self.send_to_agent(UiToAgent::Shutdown).await;
        Ok(())
    }
}
