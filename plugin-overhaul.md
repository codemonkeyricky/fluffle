# Plugin Overhaul: User-Defined Agent Profiles as JSON Plugins

## Overview

This document outlines the design for extending nanocode's plugin system to support user-defined agent profiles as JSON plugins. The current system has compile-time plugin registration with two specialized agent types (general and explore). The new system will allow users to define unlimited agent types via JSON profiles loaded at runtime.

## Design Goals

1. **Extensibility**: Users can define unlimited agent types without recompilation
2. **Runtime Flexibility**: Profiles loaded from filesystem at startup
3. **Backward Compatibility**: Existing behavior preserved, explore agent becomes a profile
4. **Profile Nesting**: Profiles can reference other profiles as tools (unlimited nesting)
5. **Simple Configuration**: JSON schema with clear structure
6. **Tool Filtering**: Whitelist-based tool access control

## Architecture

### Current Architecture
- Plugins registered at compile-time via `inventory::submit!`
- Tools discovered via plugin inventory scanning
- Two agent types: general (default) and explore (specialized subagent)
- System prompts set when creating subagents

### New Architecture
- **AgentProfile**: Struct representing a user-defined agent type
- **ProfileLoader**: Loads profiles from filesystem at runtime
- **ProfilePlugin**: Dynamic plugin that generates tools from loaded profiles
- **ProfileRegistry**: Central registry mapping profile names to configurations
- **Tool Filtering**: Agent creation filters tools based on profile whitelist

## Implementation Details

### 1. Profile Definition (`src/agent_profile.rs`)

```rust
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
```

### 2. Profile Loading (`src/profile_loader.rs`)

Load profiles from two locations:
1. **Built-in profiles**: `src/agent_profiles/` (relative to source directory)
2. **User profiles**: `~/.config/nanocode/agents/` (user config directory)

Loading algorithm:
1. Scan both directories for `.json` files
2. Parse and validate JSON against schema
3. Check for duplicate names (user profiles override built-in)
4. Store in `ProfileRegistry`

### 3. Profile Plugin (`src/plugins/agent_profile.rs`)

Creates dynamic tools for each loaded profile:

```rust
struct AgentProfilePlugin {
    profiles: HashMap<String, AgentProfile>,
}

impl Plugin for AgentProfilePlugin {
    fn name(&self) -> &'static str { "agent_profile" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.profiles.values()
            .map(|profile| {
                Arc::new(ProfileTool::new(profile.clone())) as Arc<dyn Tool>
            })
            .collect()
    }
}

struct ProfileTool {
    profile: AgentProfile,
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
        // Create agent with profile configuration
        let description = params.get("description").and_then(|d| d.as_str())
            .unwrap_or("");
        
        // Load config and create agent with profile
        let config = Config::load().await?;
        let mut agent = Agent::new_with_profile(&self.profile.name, config)?;
        agent.set_context(ctx.clone());
        
        // Run the task
        match agent.process(description).await {
            Ok(summary) => ToolResult::success(summary),
            Err(e) => ToolResult::error(format!("Profile agent failed: {}", e)),
        }
    }
}
```

### 4. Agent Extension (`src/agent.rs`)

Add profile support to Agent:

```rust
impl Agent {
    /// Create agent with a specific profile
    pub fn new_with_profile(profile_name: &str, config: Config) -> Result<Self> {
        let profile = PROFILE_REGISTRY.get(profile_name)
            .ok_or_else(|| Error::Agent(format!("Profile not found: {}", profile_name)))?;
        
        // Create base agent
        let mut agent = Self::new(config)?;
        
        // Apply profile configuration
        agent.apply_profile(profile)?;
        
        Ok(agent)
    }
    
    /// Apply profile settings to existing agent
    fn apply_profile(&mut self, profile: &AgentProfile) -> Result<()> {
        // Filter tools based on profile whitelist
        self.filter_tools(&profile.tools)?;
        
        // Set system prompt
        self.system_prompt = Some(profile.system_prompt.clone());
        
        // Apply config overrides
        self.apply_config_overrides(&profile.config_overrides)?;
        
        Ok(())
    }
    
    /// Filter tools to only include those in the whitelist
    fn filter_tools(&mut self, tool_names: &[String]) -> Result<()> {
        let available_tools: HashMap<_, _> = self.tools.iter()
            .map(|t| (t.name().to_string(), t.clone()))
            .collect();
        
        let mut filtered = Vec::new();
        for name in tool_names {
            if let Some(tool) = available_tools.get(name) {
                filtered.push(tool.clone());
            } else {
                // Check if it's a profile tool
                if PROFILE_REGISTRY.contains_key(name) {
                    // Profile tools are handled separately by the profile plugin
                    // They'll be available through their own tool registration
                    continue;
                }
                // Return error for unknown tools
                return Err(Error::Agent(format!("Unknown tool in profile: {}", name)));
            }
        }
        
        self.tools = filtered;
        Ok(())
    }
}
```

### 5. Default "Generalist" Profile

Create `src/agent_profiles/generalist.json`:

```json
{
  "name": "generalist",
  "description": "General-purpose agent with access to all tools",
  "system_prompt": "You are a helpful AI assistant with access to various tools. Use the available tools to help the user with their tasks.",
  "tools": [
    "file_ops",
    "bash_exec", 
    "git_ops",
    "task",
    "explore"
  ]
}
```

