use nanocode::ui::{App, Event, EventHandler};
use nanocode::error::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new().await?;
    let event_handler = EventHandler::new(250);

    // Update plugin count in status (already set in App::new, but keep for clarity)
    app.status.plugins_loaded = app.agent.tools().len();

    // Main loop
    loop {
        terminal.draw(|f| {
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
                _ => {}
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

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}