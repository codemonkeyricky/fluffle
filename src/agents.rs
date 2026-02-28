use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Unique identifier (e.g., "explorer", "code-reviewer")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Custom system prompt for this agent type
    pub system_prompt: String,

    /// List of tool names this agent can access
    /// Can include both built-in tools and other profile names
    pub tools: Vec<String>,

    /// Optional configuration overrides
    #[serde(default)]
    pub config_overrides: HashMap<String, Value>,
}
