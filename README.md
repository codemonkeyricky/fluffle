# nano code

A Claude-Code-like code agent with plugin architecture, built in Rust.

## Features

- **Plugin architecture:** Extensible tool system with compile-time registration
- **Multiple AI providers:** Supports OpenAI and Anthropic via abstraction layer
- **TUI interface:** Terminal user interface with Ratatui (four-pane layout)
- **Built-in tools:** File operations, bash execution, git operations
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
- Tool outputs appear in the Tool Output pane
- Press Ctrl+C to exit

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
└── plugins/      # Built-in plugins (file_ops, bash_exec, git_ops)
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

## License

MIT