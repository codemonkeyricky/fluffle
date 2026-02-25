use nanocode::ui::{App, Event, EventHandler};
use nanocode::error::Result;
use nanocode::headless;
use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "nanocode")]
struct Args {
    #[structopt(short, long, help = "Run in headless mode (stdout/stdin)")]
    headless: bool,
    #[structopt(short = "p", long, help = "Prompt for headless mode")]
    prompt: Option<String>,
}

fn parse_args(args: std::env::Args) -> Args {
    let args = args.collect::<Vec<_>>();
    let mut headless = false;
    let mut prompt: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "-p" || args[i] == "--headless" {
            headless = true;
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                prompt = Some(args[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
        } else if args[i] == "-h" || args[i] == "--help" {
            println!("nanocode 0.1.0");
            println!();
            println!("USAGE:");
            println!("    nanocode [FLAGS]");
            println!();
            println!("FLAGS:");
            println!("        --help        Prints help information");
            println!("    -h, --headless    Run in headless mode (stdout/stdin)");
            println!("    -p, --prompt P    Prompt for headless mode");
            std::process::exit(0);
        } else {
            i += 1;
        }
    }

    Args { headless, prompt }
}

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

        Ok(Self { terminal: Some(terminal) })
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
    let args = parse_args(std::env::args());

    if args.headless {
        let config = nanocode::config::Config::load().await?;
        headless::run(config, args.prompt).await
    } else {
        let mut guard = TerminalGuard::setup()?;
        let mut app = App::new().await?;
        let mut event_handler = EventHandler::new(250);


        let result = async {
            loop {
                guard.terminal_mut().draw(|f| {
                    nanocode::ui::components::render(f, &app);
                })?;

                match event_handler.next().await {
                    Some(Event::Key(key)) => match key.code {
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
                    Some(Event::Tick) => {
                        if app.check_task_completion().await {
                            continue;
                        }

                        if app.shared_messages.is_dirty() {
                            continue;
                        }
                    }
                    Some(Event::TaskCompleted) => {
                    }
                    None => {
                        break;
                    }
                }

                if app.should_quit {
                    break;
                }
            }
            Ok(())
        }.await;

        drop(guard);
        result
    }
}
