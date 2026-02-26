use crate::agent_profile::AgentProfile;
use crate::config::Config;
use crate::plugin::{Plugin, Tool};
use crate::profile_loader;
use crate::types::{ToolContext, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct AgentProfilePlugin;

impl Plugin for AgentProfilePlugin {
    fn name(&self) -> &'static str {
        "agent_profile"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        // Load profiles if not already loaded
        if let Err(e) = profile_loader::load_profiles() {
            tracing::error!("Failed to load profiles: {}", e);
            return Vec::new();
        }

        let profile_names = profile_loader::profile_names();
        let mut tools = Vec::new();
        for name in profile_names {
            if let Some(profile) = profile_loader::get_profile(&name) {
                // Leak the profile to get a static reference
                let profile = Box::leak(Box::new(profile));
                tools.push(Arc::new(ProfileTool { profile }) as Arc<dyn Tool>);
            }
        }
        tools
    }
}

struct ProfileTool {
    // Leaked static reference to the profile
    profile: &'static AgentProfile,
}

#[async_trait]
impl Tool for ProfileTool {
    fn name(&self) -> &'static str {
        &self.profile.name
    }

    fn description(&self) -> &'static str {
        &self.profile.description
    }

    fn parameters(&self) -> ToolParameters {
        // Similar to task tool: description parameter
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Task description for the subagent",
                    "default": ""
                }
            },
            "required": ["description"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult {
        // Parse parameters
        let description = match params.get("description").and_then(|d| d.as_str()) {
            Some(d) => d,
            None => return ToolResult::error("Missing 'description' parameter"),
        };

        // Load config (should match parent agent's config)
        let config = match Config::load().await {
            Ok(config) => config,
            Err(e) => return ToolResult::error(format!("Failed to load config: {}", e)),
        };

        // Create agent with profile
        let mut agent = match crate::Agent::new_with_profile(&self.profile.name, config) {
            Ok(agent) => agent,
            Err(e) => return ToolResult::error(format!("Failed to create agent with profile: {}", e)),
        };

        // Set working directory from parent context
        agent.set_context(ctx.clone());

        // Run the task
        match agent.process(description).await {
            Ok(summary) => ToolResult::success(summary),
            Err(e) => ToolResult::error(format!("Profile agent failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_loader;

    #[test]
    fn test_profile_tool_creation() {
        profile_loader::clear_profiles();
        let plugin = AgentProfilePlugin;
        let tools = plugin.tools();
        // Should have at least generalist and explorer
        assert!(!tools.is_empty());
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(tool_names.contains(&"generalist"));
        assert!(tool_names.contains(&"explorer"));
    }
}

static AGENT_PROFILE_PLUGIN: AgentProfilePlugin = AgentProfilePlugin;

inventory::submit! {
    &AGENT_PROFILE_PLUGIN as &'static dyn Plugin
}