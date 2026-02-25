use crate::config::Config;
use crate::plugin::{Plugin, Tool};
use crate::types::{ToolContext, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

const DEFAULT_EXPLORE_PROMPT: &str = r#"You are an expert codebase explorer. Your task is to explore a codebase, understand its structure, and provide insights. You have tools to read files, list directories, execute bash commands, and use git operations. 

Systematically explore the codebase and provide a concise summary including:
- Project type (language, framework, build system)
- Main directories and their purposes
- Key dependencies and configuration files
- Notable patterns or architecture
- Any findings relevant to the user's request

Be thorough but focused. Your final response should be a clear summary of your exploration."#;

pub struct ExplorePlugin;

impl Plugin for ExplorePlugin {
    fn name(&self) -> &'static str {
        "explore"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![Arc::new(ExploreTool)]
    }
}

struct ExploreTool;

#[async_trait]
impl Tool for ExploreTool {
    fn name(&self) -> &'static str {
        "explore"
    }

    fn description(&self) -> &'static str {
        "Explore a codebase using a specialized subagent with code exploration expertise"
    }

    fn parameters(&self) -> ToolParameters {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Specific exploration goal or focus area",
                    "default": "Explore the codebase structure and provide insights"
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

        // Create base agent
        let mut agent = match crate::Agent::new(config) {
            Ok(agent) => agent,
            Err(e) => return ToolResult::error(format!("Failed to create agent: {}", e)),
        };

        // Set working directory from parent context
        agent.set_context(ctx.clone());

        // Set explore system prompt
        match agent.with_system_prompt(Some(DEFAULT_EXPLORE_PROMPT.to_string())) {
            Ok(subagent) => agent = subagent,
            Err(e) => return ToolResult::error(format!("Failed to set system prompt: {}", e)),
        }

        // Run the exploration task
        match agent.process(description).await {
            Ok(summary) => ToolResult::success(summary),
            Err(e) => ToolResult::error(format!("Explore subagent failed: {}", e)),
        }
    }
}

static EXPLORE_PLUGIN: ExplorePlugin = ExplorePlugin;

inventory::submit! {
    &EXPLORE_PLUGIN as &'static dyn Plugin
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolContext;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_explore_tool_metadata() {
        let tool = ExploreTool;
        assert_eq!(tool.name(), "explore");
        assert!(tool.description().contains("exploration"));

        let params = tool.parameters();
        assert!(params.is_object());
        let props = params.get("properties").unwrap().as_object().unwrap();
        assert!(props.contains_key("description"));
    }

    #[tokio::test]
    async fn test_explore_tool_missing_description() {
        let tool = ExploreTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
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
