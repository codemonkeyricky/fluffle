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

pub mod agent;
pub mod agent_profile;
pub mod agent_thread;
pub mod profile_loader;
pub mod ai;
pub mod config;
pub mod error;
pub mod messaging;
pub mod plugin;
pub mod plugins;
pub mod types;
pub mod ui;

#[cfg(test)]
mod test_utils;

// Re-export commonly used types
pub use agent::Agent;
pub use agent_thread::{spawn, spawn_with_agent};
pub use ai::{create_provider, AiProvider, AiResponse, Message, ToolCall, ToolDefinition};
pub use config::Config;
pub use error::{Error, Result};
pub use messaging::{AgentToUi, UiToAgent};
pub use plugin::{Plugin, Tool};
pub use types::{ToolContext, ToolParameters, ToolResult};
pub use ui::{App, Event, EventHandler, Ui, create_ui};
