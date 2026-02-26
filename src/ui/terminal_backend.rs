use crate::agent_thread::spawn;
use crate::config::Config;
use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use crate::ui::components;
use crate::ui::ui_trait::Ui;
use crate::ui::{App, Event, EventHandler};
use async_trait::async_trait;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
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

/// Terminal UI backend using ratatui.
pub struct TerminalUi {
    /// Terminal guard for cleanup.
    guard: TerminalGuard,
    /// UI application state.
    app: App,
    /// Event handler for user input.
    event_handler: EventHandler,
    /// Receiver for messages from agent thread.
    agent_to_ui_rx: mpsc::Receiver<AgentToUi>,
    /// Sender for requests to agent thread.
    ui_to_agent_tx: mpsc::Sender<UiToAgent>,
}

impl TerminalUi {
    /// Create a new terminal UI backend.
    /// Sets up terminal, creates channels, spawns agent thread.
    pub async fn new(config: Config) -> Result<Self> {
        // Setup terminal
        let guard = TerminalGuard::setup()?;
        
        // Create channel for agent->UI updates
        let (agent_to_ui_tx, agent_to_ui_rx) = mpsc::channel(100);
        
        // Spawn agent thread (returns sender for UI->agent requests)
        let ui_to_agent_tx = spawn(config.clone(), agent_to_ui_tx);
        let ui_to_agent_tx_clone = ui_to_agent_tx.clone();
        
        // Create UI app with channels
        let app = App::new(config, ui_to_agent_tx).await?;
        let event_handler = EventHandler::new(250);
        
        Ok(Self {
            guard,
            app,
            event_handler,
            agent_to_ui_rx,
            ui_to_agent_tx: ui_to_agent_tx_clone,
        })
    }
}

#[async_trait]
impl Ui for TerminalUi {
    async fn run(&mut self) -> Result<()> {
        loop {
            self.guard.terminal_mut().draw(|f| {
                components::render(f, &self.app);
            })?;

            tokio::select! {
                Some(event) = self.event_handler.next() => {
                    match event {
                        Event::Key(key) => match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.app.quit();
                            }
                            KeyCode::Enter => {
                                if !self.app.input.is_empty() {
                                    let input = std::mem::take(&mut self.app.input);
                                    self.app.messages.push(format!("> {}", input));
                                    match self.ui_to_agent_tx.try_send(UiToAgent::Request(input)) {
                                        Ok(()) => {
                                            self.app.pending_requests += 1;
                                        }
                                        Err(e) => {
                                            self.app.messages.push(format!("Error queuing request: {}", e));
                                        }
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                self.app.input.push(c);
                            }
                            KeyCode::Backspace => {
                                self.app.input.pop();
                            }
                            _ => {}
                        },
                        Event::Tick => {
                            // Tick events can be used for periodic UI updates
                        }
                        Event::TaskCompleted => {}
                    }
                }
                Some(msg) = self.agent_to_ui_rx.recv() => {
                    self.app.handle_agent_message(msg);
                }
                else => break,
            }

            if self.app.should_quit {
                break;
            }
        }

        // Send shutdown signal to agent thread
        let _ = self.ui_to_agent_tx.send(UiToAgent::Shutdown).await;
        Ok(())
    }
}