# Manus: Planning with Files Agents

This module implements the "planning with files" methodology using specialized agents that work with three markdown files: `todos.md`, `findings.md`, and `progress.md`.

## Agents

### Dispatcher
- **Role**: Route to planner — creates a new plan or continues an existing one.
- **Tools**: `bash_exec`, `file_list`, `planner`
- **Behavior**: Checks for existing planning files and delegates to planner.

### Planner
- **Role**: Orchestrate plan creation and execution via a strict FSM algorithm.
- **Tools**: `file_read`, `file_write`, `file_edit`, `explorer`, `loop`
- **Behavior**: Reads todos.md and the findings.md Summary each iteration. If no plan exists, calls explorer then writes todos.md. For each pending task, optionally calls explorer for context, then delegates to loop. Marks tasks done, archives completed phases to `todos_archive.md`.

### Explorer
- **Role**: Codebase research and findings collection.
- **Tools**: `file_read`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `append_to_findings`
- **Behavior**: Reads findings.md Summary before exploring. Appends new findings, then rewrites the Summary section with the 3–5 most critical facts.

### Loop
- **Role**: Wrap worker in a retry loop with external bash verification.
- **Tools**: `bash_exec`, `worker`
- **Behavior**: Builds a structured `## Task / ## Requirements / ## Validation` message for worker. Runs exit_condition bash check after every worker call — never trusts worker self-report. Retries up to max_iterations with failure context.

### Worker
- **Role**: Execute a concrete task and update findings/progress.
- **Tools**: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `append_to_findings`, `explorer`
- **Behavior**: Receives a structured message with `## Task`, `## Requirements`, `## Validation` sections. Executes the task, appends to findings.md, keeps the Summary section current, and logs progress. Self-heals on errors before escalating.

## Planning Files

| File | Owner | Purpose |
|------|-------|---------|
| `todos.md` | planner | Phased task list (`- [ ]` / `- [x]`) |
| `findings.md` | explorer, worker | Knowledge base with a concise `## Summary` at top |
| `progress.md` | worker | Chronological action log |
| `todos_archive.md` | planner | Completed phases moved here to keep todos.md small |

### findings.md Summary section

The `## Summary` section (3–5 bullets) at the top of findings.md is the only part planner reads. Explorer and worker must keep it current after every update. This caps planner's context cost regardless of project size.

## Workflow

1. **Dispatcher** checks for planning files, delegates to **Planner**.
2. **Planner** (STEP 1) reads todos.md + findings.md Summary + progress.md.
3. If no plan: **Planner** calls **Explorer** → writes todos.md.
4. **Planner** finds next `- [ ]` task, optionally calls **Explorer** for context.
5. **Planner** calls **Loop** with description, exit_condition, requirements.
6. **Loop** builds structured message → calls **Worker** → verifies with bash_exec → retries if needed.
7. **Worker** implements task, updates findings.md (including Summary) and progress.md.
8. **Planner** marks task `[x]`, archives completed phases, repeats.

## Tools

Standard file operations: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `append_to_findings`.

## Usage

Invoke dispatcher with a goal:
> "Build a REST API for user authentication."

Or invoke planner directly if planning files already exist:
> "Execute the current plan."

## Design Notes

- **Context budget**: Planner reads only the findings.md `## Summary` (not the full file). Completed phases are archived. Templates are minimal.
- **Instruction following**: Planner uses a numbered FSM algorithm — one action per step, no embedded conditionals.
- **Tool call simplicity**: Worker accepts a single `description` field containing structured markdown. Loop constructs this string, eliminating multi-field tool call failures on small LLMs.
- **Verification**: Loop always verifies completion via bash_exec — never via worker self-report.
