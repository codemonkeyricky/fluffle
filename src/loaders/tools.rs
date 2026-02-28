use crate::plugin::{Plugin, Tool};
use crate::types::{ToolContext, ToolParameters, ToolResult};
use anyhow::{anyhow, Context, Result as AnyResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicToolDef {
    /// Unique name of the tool
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON schema for tool parameters
    pub parameters: ToolParameters,
    /// Execution configuration
    pub execution: ExecutionDef,
    /// Optional list of parameter names that should be securely joined to working directory
    #[serde(default)]
    pub secure_parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionDef {
    /// Execute a bash command with parameter substitution
    Bash {
        /// Command template with {{param}} placeholders
        command_template: String,
    },
    /// Make an HTTP request
    Http {
        /// URL template with {{param}} placeholders
        url: String,
        /// HTTP method (GET, POST, etc.)
        method: String,
        /// Optional headers mapping
        headers: Option<HashMap<String, String>>,
        /// Optional body template (for POST, PUT)
        body_template: Option<String>,
    },
}

/// Escape a string for use in a bash single-quoted string.
/// Turns ' into '\'' (end string, escaped quote, restart string).
fn bash_escape_single_quoted(s: &str) -> String {
    s.replace('\'', "'\\''")
}

/// Render a command template by replacing {{param}} placeholders with values from params.
/// If a parameter is in secure_parameters, securely join it to the working directory.
/// String parameters are shell-escaped for single quotes.
/// params must be a JSON object.
fn render_command(
    template: &str,
    params: &Value,
    ctx: &ToolContext,
    secure_parameters: &[String],
) -> AnyResult<String> {
    let mut result = template.to_string();
    let param_map = params
        .as_object()
        .context("Parameters must be a JSON object")?;
    let secure_set: HashSet<_> = secure_parameters.iter().collect();

    for (key, value) in param_map {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            Value::String(s) => {
                if secure_set.contains(key) {
                    // Securely join path to working directory
                    let secure_path = secure_join(&ctx.working_directory, s)
                        .map_err(|e| anyhow!("Failed to securely join path '{}': {}", s, e))?;
                    bash_escape_single_quoted(&secure_path.to_string_lossy())
                } else {
                    bash_escape_single_quoted(s)
                }
            }
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(_) | Value::Object(_) => {
                return Err(anyhow::anyhow!(
                    "Unsupported parameter type for placeholder {}: array or object",
                    key
                ));
            }
        };
        result = result.replace(&placeholder, &replacement);
    }
    Ok(result)
}

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn name(&self) -> &'static str {
        "tools"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        match load_tools() {
            Ok(defs) => defs
                .into_iter()
                .map(|def| Arc::new(DynamicTool::new(def)) as Arc<dyn Tool>)
                .collect(),
            Err(e) => {
                tracing::error!("Failed to load dynamic tools: {}", e);
                Vec::new()
            }
        }
    }
}

struct DynamicTool {
    def: DynamicToolDef,
}

impl DynamicTool {
    fn new(def: DynamicToolDef) -> Self {
        Self { def }
    }
}

#[async_trait]
impl Tool for DynamicTool {
    fn name(&self) -> &'static str {
        // We need to leak the string to get static reference
        Box::leak(self.def.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.def.description.clone().into_boxed_str())
    }

    fn parameters(&self) -> ToolParameters {
        self.def.parameters.clone()
    }

    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult {
        match &self.def.execution {
            ExecutionDef::Bash { command_template } => {
                self.execute_bash(ctx, params, command_template).await
            }
            ExecutionDef::Http {
                url,
                method,
                headers,
                body_template,
            } => {
                self.execute_http(ctx, params, url, method, headers, body_template)
                    .await
            }
        }
    }
}

impl DynamicTool {
    async fn execute_bash(
        &self,
        ctx: &ToolContext,
        params: ToolParameters,
        command_template: &str,
    ) -> ToolResult {
        // Render command template with parameters
        let command =
            match render_command(command_template, &params, ctx, &self.def.secure_parameters) {
                Ok(cmd) => cmd,
                Err(e) => {
                    return ToolResult::error(format!("Failed to render command template: {}", e))
                }
            };

        // Execute command
        match Command::new("bash")
            .arg("-c")
            .arg(&command)
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

    async fn execute_http(
        &self,
        _ctx: &ToolContext,
        _params: ToolParameters,
        _url: &str,
        _method: &str,
        _headers: &Option<HashMap<String, String>>,
        _body_template: &Option<String>,
    ) -> ToolResult {
        ToolResult::error("HTTP execution not yet implemented")
    }
}

/// Load dynamic tools from built-in and user directories
fn load_tools() -> AnyResult<Vec<DynamicToolDef>> {
    let mut tools_map = std::collections::HashMap::new();

    // Load built-in tools first
    if let Ok(builtin_dir) = builtin_tools_dir() {
        if builtin_dir.exists() {
            load_tools_from_dir(&builtin_dir, &mut tools_map)?;
        }
    }

    // Load user tools (override built-in ones)
    let user_dir = user_tools_dir()?;
    // Create directory if it doesn't exist (no error if already exists)
    let _ = fs::create_dir_all(&user_dir);

    if user_dir.exists() {
        load_tools_from_dir(&user_dir, &mut tools_map)?;
    }

    // Convert to vector, preserving insertion order (built-in then user)
    let tools: Vec<DynamicToolDef> = tools_map.into_values().collect();
    Ok(tools)
}

/// Load tools from a directory into a map (name -> tool)
fn load_tools_from_dir(
    dir: &Path,
    tools_map: &mut std::collections::HashMap<String, DynamicToolDef>,
) -> AnyResult<()> {
    for entry in fs::read_dir(dir).context("Failed to read tools directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.extension().map(|ext| ext == "json").unwrap_or(false) {
            match load_tool_from_file(&path) {
                Ok(tool) => {
                    tools_map.insert(tool.name.clone(), tool);
                }
                Err(e) => tracing::warn!("Failed to load tool from {}: {}", path.display(), e),
            }
        }
    }
    Ok(())
}

