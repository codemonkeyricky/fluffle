# Manus: Planning with Files Agents

This module implements the "planning with files" methodology using specialized agents that work with three markdown files: `todos.md`, `findings.md`, and `progress.md`.

## Agents

### Dispatcher
- **Role**: Decide whether to create a new plan or execute an existing plan.
- **Tools**: `planner`
- **Behavior**: Checks for existing planning files (todos.md, findings.md, progress.md) and delegates to the planner agent.

### Planner
- **Role**: Create, manage, and execute plans by delegating tasks to workers.
- **Tools**: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `worker`
- **Behavior**: Reads planning files to determine current phase and pending tasks, delegates tasks to workers with validation criteria, updates todos.md progress, monitors findings.md and progress.md, and removes stale findings.

### Worker
- **Role**: Execute specific tasks delegated by the planner.
- **Tools**: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`
- **Behavior**: Performs concrete implementation work, updates findings.md with discoveries, and logs detailed actions in progress.md. Does not modify todos.md.

## Tools

All standard file operations tools are included: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`.

## Workflow

1. **Dispatcher** checks for existing planning files (todos.md, findings.md, progress.md).
2. If planning files exist, delegates to **Planner** to continue execution. If not, delegates to **Planner** to create new plan.
3. **Planner** reads planning files to determine current phase and pending tasks.
4. **Planner** delegates each task to **Worker** with clear description, requirements, and validation criteria.
5. **Worker** executes the task, updating `findings.md` and `progress.md`.
6. **Planner** updates `todos.md` with progress and removes stale findings.
7. Process repeats until all phases are complete.

## Templates

The `templates/` directory contains starter markdown files:
- `todos.md`
- `findings.md`
- `progress.md`

These follow the structure defined in the [planning-with-files](https://github.com/OthmanAdi/planning-with-files) methodology.

## Usage

Assume a plan already exists in the project root. The planner can be invoked directly to manage execution.

Example prompt: "Execute the current plan using the planner."

## Notes

- Agents follow the 2-Action Rule: after every 2 view/browser/search operations, they update `findings.md`.
- Errors are logged in both `todos.md` and `progress.md`.
- Stale findings should be periodically removed by the planner.