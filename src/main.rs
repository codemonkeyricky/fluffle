use nanocode::ui::{App, Event, EventHandler};
use nanocode::error::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

/// Guard that ensures terminal state is restored when dropped.
struct TerminalGuard {
    terminal: Option<Terminal<CrosstermBackend<io::Stdout>>>,
}

impl TerminalGuard {
    /// Setup terminal and return guard that will restore on drop.
    fn setup() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal: Some(terminal) })
    }

    /// Get mutable reference to terminal.
    /// Panics if terminal has been taken.
    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        self.terminal.as_mut().expect("terminal should be present")
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Restore terminal state
        let _ = disable_raw_mode();
        if let Some(terminal) = &mut self.terminal {
            let _ = execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
            let _ = terminal.show_cursor();
        } else {
            // Terminal was taken, still attempt to restore stdout
            let _ = execute!(
                io::stdout(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal with guard
    let mut guard = TerminalGuard::setup()?;

    // Create app
    let mut app = App::new().await?;
    let event_handler = EventHandler::new(250);

    // Update plugin count (already set in App::new, but refresh for consistency)
    app.status.plugins_loaded = app.agent.tools().len();

    // Main loop
    let result = async {
        loop {
            guard.terminal_mut().draw(|f| {
                nanocode::ui::components::render(f, &app);
            })?;

            match event_handler.next() {
                Ok(Event::Key(key)) => match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.quit();
                    }
                    KeyCode::Enter => {
                        app.handle_input().await?;
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    _ => {} // Ignore other keys
                },
                Ok(Event::Tick) => {
                    // Update status or other periodic tasks
                }
                Err(_) => {
                    // Channel disconnected, break loop
                    break;
                }
            }

            if app.should_quit {
                break;
            }
        }
        Ok(())
    }.await;

    // Explicitly drop guard to restore terminal before returning
    drop(guard);
    result
}