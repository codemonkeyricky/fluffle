use crate::agents::AgentProfile;
use crate::config::Config;
use crate::messaging::AgentToUi;
use crate::plugin::{Plugin, Tool};
use crate::profile_loader;
use crate::types::{ToolContext, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct AgentProfilePlugin;

impl Plugin for AgentProfilePlugin {
    fn name(&self) -> &'static str {
        "agents"
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
        // Use profile-specific tool parameters if defined, otherwise default
        match &self.profile.tool_parameters {
            Some(schema) => schema.clone(),
            None => json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Task description for the subagent",
                        "default": ""
                    }
                },
                "required": ["description"]
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult {
        // Parse parameters
        let description = match params.get("description").and_then(|d| d.as_str()) {
            Some(d) => d.to_string(),
            None => return ToolResult::error("Missing 'description' parameter"),
        };
        let requirements = params
            .get("requirements")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string());
        let validation_criteria = params
            .get("validation_criteria")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Build JSON payload for worker agents
        let payload = json!({
            "description": description,
            "requirements": requirements,
            "validation_criteria": validation_criteria
        });
        let description_json = payload.to_string();

        // Get agent_to_ui_tx to send spawn request
        let Some(agent_to_ui_tx) = ctx.agent_to_ui_tx.as_ref() else {
            // Fallback to inline execution if no UI channel (e.g., headless mode)
            return self.execute_inline(ctx, description_json).await;
        };

        // Create oneshot channel for result
        let (result_tx, result_rx) = oneshot::channel();

        // Send SpawnChild request to UI
        let spawn_msg = AgentToUi::SpawnChild {
            name: self.profile.name.clone(),
            description: description_json,
            system_prompt: None, // profile already has system prompt
            result_tx,
        };
        if let Err(e) = agent_to_ui_tx.send(spawn_msg).await {
            return ToolResult::error(format!("Failed to send spawn request: {}", e));
        }

        // Wait for child result
        match result_rx.await {
            Ok(result) => result,
            Err(_) => ToolResult::error("Child agent result channel closed"),
        }
    }
}

impl ProfileTool {
    async fn execute_inline(&self, ctx: &ToolContext, description: String) -> ToolResult {
        // Load config (should match parent agent's config)
        let config = match Config::load().await {
            Ok(config) => config,
            Err(e) => return ToolResult::error(format!("Failed to load config: {}", e)),
        };

        // Create agent with profile
        let mut agent = match crate::Agent::new_with_profile(&self.profile.name, config, None) {
            Ok(agent) => agent,
            Err(e) => {
                return ToolResult::error(format!("Failed to create agent with profile: {}", e))
            }
        };

        // Set working directory from parent context
        agent.set_context(ctx.clone());

        // Run the task
        match agent.process(&description).await {
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
        crate::app_name::set_app_name("coding");
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
