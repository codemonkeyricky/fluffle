//! Plugin system traits for the nano code agent.
//!
//! This module defines the core traits for tools and plugins that can be
//! registered with the agent. Tools represent individual capabilities
//! (file operations, bash execution, etc.), while plugins are collections
//! of related tools.
//!
//! Plugins are registered at compile time using the `inventory` crate.

use std::sync::Arc;
use async_trait::async_trait;
use crate::types::{ToolContext, ToolResult, ToolParameters};

/// A tool that can be executed by the AI agent.
///
/// Tools represent individual capabilities that the agent can use, such as
/// file operations, bash command execution, or git operations.
/// Each tool must implement this trait to be discoverable by the agent.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique name of the tool.
    fn name(&self) -> &'static str;

    /// Returns a human-readable description of what the tool does.
    fn description(&self) -> &'static str;

    /// Returns a JSON schema describing the parameters the tool accepts.
    /// This schema is used by the AI to understand how to call the tool.
    fn parameters(&self) -> ToolParameters;

    /// Executes the tool with the given context and parameters.
    /// Returns a `ToolResult` indicating success or failure.
    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult;
}

/// A plugin that provides a collection of related tools.
///
/// Plugins are the primary extension mechanism for the nano code agent.
/// Each plugin can provide multiple tools that share common functionality
/// or dependencies.
pub trait Plugin: Send + Sync {
    /// Returns the unique name of the plugin.
    fn name(&self) -> &'static str;

    /// Returns the version of the plugin.
    fn version(&self) -> &'static str;

    /// Returns all tools provided by this plugin.
    fn tools(&self) -> Vec<Arc<dyn Tool>>;
}

// Static plugin registration using the `inventory` crate.
// Plugins can register themselves at compile time using `inventory::submit!`.
inventory::collect!(&'static dyn Plugin);

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn test_tool_trait_methods() {
        struct TestTool;

        #[async_trait]
        impl Tool for TestTool {
            fn name(&self) -> &'static str { "test" }
            fn description(&self) -> &'static str { "test tool" }
            fn parameters(&self) -> ToolParameters { json!({}) }
            async fn execute(&self, _ctx: &ToolContext, _params: ToolParameters) -> ToolResult {
                ToolResult::success("test")
            }
        }

        let tool = TestTool;
        assert_eq!(tool.name(), "test");
        assert_eq!(tool.description(), "test tool");
    }

    #[test]
    fn test_tool_is_send_and_sync() {
        // Compile-time assertion that dyn Tool implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        // This will fail to compile if Tool doesn't have Send + Sync bounds
        assert_send_sync::<Arc<dyn Tool>>();
    }
}