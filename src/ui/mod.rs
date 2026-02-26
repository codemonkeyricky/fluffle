pub mod app;
pub mod components;
pub mod event;
pub mod ui_trait;
pub mod headless_backend;
pub mod terminal_backend;

pub use app::App;
pub use event::{Event, EventHandler};
pub use ui_trait::Ui;

use crate::config::Config;
use crate::error::Result;

/// Create the appropriate UI backend based on configuration.
pub async fn create_ui(config: Config, headless: bool, prompt: Option<String>) -> Result<Box<dyn Ui>> {
    match headless {
        true => {
            let ui = headless_backend::HeadlessUi::new(config, prompt)?;
            Ok(Box::new(ui))
        }
        false => {
            let ui = terminal_backend::TerminalUi::new(config).await?;
            Ok(Box::new(ui))
        }
    }
}
