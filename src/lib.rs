//! Nanocode: A framework for building AI-powered applications with a TUI interface.
//!
//! This library provides:
//! - AI agent runtime with support for multiple providers (OpenAI, Anthropic)
//! - Tool and plugin system for extending agent capabilities
//! - Configuration management
//! - TUI application framework with event handling
//!
//! # Basic Usage
//!
//! ```no_run
//! use nanocode::{Agent, Config, create_provider};
//! use nanocode::ai::{AiProvider, Message};
//! use nanocode::plugin::Tool;
//! ```
//!
//! For more details, see the individual modules.

pub mod error;
pub mod types;
pub mod plugin;
pub mod config;
pub mod agent;
pub mod ai;
pub mod plugins;
pub mod ui;
pub mod headless;

#[cfg(test)]
mod test_utils;

// Re-export commonly used types
pub use error::{Error, Result};
pub use types::{ToolContext, ToolResult, ToolParameters};
pub use plugin::{Tool, Plugin};
pub use config::Config;
pub use agent::Agent;
pub use ai::{AiProvider, Message, ToolDefinition, AiResponse, ToolCall, create_provider};
pub use ui::{App, Event, EventHandler};