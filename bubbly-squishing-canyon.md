# Implementation Plan: Nano Code Agent

## Context

Create a Claude-Code-like code agent called "nano code" using:
- **AI-SDK** (https://github.com/lazy-hq/aisdk) for AI backend - a Rust crate supporting 73+ AI providers with tool execution capabilities
- **Ratatui** (https://ratatui.rs/) for TUI frontend - a Rust library for terminal user interfaces
- **Plugin architecture** for extensibility
- **MVP features**: Chat interface with file operations, bash command execution, and git operations
- **Configuration**: API keys via environment variables from `.env` file
- **Starting point**: Empty directory `/home/richard/dev/nanocode`

The goal is to create a simple but extensible code agent that can be used for software engineering tasks, similar to Claude Code but with a plugin-based architecture for future extensibility.

## Approach

### Architecture Overview
- **Single Rust crate** with modules (not Cargo workspace) for simplicity
- **Static compile-time plugin registration** using `inventory` crate
- **Minimal sandboxing initially** for bash execution (focus on MVP functionality)
- **Layered configuration** with `.env` file support for API keys
- **Event-driven TUI** with Ratatui and crossterm

### Project Structure (Single Crate)

```
nanocode/
├── Cargo.toml                    # Single crate configuration
├── src/
│   ├── main.rs                   # TUI entry point
│   ├── lib.rs                    # Library exports
│   ├── agent.rs                  # AI agent orchestration
│   ├── plugin.rs                 # Plugin/trait definitions
│   ├── config.rs                 # Configuration management
│   ├── error.rs                  # Error types
│   ├── types.rs                  # Common types (ToolResult, Context)
│   ├── execution.rs              # Tool execution engine
│   ├── ui/                       # TUI components
│   │   ├── mod.rs
│   │   ├── app.rs                # Main application state
│   │   ├── components.rs         # UI widgets
│   │   └── event.rs              # Event handling
│   └── plugins/                  # Built-in plugins as modules
│       ├── mod.rs                # Plugin registration
│       ├── file_ops.rs           # File operation tools
│       ├── bash_exec.rs          # Bash execution tools
│       └── git_ops.rs            # Git operation tools
├── .env.example                  # Example environment variables
└── config/                       # Configuration files
    └── default.toml
```

### Core Abstractions

**Key Traits:**
```rust
// Tool trait - core interface all tools implement
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> ToolParameters; // JSON schema for AI-SDK
    async fn execute(&self, ctx: &ToolContext, params: Value) -> ToolResult;
}

// Plugin trait - collections of tools
pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn tools(&self) -> Vec<Arc<dyn Tool>>;
}
```

**Static Plugin Registration:**
- Use `inventory` crate for compile-time plugin registration
- Each plugin module implements `Plugin` trait and registers via `inventory::submit!`
- Main application discovers plugins via `inventory::iter::<Plugin>`

**Agent Orchestration:**
- `Agent` struct integrates AI-SDK model with registered tools
- Handles conversation history, tool selection, and execution
- Uses AI-SDK's structured output for tool parameter parsing

### Tool Execution Flow

1. User input → TUI → Agent
2. Agent uses AI-SDK to parse input and select tools
3. AI-SDK returns structured tool call with parameters
4. Agent validates parameters against tool schema
5. Agent creates `ToolContext` with working directory and permissions
6. Tool executes with context and parameters
7. Tool returns `ToolResult` (success/failure + output)
8. Agent formats result and updates conversation history
9. TUI displays result to user

### TUI Design

**Layout:**
```
┌─────────────────────────────────────┐
│ Chat History (scrollable)           │
├─────────────────────────────────────┤
│ Current Tool Output                 │
├─────────────────────────────────────┤
│ Input Area                          │
├─────────────────────────────────────┤
│ Status Bar (model, tokens, plugins) │
└─────────────────────────────────────┘
```

**Event System:**
- `crossterm` for terminal input
- Async event loop with `tokio`
- Key bindings: Ctrl+C (exit), Ctrl+R (clear), Tab (autocomplete)

### Configuration Management

**Layered Configuration:**
1. Defaults (compiled-in)
2. System config (`/etc/nanocode/config.toml`)
3. User config (`~/.config/nanocode/config.toml`)
4. Environment variables (`.env` file via `dotenvy`)
5. Command-line arguments

**API Key Configuration:**
- `.env` file with `ANTHROPIC_API_KEY=sk-...` or `OPENAI_API_KEY=sk-...`
- Multiple provider support via AI-SDK feature flags

## Critical Files to Create

**`/home/richard/dev/nanocode/prd.md`** - Design summary document with architecture decisions and MVP features

**`/home/richard/dev/nanocode/Cargo.toml`** - Dependencies: `aisdk`, `ratatui`, `crossterm`, `tokio`, `serde`, `thiserror`, `tracing`, `inventory`, `config`, `dotenvy`

**`/home/richard/dev/nanocode/src/lib.rs`** - Core library entry point with trait definitions and plugin registration

**`/home/richard/dev/nanocode/src/plugin.rs`** - Plugin and Tool trait definitions, plugin registration system

**`/home/richard/dev/nanocode/src/agent.rs`** - AI agent orchestration integrating AI-SDK with tool execution

**`/home/richard/dev/nanocode/src/ui/app.rs`** - Main TUI application state management and event handling

**`/home/richard/dev/nanocode/src/plugins/mod.rs`** - Plugin registration and built-in plugin implementations

**`/home/richard/dev/nanocode/src/plugins/bash_exec.rs`** - Bash execution tools (initially without sandboxing)

## Implementation Sequence

### Phase 1: Foundation (Days 1-2)
1. Create design documentation: Write `prd.md` with design summary
2. Initialize Rust project: `cargo init --name nanocode`
3. Add dependencies to `Cargo.toml`
4. Create core traits (`Tool`, `Plugin`, `Context`) in `src/plugin.rs`
5. Set up configuration system with `.env` support in `src/config.rs`
6. Create error types in `src/error.rs`

### Phase 2: Plugin Implementation (Days 3-4)
6. Implement static plugin registration with `inventory` crate
7. Create file operations plugin (read, write, list) in `src/plugins/file_ops.rs`
8. Create bash execution plugin (basic command execution) in `src/plugins/bash_exec.rs`
9. Create git operations plugin (status, diff) in `src/plugins/git_ops.rs`

### Phase 3: AI Integration (Days 5-6)
10. Integrate AI-SDK with mock model for testing in `src/agent.rs`
11. Implement agent logic for tool selection and execution
12. Add error handling and retry logic
13. Implement conversation history management

### Phase 4: TUI Implementation (Days 7-8)
14. Create basic Ratatui layout and components in `src/ui/`
15. Implement event handling and input processing
16. Connect TUI to agent with async communication
17. Add status indicators and progress reporting

### Phase 5: MVP Polish (Days 9-10)
18. Add configuration validation and reloading
19. Implement comprehensive error reporting
20. Add help system and documentation
21. Basic testing and examples

## Verification

### Testing Strategy
1. **Unit tests** for individual tools and core components
2. **Integration tests** for plugin registration and tool execution
3. **Manual testing** of full workflow:
   - Start application: `cargo run`
   - Configure API key in `.env` file
   - Test file operations: "Read the README.md file"
   - Test bash execution: "List files in current directory"
   - Test git operations: "Show git status"
   - Verify tool outputs appear in TUI

### Success Criteria
- Application starts without errors
- API keys loaded from `.env` file
- All three plugin types (file, bash, git) register successfully
- AI agent can parse user input and select appropriate tools
- Tools execute and return results to TUI
- TUI displays conversation history and tool outputs
- Basic error handling for invalid commands or API failures

### End-to-End Test Commands
```bash
# Set up environment
echo "ANTHROPIC_API_KEY=test-key" > .env
cargo build

# Run application
cargo run

# Expected: TUI starts, shows status bar with loaded plugins
# Test commands in TUI:
# 1. "Read Cargo.toml" → file tool executes
# 2. "Run ls -la" → bash tool executes
# 3. "Show git status" → git tool executes
```

## Dependencies

**Primary:**
- `aisdk = "0.1"` - AI provider abstraction with tool execution
- `ratatui = "0.26"` - TUI framework
- `crossterm = "0.27"` - Terminal input/output
- `tokio = { version = "1.0", features = ["full"] }` - Async runtime

**Supporting:**
- `inventory = "0.3"` - Static plugin registration
- `config = "0.13"` - Configuration management
- `dotenvy = "0.15"` - `.env` file loading
- `serde = { version = "1.0", features = ["derive"] }` - Serialization
- `thiserror = "1.0"` - Error handling
- `tracing = "0.1"` - Logging

## Design Documentation

Create `prd.md` in the project root as a design summary document covering:

### Project Overview
- **Name**: nano code
- **Purpose**: Claude-Code-like code agent with plugin architecture
- **Target Users**: Developers needing AI-assisted coding in terminal
- **Core Value**: Extensible plugin system for tool integration

### Architecture Decisions
- **Language**: Rust (full-stack)
- **AI Backend**: AI-SDK with 73+ provider support
- **Frontend**: Ratatui TUI
- **Plugin System**: Static compile-time registration via `inventory` crate
- **Configuration**: Layered system with `.env` file support

### MVP Features
1. **Chat Interface**: TUI-based conversation with AI agent
2. **File Operations**: Read, write, list files in working directory
3. **Bash Execution**: Run shell commands (initially without sandboxing)
4. **Git Operations**: Status, diff, basic git commands
5. **Plugin Architecture**: Extensible tool system for future additions

### Technical Design
- Single Rust crate with modular structure
- Event-driven TUI with async execution
- Tool execution flow with context management
- Error handling and recovery strategies

### Success Metrics
- Application starts and loads plugins successfully
- All three tool types work end-to-end
- API keys configured via `.env` file
- TUI displays conversation history and tool outputs

## Notes & Trade-offs

1. **Single crate vs workspace**: Chosen for MVP simplicity, can refactor to workspace later if needed
2. **Static plugin registration**: Simpler implementation, dynamic loading can be added later
3. **Minimal sandboxing**: Security can be enhanced after core functionality is working
4. **AI provider abstraction**: AI-SDK supports multiple providers; can start with Claude API similar to Claude Code
5. **Extensibility**: Plugin architecture allows adding new tools without modifying core code