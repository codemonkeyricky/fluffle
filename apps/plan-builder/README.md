# Plan Builder App

This app provides two specialized agents for breaking down plans into tasks and executing them.

## Agents

### Task Agent (`task-agent`)

- **Purpose**: Breaks down high-level plans into discrete, executable tasks.
- **System Prompt**: Instructs the agent to analyze plans, decompose them, delegate tasks using the `builder-agent` tool, and monitor completion.
- **Tools**:
  - `builder-agent`: Spawns a builder agent to execute a specific task (dynamic profile tool).
  - `file_read`: Read plan documents or other files.
  - `file_list`: List files to understand the workspace.

### Builder Agent (`builder-agent`)

- **Purpose**: Executes individual tasks using available tools.
- **System Prompt**: Instructs the agent to perform tasks step‑by‑step, verify success, and report completion.
- **Tools**:
  - `file_read`, `file_write`, `file_list`: File operations.
  - `bash_exec`: Execute shell commands.
  - `git_status`, `git_diff`: Git operations.

## Workflow

1. Start the app with `--app plan-builder`. The default agent is the **task agent**.
2. Provide a high‑level plan (e.g., “Create a simple web server with Express.js”).
3. The task agent will:
   - Analyze the plan and identify required tasks.
    - For each task, call the `builder-agent` tool with a clear description.
   - Each delegation spawns a **builder agent** that executes the task and returns a result.
   - Wait for the builder agent to finish before proceeding.
   - If a task fails, decide whether to retry, adjust, or abort.
4. After all tasks are completed, the task agent provides a final summary.

## Usage

```bash
nanocode --app plan-builder
```

In headless mode:

```bash
nanocode --app plan-builder --headless -p "Create a simple REST API"
```

## Configuration

- Agent profiles are defined in `apps/plan‑builder/agents/`.
- Tools are loaded from `apps/plan‑builder/tools/` (none by default; uses built‑in tools).
- The default agent profile is `task‑agent` for this app (changed from `generalist`).

## Customization

- Edit the agent JSON files to adjust system prompts, tools, or config overrides.
- Add custom dynamic tools by placing JSON definitions in `apps/plan‑builder/tools/`.
- The `builder-agent` tool is dynamically created by the profile plugin based on the `builder-agent` profile.