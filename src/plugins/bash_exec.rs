use crate::plugin::{Plugin, Tool};
use crate::types::{ToolContext, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tokio::process::Command;

pub struct BashExecPlugin;

impl Plugin for BashExecPlugin {
    fn name(&self) -> &'static str {
        "bash_execution"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![Arc::new(BashExecTool)]
    }
}

struct BashExecTool;

#[async_trait]
impl Tool for BashExecTool {
    fn name(&self) -> &'static str {
        "bash_exec"
    }

    fn description(&self) -> &'static str {
        "Execute a bash command and return the output"
    }

    fn parameters(&self) -> ToolParameters {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Bash command to execute"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult {
        let command = match params.get("command").and_then(|c| c.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'command' parameter"),
        };

        match Command::new("bash")
            .arg("-c")
            .arg(command)
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

static BASH_EXEC_PLUGIN: BashExecPlugin = BashExecPlugin;

inventory::submit! {
    &BASH_EXEC_PLUGIN as &'static dyn Plugin
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolContext;
    use serde_json::json;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_bash_exec_tool() {
        let tool = BashExecTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
        };

        let result = tool.execute(&ctx, json!({"command": "echo hello"})).await;
        assert!(result.is_success());
        assert_eq!(result.output().trim(), "hello");
    }

    #[tokio::test]
    async fn test_bash_exec_missing_command() {
        let tool = BashExecTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
        };

        let result = tool.execute(&ctx, json!({})).await;
        assert!(!result.is_success());
        assert!(result
            .error_message()
            .unwrap()
            .contains("Missing 'command' parameter"));
    }

    #[tokio::test]
    async fn test_bash_exec_failing_command() {
        let tool = BashExecTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
        };

        let result = tool.execute(&ctx, json!({"command": "false"})).await;
        assert!(!result.is_success());
        assert!(result.error_message().unwrap().contains("Command failed"));
    }

    #[tokio::test]
    async fn test_bash_exec_command_with_stderr() {
        let tool = BashExecTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
        };

        // Command that writes to stderr but exits with success
        let result = tool
            .execute(&ctx, json!({"command": "echo error >&2; echo stdout"}))
            .await;
        assert!(result.is_success());
        assert_eq!(result.output().trim(), "stdout");
    }

    #[tokio::test]
    async fn test_bash_exec_command_with_newline() {
        let tool = BashExecTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
        };

        let result = tool
            .execute(&ctx, json!({"command": "echo -e 'line1\nline2'"}))
            .await;
        assert!(result.is_success());
        assert_eq!(result.output().trim(), "line1\nline2");
    }
}