fn load_tool_from_file(path: &Path) -> AnyResult<DynamicToolDef> {
    let content = fs::read_to_string(path).context("Failed to read tool file")?;
    let tool: DynamicToolDef =
        serde_json::from_str(&content).context("Failed to parse tool JSON")?;
    Ok(tool)
}

/// Get built-in tools directory (relative to source)
fn builtin_tools_dir() -> AnyResult<PathBuf> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("app");
    path.push("tools");
    Ok(path)
}

/// Get user tools directory (~/.config/nanocode/tools)
fn user_tools_dir() -> AnyResult<PathBuf> {
    let mut path = dirs::config_dir().context("Could not find config directory")?;
    path.push("nanocode");
    path.push("tools");
    Ok(path)
}

/// Backward compatibility alias
fn tools_dir() -> AnyResult<PathBuf> {
    user_tools_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_command() {
        let params = json!({
            "name": "world",
            "count": 5,
            "flag": true
        });
        let template = "Hello {{name}}! Count: {{count}}, flag: {{flag}}";
        let ctx = ToolContext {
            working_directory: PathBuf::from("/tmp"),
            permissions: vec![],
            agent_to_ui_tx: None,
        };
        let result = render_command(template, &params, &ctx, &[]).unwrap();
        assert_eq!(result, "Hello world! Count: 5, flag: true");
    }

    #[test]
    fn test_render_command_missing_placeholder() {
        let params = json!({"name": "world"});
        let template = "Hello {{name}}! {{missing}}";
        let ctx = ToolContext {
            working_directory: PathBuf::from("/tmp"),
            permissions: vec![],
            agent_to_ui_tx: None,
        };
        // missing placeholder should remain unchanged
        let result = render_command(template, &params, &ctx, &[]).unwrap();
        assert_eq!(result, "Hello world! {{missing}}");
    }

    #[test]
    fn test_render_command_invalid_params() {
        let params = json!([1, 2, 3]); // array not object
        let template = "Hello";
        let ctx = ToolContext {
            working_directory: PathBuf::from("/tmp"),
            permissions: vec![],
            agent_to_ui_tx: None,
        };
        let result = render_command(template, &params, &ctx, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_command_secure_parameter() {
        let params = json!({
            "path": "subdir/file.txt",
            "other": "value"
        });
        let template = "cat {{path}} && echo {{other}}";
        let ctx = ToolContext {
            working_directory: PathBuf::from("/base"),
            permissions: vec![],
            agent_to_ui_tx: None,
        };
        let secure_params = vec!["path".to_string()];
        let result = render_command(template, &params, &ctx, &secure_params).unwrap();
        // On Unix, path separator is /
        assert_eq!(result, "cat /base/subdir/file.txt && echo value");
    }

    #[test]
    fn test_render_command_path_traversal_blocked() {
        let params = json!({
            "path": "../../etc/passwd"
        });
        let template = "cat {{path}}";
        let ctx = ToolContext {
            working_directory: PathBuf::from("/base"),
            permissions: vec![],
            agent_to_ui_tx: None,
        };
        let secure_params = vec!["path".to_string()];
        let result = render_command(template, &params, &ctx, &secure_params);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Path traversal") || err_msg.contains("path traversal"));
    }

    #[test]
    fn test_dynamic_tool_def_deserialization() {
        let json = r#"
        {
            "name": "echo_tool",
            "description": "Echoes input",
            "parameters": {
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                },
                "required": ["text"]
            },
            "execution": {
                "type": "bash",
                "command_template": "echo {{text}}"
            }
        }
        "#;
        let tool: DynamicToolDef = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "echo_tool");
        assert_eq!(tool.description, "Echoes input");
        if let ExecutionDef::Bash { command_template } = tool.execution {
            assert_eq!(command_template, "echo {{text}}");
        } else {
            panic!("Expected Bash execution");
        }
    }

    // Note: Integration tests requiring file system are omitted for simplicity.
}

static TOOLS_PLUGIN: ToolsPlugin = ToolsPlugin;

inventory::submit! {
    &TOOLS_PLUGIN as &'static dyn Plugin
}