The generalist agent becomes the default when no profile is specified.

### 6. Updated Explore Agent

Convert existing explore agent to a profile `src/agent_profiles/explorer.json`:

```json
{
  "name": "explorer",
  "description": "Agent specialized in exploring codebases",
  "system_prompt": "You are an expert codebase explorer. Your task is to explore a codebase, understand its structure, and provide insights...",
  "tools": ["file_ops", "bash_exec", "git_ops"],
  "config_overrides": {
    "temperature": 0.2,
    "max_tool_iterations": 30
  }
}
```

## JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["name", "description", "system_prompt", "tools"],
  "properties": {
    "name": {
      "type": "string",
      "description": "Unique identifier for the profile"
    },
    "description": {
      "type": "string",
      "description": "Human-readable description of the agent type"
    },
    "system_prompt": {
      "type": "string",
      "description": "Custom system prompt for this agent"
    },
    "tools": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "List of tool names this agent can access"
    },
    "config_overrides": {
      "type": "object",
      "additionalProperties": true,
      "description": "Optional configuration overrides"
    }
  }
}
```

## Loading Mechanism

### Profile Discovery Order
1. `src/agent_profiles/` (built-in, relative to source)
2. `~/.config/nanocode/agents/` (user config)
3. User profiles override built-in profiles with same names

### Profile Validation
- Basic JSON schema validation
- Check for required fields
- Verify tool names exist (warning for unknown tools)
- Allow profile names in tools list for nesting

### Profile Registry
Global registry accessible via `PROFILE_REGISTRY` static:

```rust
lazy_static::lazy_static! {
    static ref PROFILE_REGISTRY: RwLock<HashMap<String, AgentProfile>> = 
        RwLock::new(HashMap::new());
}
```

## Profile Nesting

Profiles can include other profile names in their `tools` list, enabling composition:

```json
{
  "name": "code-reviewer",
  "description": "Agent for code review tasks",
  "system_prompt": "You are a code review expert...",
  "tools": ["file_ops", "bash_exec", "explorer"]
}
```

Nesting is unlimited but users must avoid circular dependencies.

## Tool Filtering Behavior

- **Whitelist-only**: Only tools listed in `tools` array are available
- **Profile tools**: When a profile name appears in tools list, the corresponding profile tool is available
- **Unknown tools**: Warning logged, tool omitted from filtered list
- **Empty tools list**: Agent has no tools (useful for pure conversation agents)

## Configuration Overrides

Profiles can override agent configuration:

```json
{
  "config_overrides": {
    "temperature": 0.2,
    "max_tokens": 2048,
    "max_tool_iterations": 20
  }
}
```

Supported overrides:
- `temperature` (f32)
- `max_tokens` (u32)
- `max_tool_iterations` (u32)
- Other config fields as needed

## Migration Plan

### Phase 1: Core Infrastructure
1. Create `AgentProfile` struct and serialization
2. Implement `ProfileLoader` and `ProfileRegistry`
3. Create `agent_profiles/` directory with sample profiles

### Phase 2: Profile Plugin
1. Implement `AgentProfilePlugin` with dynamic tool generation
2. Integrate profile loading into plugin initialization
3. Update `Agent` struct with profile support methods

### Phase 3: Default Profiles
1. Convert explore agent to profile-based
2. Create generalist profile with all tools
3. Update main agent creation to use generalist profile by default

### Phase 4: Testing & Documentation
1. Add unit tests for profile loading and filtering
2. Create example user profiles
3. Update README with profile usage documentation

## Example Profiles

### Security Auditor
```json
{
  "name": "security-auditor",
  "description": "Agent specialized in security code review",
  "system_prompt": "You are a security expert reviewing code for vulnerabilities...",
  "tools": ["file_ops", "bash_exec", "git_ops"],
  "config_overrides": {
    "temperature": 0.1
  }
}
```

### Documentation Writer
```json
{
  "name": "documentation-writer",
  "description": "Agent for writing documentation",
  "system_prompt": "You are a technical writer creating clear documentation...",
  "tools": ["file_ops"]
}
```

### Testing Specialist
```json
{
  "name": "testing-specialist",
  "description": "Agent focused on writing and running tests",
  "system_prompt": "You are a testing expert...",
  "tools": ["file_ops", "bash_exec", "testing-tool"]
}
```

## Benefits

1. **User Empowerment**: Users define custom agent types without coding
2. **Runtime Flexibility**: New profiles added without recompilation
3. **Composition**: Profiles can build on each other via nesting
4. **Consistency**: All agents use same underlying plugin system
5. **Configurable**: Profiles can fine-tune agent behavior
6. **Discoverable**: Profile tools appear in agent's tool list automatically

## Open Questions

1. Should profiles support tool parameter defaults?
2. How to handle profile versioning?
3. Should there be a profile validation CLI tool?
4. How to handle profile dependencies (e.g., profile A requires profile B)?

## Next Steps

1. Implement `AgentProfile` struct
2. Create profile loading infrastructure
3. Develop profile plugin with dynamic tool generation
4. Convert explore agent to profile-based
5. Test profile nesting and tool filtering
6. Document profile creation and usage