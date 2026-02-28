use crate::config::Config;
use crate::plugin::{Plugin, Tool};
use crate::types::{ToolContext, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn name(&self) -> &'static str {
        "task"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![Arc::new(TaskTool)]
    }
}

struct TaskTool;

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &'static str {
        "task"
    }

    fn description(&self) -> &'static str {
        "Create a subagent conversation with custom system prompt"
    }

    fn parameters(&self) -> ToolParameters {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Task description for the subagent",
                    "default": ""
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Custom system prompt for the subagent"
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

        let system_prompt = params.get("system_prompt").and_then(|sp| sp.as_str());

        // Load config (should match parent agent's config)
        let config = match Config::load().await {
            Ok(config) => config,
            Err(e) => return ToolResult::error(format!("Failed to load config: {}", e)),
        };

        // Create base agent
        let mut agent = match crate::Agent::new(config) {
            Ok(agent) => agent,
            Err(e) => return ToolResult::error(format!("Failed to create agent: {}", e)),
        };

        // Set working directory from parent context
        agent.set_context(ctx.clone());

        // Set system prompt if provided
        if let Some(prompt) = system_prompt {
            // Use with_system_prompt to create a new agent with the prompt
            match agent.with_system_prompt(Some(prompt.to_string())) {
                Ok(subagent) => agent = subagent,
                Err(e) => return ToolResult::error(format!("Failed to set system prompt: {}", e)),
            }
        }

        // Run the task
        match agent.process(description).await {
            Ok(summary) => ToolResult::success(summary),
            Err(e) => ToolResult::error(format!("Subagent failed: {}", e)),
        }
    }
}

static TASK_PLUGIN: TaskPlugin = TaskPlugin;

inventory::submit! {
    &TASK_PLUGIN as &'static dyn Plugin
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolContext;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_task_tool_metadata() {
        let tool = TaskTool;
        assert_eq!(tool.name(), "task");
        assert!(tool.description().contains("subagent"));

        let params = tool.parameters();
        assert!(params.is_object());
        let props = params.get("properties").unwrap().as_object().unwrap();
        assert!(props.contains_key("description"));
        assert!(props.contains_key("system_prompt"));
    }

    #[tokio::test]
    async fn test_task_tool_missing_description() {
        let tool = TaskTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
            agent_to_ui_tx: None,
        };

        let result = tool.execute(&ctx, json!({})).await;
        assert!(!result.is_success());
        assert!(result
            .error_message()
            .unwrap()
            .contains("Missing 'description'"));
    }

    // Note: Full execution test requires valid config and API key
    // This is better suited for integration tests
}
