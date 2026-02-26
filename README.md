# nano code

A Claude-Code-like code agent with plugin architecture, built in Rust.

## Features

- **Plugin architecture:** Extensible tool system with compile-time registration
- **Multiple AI providers:** Supports OpenAI and Anthropic via abstraction layer
- **TUI interface:** Terminal user interface with Ratatui (four-pane layout)
- **Built-in tools:** File operations, bash execution, git operations, subagent creation, codebase exploration
- **Dynamic tool loading:** Define custom tools via JSON files without writing Rust code
- **Configuration:** Layered config system with `.env` support

## Installation

Requires Rust (install via [rustup](https://rustup.rs/)).

```bash
git clone <repository>
cd nanocode
cargo build --release
```

## Configuration

1. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` and add your API key:
   ```
   OPENAI_API_KEY=sk-your-key-here
   # OR
   ANTHROPIC_API_KEY=sk-your-key-here
   ```

3. Configure provider in `config/default.toml` or via environment:
   ```bash
   export NANOCODE_PROVIDER=openai  # or anthropic
   ```

## Usage

```bash
cargo run
```

In the TUI:
- Type commands like "Read Cargo.toml" or "Run ls -la"
- Press Enter to execute
- See results in output pane

## Subagent System

The agent supports creating subagents for specialized tasks. Two new tools are available:

### Task Tool
Create a subagent with a custom system prompt:
```
task(description="Find all TODO comments in the codebase", system_prompt="You are a code reviewer specialized in identifying technical debt.")
```

### Explore Tool
Specialized subagent for exploring codebases with a predefined system prompt:
```
explore(description="Understand the project structure and main components")
```

Subagents run synchronously with isolated conversation history and inherit the parent agent's configuration and tools. They return a summarized result to the main conversation.

## Dynamic Tools

You can define custom tools via JSON files without writing Rust code. Place tool definition files in `~/.config/nanocode/tools/` with a `.json` extension.

### Tool Definition Format

```json
{
  "name": "echo_tool",
  "description": "Echoes the input text",
  "parameters": {
    "type": "object",
    "properties": {
      "text": {
        "type": "string",
        "description": "Text to echo"
      }
    },
    "required": ["text"]
  },
  "execution": {
    "type": "bash",
    "command_template": "echo {{text}}"
  }
}
```

Supported execution types:
- **bash**: Executes a bash command with parameter substitution using `{{param}}` placeholders.
- **http**: (Planned) Makes HTTP requests.

### Example: List Files Tool

```json
{
  "name": "list_files",
  "description": "List files in a directory",
  "parameters": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Directory path",
        "default": "."
      }
    },
    "required": []
  },
  "execution": {
    "type": "bash",
    "command_template": "ls -la {{path}}"
  }
}
```

Tools are loaded at startup and appear in the agent's available tool list. You can reference them in agent profiles.

### Migrated Built-in Plugins

Several built-in plugins have been migrated to JSON tool definitions:

1. **bash_exec** → `bash_exec.json`: Generic bash command execution
2. **git_status** → `git_status.json`: Git status command  
3. **git_diff** → `git_diff.json`: Git diff with optional path parameter

These JSON files are automatically loaded from `~/.config/nanocode/tools/`. The corresponding Rust plugins are disabled in favor of the dynamic versions.

**Note**: File operations (`file_read`, `file_write`, `file_list`) remain as Rust plugins due to security requirements and complex parameter handling. They may be migrated in the future with enhanced dynamic tool capabilities.

## Development

### Project Structure

```
src/
├── agent.rs      # AI agent orchestration with tool calling
├── ai/           # AI provider abstraction (OpenAI, Anthropic)
├── config.rs     # Configuration management
├── error.rs      # Error types
├── plugin.rs     # Plugin/trait definitions
├── types.rs      # Common types
├── ui/           # TUI components (app, components, event)
└── plugins/      # Built-in plugins (file_ops, bash_exec, git_ops, task, explore)
```

### Adding a New Plugin

1. Create a new module in `src/plugins/`
2. Implement the `Plugin` and `Tool` traits
3. Register with `inventory::submit!`
4. Tools will be automatically discovered at runtime

Example plugin structure:
```rust
use std::sync::Arc;
use async_trait::async_trait;
use crate::plugin::{Plugin, Tool};
use crate::types::{ToolContext, ToolResult, ToolParameters};
use serde_json::json;
use inventory;

pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &'static str {
        "my_tool"
    }

    fn description(&self) -> &'static str {
        "A custom tool that does something useful"
    }

    fn parameters(&self) -> ToolParameters {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _ctx: &ToolContext, _params: ToolParameters) -> ToolResult {
        Ok(ToolResult::success(
            "Custom tool executed successfully"
        ))
    }
}

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &'static str { "my_plugin" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![Arc::new(MyTool)]
    }
}

static MY_PLUGIN: MyPlugin = MyPlugin;

inventory::submit! {
    &MY_PLUGIN as &'static dyn Plugin
}
```

For simpler tool definitions without Rust coding, see [Dynamic Tools](#dynamic-tools) above.

## License

MIT