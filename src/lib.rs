pub mod error;
pub mod types;
pub mod plugin;
pub mod config;
pub mod agent;
pub mod ai;
pub mod ui;
pub mod plugins;

// Re-export commonly used types
pub use error::{Error, Result};
pub use types::{ToolContext, ToolResult, ToolParameters};
pub use plugin::{Tool, Plugin};
pub use config::Config;
pub use agent::Agent;
pub use ai::{AiProvider, Message, ToolDefinition, AiResponse, ToolCall, create_provider};
pub use ui::App;