pub mod agent_stack;
pub mod app;
pub mod bottom_pane;
pub mod channels;

pub mod event;
pub mod headless_backend;

pub mod render;
pub mod simple_tui;
pub mod ui_trait;

pub use agent_stack::{AgentHandle, AgentStack};
pub use app::App;
pub use channels::UiChannels;
pub use event::{Event, EventHandler};
pub use ui_trait::Ui;

use crate::config::Config;
use crate::error::Result;
use std::path::PathBuf;

/// Create the appropriate UI backend based on configuration.
pub async fn create_ui(
    config: Config,
    headless: bool,
    prompt: Option<String>,
    workdir: Option<PathBuf>,
) -> Result<Box<dyn Ui>> {
    match headless {
        true => {
            let ui = headless_backend::HeadlessUi::new(config, prompt, workdir)?;
            Ok(Box::new(ui))
        }
        false => {
            let ui = simple_tui::SimpleTui::new(config, workdir).await?;
            Ok(Box::new(ui))
        }
    }
}
