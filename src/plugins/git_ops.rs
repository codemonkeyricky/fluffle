use std::sync::Arc;
use serde_json::json;
use async_trait::async_trait;
use tokio::process::Command;
use crate::plugin::{Tool, Plugin};
use crate::types::{ToolContext, ToolResult, ToolParameters};

pub struct GitOpsPlugin;

impl Plugin for GitOpsPlugin {
    fn name(&self) -> &'static str {
        "git_operations"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(GitStatusTool),
            Arc::new(GitDiffTool),
        ]
    }
}

struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &'static str {
        "git_status"
    }

    fn description(&self) -> &'static str {
        "Show git status of the current repository"
    }

    fn parameters(&self) -> ToolParameters {
        json!({})
    }

    async fn execute(&self, ctx: &ToolContext, _params: ToolParameters) -> ToolResult {
        match Command::new("git")
            .arg("status")
            .current_dir(&ctx.working_directory)
            .output()
            .await
        {
            Ok(output) => {
                if output.status.success() {
                    ToolResult::success(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    ToolResult::error(format!(
                        "Command failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            }
            Err(e) => ToolResult::error(format!("Failed to execute command: {}", e)),
        }
    }
}

struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &'static str {
        "git_diff"
    }

    fn description(&self) -> &'static str {
        "Show git diff of the current repository"
    }

    fn parameters(&self) -> ToolParameters {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Optional path to show diff for (default: all changes)"
                }
            },
            "required": []
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult {
        let mut command = Command::new("git");
        command.arg("diff").current_dir(&ctx.working_directory);

        // Add optional path parameter
        if let Some(path_value) = params.get("path") {
            if let Some(path) = path_value.as_str() {
                if !path.is_empty() {
                    command.arg("--").arg(path);
                }
            }
        }

        match command.output().await {
            Ok(output) => {
                if output.status.success() {
                    ToolResult::success(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    ToolResult::error(format!(
                        "Command failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            }
            Err(e) => ToolResult::error(format!("Failed to execute command: {}", e)),
        }
    }
}

static GIT_OPS_PLUGIN: GitOpsPlugin = GitOpsPlugin;

inventory::submit! {
    &GIT_OPS_PLUGIN as &'static dyn Plugin
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolContext;
    use serde_json::json;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_git_status_tool() {
        let tool = GitStatusTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
        };

        let result = tool.execute(&ctx, json!({})).await;
        assert!(result.is_success());
        assert!(result.output().contains("On branch"));
    }

    #[tokio::test]
    async fn test_git_diff_tool() {
        let tool = GitDiffTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
        };

        let result = tool.execute(&ctx, json!({})).await;
        assert!(result.is_success());
        // git diff with no changes returns empty output
        // But we can at least check it doesn't error
    }
}