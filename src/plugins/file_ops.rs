use crate::plugin::{Plugin, Tool};
use crate::types::{ToolContext, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

/// Securely join a relative path to a base directory, preventing directory traversal attacks.
/// Returns an error if the resulting path would be outside the base directory.
fn secure_join(base: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() {
        return Ok(base.to_path_buf());
    }

    // Normalize path by removing "." and ".." components
    let mut components = Vec::new();
    for component in Path::new(relative).components() {
        match component {
            std::path::Component::Normal(name) => {
                components.push(name);
            }
            std::path::Component::ParentDir => {
                if components.pop().is_none() {
                    // Trying to go above the base directory
                    return Err("Path traversal attempt detected".to_string());
                }
            }
            std::path::Component::CurDir => {
                // Current directory, ignore
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                // Absolute paths or Windows prefixes not allowed
                return Err("Absolute paths are not allowed".to_string());
            }
        }
    }

    // Build final path
    let mut result = base.to_path_buf();
    for component in components {
        result.push(component);
    }

    Ok(result)
}

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
        let relative_path = match params.get("path").and_then(|p| p.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing 'path' parameter"),
        };

        let path = match secure_join(&ctx.working_directory, relative_path) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid path: {}", e)),
        };

        match fs::read_to_string(&path).await {
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
        let relative_path = match params.get("path").and_then(|p| p.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing 'path' parameter"),
        };

        let content = match params.get("content").and_then(|c| c.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'content' parameter"),
        };

        let path = match secure_join(&ctx.working_directory, relative_path) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid path: {}", e)),
        };

        match fs::write(&path, content).await {
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
            Some(p) => match secure_join(&ctx.working_directory, p) {
                Ok(path) => path,
                Err(e) => return ToolResult::error(format!("Invalid path: {}", e)),
            },
            None => ctx.working_directory.clone(),
        };

        match fs::read_dir(&dir_path).await {
            Ok(mut entries) => {
                let mut files = Vec::new();
                loop {
                    match entries.next_entry().await {
                        Ok(Some(entry)) => {
                            let name = entry.file_name().to_string_lossy().to_string();
                            match entry.metadata().await {
                                Ok(metadata) => {
                                    let is_dir = metadata.is_dir();
                                    let entry_type = if is_dir { "dir" } else { "file" };
                                    files.push(format!("{} [{}]", name, entry_type));
                                }
                                Err(e) => {
                                    return ToolResult::error(format!(
                                        "Failed to read metadata: {}",
                                        e
                                    ))
                                }
                            }
                        }
                        Ok(None) => break, // No more entries
                        Err(e) => {
                            return ToolResult::error(format!(
                                "Failed to read directory entry: {}",
                                e
                            ))
                        }
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

    fn create_test_dir() -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir_name = format!("nanocode_test_{}", timestamp);
        let path = std::env::temp_dir().join(dir_name);
        std::fs::create_dir(&path).unwrap();
        path
    }

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

    #[tokio::test]
    async fn test_file_write_tool() {
        let temp_dir = create_test_dir();
        let tool = FileWriteTool;
        let ctx = ToolContext {
            working_directory: temp_dir.clone(),
            permissions: vec![],
        };

        // Test writing a file
        let result = tool
            .execute(
                &ctx,
                json!({
                    "path": "test.txt",
                    "content": "Hello, world!"
                }),
            )
            .await;
        assert!(result.is_success());
        assert!(result.output().contains("Successfully wrote to"));

        // Verify the file was written
        let file_path = temp_dir.join("test.txt");
        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_file_list_tool() {
        let temp_dir = create_test_dir();

        // Create some test files and directories
        std::fs::write(temp_dir.join("file1.txt"), "").unwrap();
        std::fs::write(temp_dir.join("file2.txt"), "").unwrap();
        std::fs::create_dir(temp_dir.join("subdir")).unwrap();

        let tool = FileListTool;
        let ctx = ToolContext {
            working_directory: temp_dir.clone(),
            permissions: vec![],
        };

        // Test listing directory
        let result = tool.execute(&ctx, json!({})).await;
        assert!(result.is_success());
        let output = result.output();
        assert!(output.contains("file1.txt [file]"));
        assert!(output.contains("file2.txt [file]"));
        assert!(output.contains("subdir [dir]"));
    }

    #[tokio::test]
    async fn test_secure_join_valid_paths() {
        let base = PathBuf::from("/base");

        // Valid relative paths
        assert_eq!(
            secure_join(&base, "file.txt").unwrap(),
            PathBuf::from("/base/file.txt")
        );
        assert_eq!(
            secure_join(&base, "subdir/file.txt").unwrap(),
            PathBuf::from("/base/subdir/file.txt")
        );
        assert_eq!(
            secure_join(&base, "subdir/../file.txt").unwrap(),
            PathBuf::from("/base/file.txt")
        );
        assert_eq!(secure_join(&base, ".").unwrap(), base);
        assert_eq!(secure_join(&base, "").unwrap(), base);
    }

    #[tokio::test]
    async fn test_secure_join_path_traversal() {
        let base = PathBuf::from("/base");

        // Attempted path traversal
        assert!(secure_join(&base, "../../etc/passwd").is_err());
        assert!(secure_join(&base, "../escape").is_err());
        assert!(secure_join(&base, "subdir/../../..").is_err());

        // Absolute paths not allowed
        assert!(secure_join(&base, "/etc/passwd").is_err());
        #[cfg(windows)]
        assert!(secure_join(&base, "C:\\Windows").is_err());
    }

    #[tokio::test]
    async fn test_file_read_nonexistent_path() {
        let temp_dir = create_test_dir();
        let tool = FileReadTool;
        let ctx = ToolContext {
            working_directory: temp_dir,
            permissions: vec![],
        };

        let result = tool.execute(&ctx, json!({"path": "nonexistent.txt"})).await;
        assert!(!result.is_success());
        assert!(result
            .error_message()
            .unwrap()
            .contains("Failed to read file"));
    }

    #[tokio::test]
    async fn test_file_write_missing_parameters() {
        let temp_dir = create_test_dir();
        let tool = FileWriteTool;
        let ctx = ToolContext {
            working_directory: temp_dir,
            permissions: vec![],
        };

        // Missing path
        let result = tool.execute(&ctx, json!({"content": "test"})).await;
        assert!(!result.is_success());
        assert!(result
            .error_message()
            .unwrap()
            .contains("Missing 'path' parameter"));

        // Missing content
        let result = tool.execute(&ctx, json!({"path": "test.txt"})).await;
        assert!(!result.is_success());
        assert!(result
            .error_message()
            .unwrap()
            .contains("Missing 'content' parameter"));
    }

    #[tokio::test]
    async fn test_file_list_path_traversal() {
        let temp_dir = create_test_dir();
        let tool = FileListTool;
        let ctx = ToolContext {
            working_directory: temp_dir,
            permissions: vec![],
        };

        // Attempt to traverse out of directory
        let result = tool.execute(&ctx, json!({"path": "../.."})).await;
        assert!(!result.is_success());
        assert!(result.error_message().unwrap().contains("Invalid path"));
    }
}
