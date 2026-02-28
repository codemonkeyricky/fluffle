# Fluffle: A Microcontext Framework

[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)
![Status](https://img.shields.io/badge/Status-alpha-yellow)

Fluffle is a microcontext framework for building AI-powered applications with a focus on maintaining high-quality reasoning through small, focused context windows. It enables the creation of agentic systems where problems are decomposed into smaller subproblems, each solved within a limited context budget—critical for local inferencing with constrained VRAM.

> **Background**: This project emerged from the observation that AI‑created prototypes often suffer from “attention drift” as projects grow. By borrowing concepts from operating‑system design (process forking, isolated contexts), Fluffle keeps each agent’s context small and focused, leading to higher‑quality outputs. For a detailed discussion, see the [introductory blog post](https://github.com/codemonkeyricky/fluffle).

## Features

- **Microcontext architecture**: Each agent operates within a small, isolated context window, preserving attention fidelity.
- **Dynamic agent forking**: Agents can spawn sub‑agents with different profiles and tool sets, enabling problem decomposition.
- **JSON‑driven configuration**: Define agents and tools via JSON files—no Rust code required for basic extensions.
- **Runtime loader**: Tools and agents are discovered automatically from built‑in and user directories.
- **Multiple AI providers**: Supports OpenAI and Anthropic out of the box; designed to be extensible.
- **Secure execution**: Tools can declare `secure_parameters` to prevent directory‑traversal attacks.
- **TUI and headless modes**: Interactive terminal UI or script‑friendly stdin/stdout operation.

## Overview

Modern AI models generate tokens sequentially, with each output token becoming part of the input for the next prediction. As context grows, the fixed precision of token embeddings inevitably dilutes the model's attention over time. The key to high‑quality AI output is to work with many small, focused contexts.

Fluffle addresses this “context rot” by drawing a parallel to operating‑system process design:
- **Context ID** ↔ Process ID
- **Agents** ↔ Applications
- **Forking** ↔ Process forking
- **Tool capabilities** ↔ Process capabilities

The framework starts with a single long‑running context (cid=1) that can fork additional contexts as specialized agents. Each agent has its own set of tools, system prompt, and working environment. Forking can be recursive, enabling complex problem decomposition while preserving attention fidelity.

## Architecture

### Core Concepts
- **Microcontext**: A small, isolated context window with a dedicated agent, tools, and conversation history.
- **Agent**: An AI entity that operates within a microcontext. Agents are defined by a profile (JSON) that specifies their system prompt, allowed tools, and configuration overrides.
- **Tool**: A capability that an agent can invoke (e.g., reading files, executing shell commands, spawning sub‑agents). Tools are also defined via JSON and can be dynamically loaded.
- **Profile**: A blueprint for an agent, stored as a JSON file in `apps/<app>/agents/`. Profiles list the tools the agent can use and may override global configuration (temperature, model, etc.).
- **Runtime Loader**: Fluffle discovers agents and tools by scanning built‑in and user‑configurable directories, allowing users to extend the system without modifying the core code.

### How It Works
1. The main agent (cid=1) receives a task.
2. The agent may fork sub‑agents (e.g., an explorer to examine the codebase, a security‑auditor to check for vulnerabilities).
3. Each sub‑agent operates within its own microcontext, with a limited set of tools and a focused system prompt.
4. Results are communicated back to the parent agent via channels.
5. The parent agent synthesizes the results and delivers the final output.

This design ensures that no single context grows too large, preserving the model’s attention and reasoning quality.

## Getting Started

### Prerequisites
- Rust toolchain (stable, edition 2021)
- An API key for either OpenAI or Anthropic (set via environment variable or `.env` file)

### Installation
Clone the repository and build with Cargo:

```bash
git clone https://github.com/codemonkeyricky/fluffle.git
cd fluffle
cargo build --release
```

The binary (`nanocode`) will be available at `target/release/nanocode`. (The framework is called Fluffle; the executable is named `nanocode` for historical reasons.)

### Running the Example Coding App
Fluffle includes a ready‑to‑use software‑engineering system. To start it:

```bash
cargo run -- --app coding
```

This launches the TUI interface with the “coding” app. You can also run in headless mode (stdin/stdout) by providing a prompt:

```bash
cargo run -- --headless --prompt "List all files in the current directory"
```

To use a different built‑in app (if you’ve created one), specify its name with `--app <app-name>`.

### Configuration
Create a `.env` file in the project root (or set environment variables):

```bash
OPENAI_API_KEY=sk-...          # For OpenAI models
ANTHROPIC_API_KEY=sk-ant-...   # For Anthropic models
```

You can also adjust settings via a configuration file. The system looks for (in order):
1. `~/.config/nanocode/default.toml` (global user configuration)
2. `nanocode.toml` in the current directory (local project configuration)

See `src/config.rs` for the available options (model, provider, temperature, max_tokens, max_tool_iterations).

## Creating Custom Agents and Tools

Fluffle is designed to be extended by writing JSON definitions—no Rust code required.

### Adding a New Agent Profile
1. Create a JSON file in `apps/<app>/agents/` (e.g., `apps/coding/agents/code-reviewer.json`).
2. Define the agent’s name, description, system prompt, allowed tools, and optional config overrides:

```json
{
  "name": "code-reviewer",
  "description": "Specializes in reviewing code for best practices and potential bugs",
  "system_prompt": "You are a meticulous code reviewer...",
  "tools": ["file_read", "file_list", "bash_exec"],
  "config_overrides": {
    "temperature": 0.2,
    "max_tool_iterations": 20
  }
}
```

3. The agent will automatically appear as a tool that other agents can invoke (the profile name becomes a tool name).

### Adding a New Tool
1. Create a JSON file in `apps/<app>/tools/` (e.g., `apps/coding/tools/curl.json`).
2. Define the tool’s name, description, parameter schema, execution type, and any secure parameters. The `execution.type` can be `"bash"` (with a `command_template`) or `"http"` (not yet implemented):

```json
{
  "name": "curl",
  "description": "Make an HTTP request",
  "parameters": {
    "type": "object",
    "properties": {
      "url": {
        "type": "string",
        "description": "URL to fetch"
      }
    },
    "required": ["url"]
  },
  "execution": {
    "type": "bash",
    "command_template": "curl -s {{url}}"
  },
  "secure_parameters": ["url"]
}
```

3. The tool will be automatically loaded and available to all agents that include it in their profile.

**Security note**: The `secure_parameters` field lists parameter names that will be securely joined to the agent’s working directory, preventing directory‑traversal attacks. Use this for any file‑path parameters.

### Building Your Own App
You can create entirely new applications by adding a subdirectory under `apps/` with its own `agents/` and `tools/` folders. Launch your app with `--app <your-app-name>`.

**Advanced**: For maximum flexibility, you can write tools in Rust by implementing the `Tool` trait and registering them via the plugin inventory (see `src/plugin.rs`). This allows you to integrate arbitrary libraries and perform complex operations beyond shell commands.

## Example: The Coding App

The included `coding` app demonstrates a complete software‑engineering assistant with three agent profiles:

1. **Generalist** – full access to all tools; the default “do‑everything” agent.
2. **Explorer** – specialized for exploring codebases (read, list, git status/diff).
3. **Security‑Auditor** – focused on security review (lower temperature, limited toolset).

Each agent can fork the others, enabling collaborative problem‑solving. For instance, the generalist might ask the explorer to survey a repository, then ask the security‑auditor to examine suspicious files.

## Development

### Project Structure
```
fluffle/
├── src/                    # Core Rust implementation
│   ├── agent.rs           # Agent definition and microcontext management
│   ├── agent_thread.rs    # Thread spawning and channel communication
│   ├── ai/                # AI provider abstractions (OpenAI, Anthropic)
│   ├── loaders/           # Dynamic loading of agents and tools
│   ├── plugin.rs          # Plugin system for tools
│   ├── ui/                # TUI and headless interfaces
│   └── ...
├── apps/                  # Application definitions
│   └── coding/            # Example software‑engineering app
│       ├── agents/        * JSON profiles
│       └── tools/         * JSON tool definitions
├── Cargo.toml
└── README.md
```

### Running Tests
```bash
cargo test
```

### Linting
```bash
cargo clippy --all-targets --all-features
```

### Debugging
When running agents, detailed logs are written to `agent.log` in the current directory. The log includes token usage, tool calls, and model responses. For headless runs, you can also increase verbosity by setting the `RUST_LOG` environment variable (e.g., `RUST_LOG=debug`).

## Why “Fluffle”?

The name evokes rapid forking—like a fluffle of rabbits multiplying—and reflects the framework’s ability to quickly spawn focused microcontexts. It also hints at the fluffy, lightweight nature of each context.

## Roadmap

- [ ] Support for additional AI providers (local LLMs via Ollama, Gemini, etc.)
- [ ] More built‑in tools (HTTP, database queries, etc.)
- [ ] Persistent context storage across sessions
- [ ] Visual graph of agent interactions
- [ ] Plugin system for Rust‑native tools

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License

Fluffle is distributed under the MIT License. See the [LICENSE](LICENSE) file for details.
