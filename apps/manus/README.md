# Manus: Planning with Files Agents

This module implements the "planning with files" methodology using specialized agents that work with three markdown files: `todos.md`, `findings.md`, and `progress.md`.

## Agents

### Planner
- **Role**: Orchestrate plan creation and execution via a strict FSM algorithm.
- **Tools**: `file_read`, `file_write`, `file_edit`, `explorer`, `worker`
- **Behavior**: Reads todos.md and the findings.md Summary each iteration. If no plan exists (or TODO: items appear in findings Summary), calls explorer then writes todos.md. For each pending task, optionally calls explorer for context, then delegates to worker. Marks tasks done, archives completed phases to `todos_archive.md`.

### Explorer
- **Role**: Codebase research and findings collection.
- **Tools**: `file_read`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `append_to_findings`
- **Behavior**: Reads findings.md Summary before exploring. Appends new findings, then rewrites the Summary section with the 3–5 most critical facts.

### Worker
- **Role**: Execute a concrete task, verify completion, and update findings/progress.
- **Tools**: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `append_to_findings`, `explorer`
- **Behavior**: Receives description, exit_condition, and requirements. Executes the task, verifies completion by running exit_condition via bash_exec, and retries with a different approach if it fails. Appends to findings.md (including Summary) and logs progress. Never reports success until exit_condition passes.

## Planning Files

| File | Owner | Purpose |
|------|-------|---------|
| `todos.md` | planner | Phased task list (`- [ ]` / `- [x]`) |
| `findings.md` | explorer, worker | Knowledge base with a concise `## Summary` at top |
| `progress.md` | worker | Chronological action log |
| `todos_archive.md` | planner | Completed phases moved here to keep todos.md small |

### findings.md Summary section

The `## Summary` section (3–5 bullets) at the top of findings.md is the only part planner reads. Explorer and worker must keep it current after every update. This caps planner's context cost regardless of project size.

Worker uses `TODO: <task>` lines in the Summary to signal discovered work back to planner. Planner detects these on its next STEP 1 read and folds them into todos.md.

## Workflow

1. **Planner** (STEP 1) reads todos.md + findings.md Summary + progress.md.
2. If no plan (or TODO: items in Summary): **Planner** calls **Explorer** → writes/updates todos.md.
3. **Planner** finds next `- [ ]` task, optionally calls **Explorer** for context.
4. **Planner** calls **Worker** with description, exit_condition, requirements.
5. **Worker** implements task → verifies via bash_exec on exit_condition → retries if needed → updates findings.md and progress.md.
6. **Planner** marks task `[x]`, archives completed phases, repeats.

## Tools

Standard file operations: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `append_to_findings`.

## Usage

Invoke planner with a goal:
> "Build a REST API for user authentication."

Or invoke planner directly if planning files already exist:
> "Execute the current plan."

## Design Notes

- **Context budget**: Planner reads only the findings.md `## Summary` (not the full file). Completed phases are archived. Templates are minimal.
- **Instruction following**: Planner uses a numbered FSM algorithm — one action per step, no embedded conditionals.
- **Verification**: Worker always verifies completion via bash_exec — never self-reports success without it.
- **Discovered work**: Worker writes `TODO: <task>` to findings Summary; planner picks it up on the next iteration without worker ever touching todos.md.
- **Fresh context on retry**: Worker re-reads findings.md Summary at the start of each retry, so it always has current state rather than a stale snapshot.
