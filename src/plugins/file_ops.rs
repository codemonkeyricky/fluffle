use std::sync::Arc;
use serde_json::{json, Value};
use async_trait::async_trait;
use crate::plugin::{Tool, Plugin};
use crate::types::{ToolContext, ToolResult, ToolParameters};
use crate::error::Result;

pub struct FileOpsPlugin;

impl Plugin for FileOpsPlugin {
    fn name(&self) -> &'static str {
        "file_operations"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(FileReadTool),
            Arc::new(FileWriteTool),
            Arc::new(FileListTool),
        ]
    }
}

struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "file_read"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file"
    }

    fn parameters(&self) -> ToolParameters {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult {
        let path = match params.get("path").and_then(|p| p.as_str()) {
            Some(p) => ctx.working_directory.join(p),
            None => return ToolResult::error("Missing 'path' parameter"),
        };

        match std::fs::read_to_string(&path) {
            Ok(content) => ToolResult::success(content),
            Err(e) => ToolResult::error(format!("Failed to read file: {}", e)),
        }
    }
}

struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &'static str {
        "file_write"
    }

    fn description(&self) -> &'static str {
        "Write content to a file"
    }

    fn parameters(&self) -> ToolParameters {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult {
        let path = match params.get("path").and_then(|p| p.as_str()) {
            Some(p) => ctx.working_directory.join(p),
            None => return ToolResult::error("Missing 'path' parameter"),
        };

        let content = match params.get("content").and_then(|c| c.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'content' parameter"),
        };

        match std::fs::write(&path, content) {
            Ok(_) => ToolResult::success(format!("Successfully wrote to {}", path.display())),
            Err(e) => ToolResult::error(format!("Failed to write file: {}", e)),
        }
    }
}

struct FileListTool;

#[async_trait]
impl Tool for FileListTool {
    fn name(&self) -> &'static str {
        "file_list"
    }

    fn description(&self) -> &'static str {
        "List files in a directory"
    }

    fn parameters(&self) -> ToolParameters {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list (default: current directory)",
                    "default": "."
                }
            },
            "required": []
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult {
        let dir_path = match params.get("path").and_then(|p| p.as_str()) {
            Some(p) => ctx.working_directory.join(p),
            None => ctx.working_directory.clone(),
        };

        match std::fs::read_dir(&dir_path) {
            Ok(entries) => {
                let mut files = Vec::new();
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            let name = path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let is_dir = path.is_dir();
                            let entry_type = if is_dir { "dir" } else { "file" };
                            files.push(format!("{} [{}]", name, entry_type));
                        }
                        Err(e) => return ToolResult::error(format!("Failed to read directory entry: {}", e)),
                    }
                }
                files.sort();
                ToolResult::success(files.join("\n"))
            }
            Err(e) => ToolResult::error(format!("Failed to read directory: {}", e)),
        }
    }
}

static FILE_OPS_PLUGIN: FileOpsPlugin = FileOpsPlugin;

inventory::submit! {
    &FILE_OPS_PLUGIN as &'static dyn Plugin
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolContext;
    use serde_json::json;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_file_read_tool() {
        let tool = FileReadTool;
        let ctx = ToolContext {
            working_directory: PathBuf::from("."),
            permissions: vec![],
        };

        let result = tool.execute(&ctx, json!({"path": "Cargo.toml"})).await;
        assert!(result.is_success());
        assert!(result.output().contains("nanocode"));
    }
}