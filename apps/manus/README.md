# Manus: Planning with Files Agents

This module implements the "planning with files" methodology using specialized agents that work with three markdown files: `task_plan.md`, `findings.md`, and `progress.md`.

## Agents

### Dispatcher
- **Role**: Decide whether to create a new plan or execute an existing plan.
- **Tools**: `planner`, `orchestrator`
- **Behavior**: Checks for existing planning files and delegates to the appropriate agent.

### Planner
- **Role**: Create or update planning files from high-level goals.
- **Tools**: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `append_to_findings`
- **Behavior**: Uses templates to initialize or modify the three planning files, ensuring they are consistent and actionable.

### Orchestrator
- **Role**: Manage execution of an existing plan by delegating tasks to workers.
- **Tools**: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `worker`
- **Behavior**: Reads planning files to determine current phase and pending tasks, delegates tasks to workers, updates `task_plan.md` progress, and monitors `findings.md` and `progress.md`.

### Worker
- **Role**: Execute specific tasks delegated by the orchestrator.
- **Tools**: `file_read`, `file_write`, `file_edit`, `file_list`, `bash_exec`, `git_status`, `git_diff`, `append_to_findings`
- **Behavior**: Performs concrete implementation work, updates `findings.md` with discoveries, and logs detailed actions in `progress.md`. Does not modify `task_plan.md`.

## Tools

All standard file operations tools are included. Additionally:

- **`append_to_findings`**: Append a bullet point to a specific section in `findings.md`. Sections: Requirements, Research Findings, Technical Decisions, Issues Encountered, Resources, Visual/Browser Findings.

## Workflow

1. **Dispatcher** checks for existing planning files.
2. If no plan exists, **Planner** creates the three files using templates.
3. **Orchestrator** reads the plan, identifies current phase and pending tasks.
4. **Orchestrator** delegates each task to **Worker** with clear description and validation criteria.
5. **Worker** executes the task, updating `findings.md` and `progress.md`.
6. **Orchestrator** updates `task_plan.md` with progress and removes stale findings.
7. Process repeats until all phases are complete.

## Templates

The `templates/` directory contains starter markdown files:
- `task_plan.md`
- `findings.md`
- `progress.md`

These follow the structure defined in the [planning-with-files](https://github.com/OthmanAdi/planning-with-files) methodology.

## Usage

Assume a plan already exists in the project root. The orchestrator can be invoked directly to manage execution.

Example prompt: "Execute the current plan using the orchestrator."

## Notes

- Agents follow the 2-Action Rule: after every 2 view/browser/search operations, they update `findings.md`.
- Errors are logged in both `task_plan.md` and `progress.md`.
- Stale findings should be periodically removed by the orchestrator.