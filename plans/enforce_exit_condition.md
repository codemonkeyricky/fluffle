# Plan: Deterministic exit_condition Enforcement

## Context

`exit_condition` enforcement is currently behavioral — the worker's system_prompt instructs the LLM to run a bash command after each file change and only emit `SUCCESS:` if it exits 0. This is unreliable. The goal is to have Rust code deterministically run the exit_condition after a sub-agent finishes, feed the error back if it fails, and retry up to 3 times total.

The child agent thread stays alive between requests (the `run()` loop awaits the next `UiToAgent::Request` after sending its `Response`), so the retry mechanism can simply re-send corrective messages to the running child thread.

## Two Execution Paths

- **Inline path** (`execute_inline` in `loaders/agents.rs`, `spawn_child_inline` in `headless_backend.rs`): agent runs synchronously via `process()`. New `process_with_exit_condition()` method on `Agent` handles retries.
- **TUI path** (`spawn_child_agent` in `simple_tui.rs`): child runs in a separate tokio task, communicates via channels. exit_condition is checked in the `Response` handler before popping the stack. On failure, send corrective `UiToAgent::Request` back to the still-alive child thread instead of popping.

## Changes

### 1. `src/agent.rs` — Add helper + new method

**`pub(crate) async fn run_exit_condition(working_directory: &Path, command: &str) -> Result<(), String>`**
- Standalone function (not a method, avoids borrow conflicts with `&mut self`)
- `tokio::process::Command::new("bash").arg("-c").arg(command).current_dir(working_directory).output().await`
- Returns `Ok(())` on exit 0
- Returns `Err(feedback)` on non-zero, including command, exit code, stdout, stderr, and "Please fix the issue and try again."

**`pub async fn process_with_exit_condition(&mut self, user_message: &str, exit_condition: Option<&str>) -> Result<String>`**
- If `exit_condition` is None: delegate to `self.process(user_message)` and return
- Clone `self.context.working_directory` before the loop (avoids borrow conflict)
- Loop `attempt in 0..3`:
  - `attempt == 0`: call `self.process(user_message)`
  - `attempt > 0`: call `self.process(&last_error_feedback)` (history persists, agent has full context)
  - Run `run_exit_condition(&workdir, condition).await`
  - `Ok(())` → return the agent response
  - `Err(feedback)` on last attempt → break
  - `Err(feedback)` otherwise → store as `last_error_feedback`, continue
- After loop: return `Err(Error::Agent("exit_condition `{command}` not satisfied after 3 attempts. Last failure:\n{last_error}"))`

### 2. `src/messaging.rs` — Add field to SpawnChild

```rust
SpawnChild {
    name: String,
    description: String,
    system_prompt: Option<String>,
    exit_condition: Option<String>,   // NEW
    result_tx: oneshot::Sender<ToolResult>,
}
```

### 3. `src/loaders/agents.rs` — Extract exit_condition, thread through both paths

In `execute()`:
- Extract: `let exit_condition = params.get("exit_condition").and_then(|v| v.as_str()).map(|s| s.to_string());`
- Inline path: `self.execute_inline(ctx, description_json, exit_condition).await`
- TUI path: add `exit_condition: exit_condition.clone()` to `AgentToUi::SpawnChild { ... }`

In `execute_inline()`:
- Add `exit_condition: Option<String>` parameter
- Replace `agent.process(&description).await` with `agent.process_with_exit_condition(&description, exit_condition.as_deref()).await`

### 4. `src/loaders/task.rs` — Add `exit_condition: None` to SpawnChild construction

No behavior change; required by the new enum field.

### 5. `src/ui/agent_stack.rs` — Carry retry state on AgentHandle

Add to `AgentHandle`:
```rust
pub exit_condition: Option<String>,
pub exit_retry_count: u32,
```

Update `push_with_result_tx()` to accept `exit_condition: Option<String>` and store it.

Add to `AgentStack`:
```rust
pub fn current_exit_condition(&self) -> Option<&str>
pub fn current_exit_retry_count(&self) -> u32
pub fn increment_exit_retry(&mut self)
```

### 6. `src/ui/simple_tui.rs` — TUI retry logic

In `handle_incoming_message` for `SpawnChild`:
- Extract `exit_condition` from the message
- Pass it to `spawn_child_agent(name, description, system_prompt, exit_condition, result_tx)`

In `spawn_child_agent()`:
- Accept `exit_condition: Option<String>`
- Pass to `push_with_result_tx(name, tx, rx, result_tx, cid, exit_condition)`

In the `Response` handler for child agents (`is_child = self.stack.len() > 1`), before the existing pop logic:
```
if let Some(ec_cmd) = self.stack.current_exit_condition() {
    let workdir = self.workdir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    match crate::agent::run_exit_condition(&workdir, ec_cmd).await {
        Ok(()) => { /* fall through to pop */ }
        Err(feedback) if self.stack.current_exit_retry_count() < 2 => {
            self.stack.increment_exit_retry();
            if let Some(child_tx) = self.stack.current_tx() {
                let _ = child_tx.send(UiToAgent::Request(feedback)).await;
            }
            return Ok(()); // Don't pop; wait for child's next Response
        }
        Err(feedback) => {
            // Exhausted retries — fall through to pop with error result
            // override `result` to ToolResult::error(...)
        }
    }
}
```

### 7. `src/ui/headless_backend.rs` — Pass exit_condition to inline spawn

- Extract `exit_condition` from `SpawnChild` destructure
- Pass to `spawn_child_inline(name, description, system_prompt, exit_condition)`
- In `spawn_child_inline`: add `exit_condition: Option<String>` param; replace `agent.process(&description)` with `agent.process_with_exit_condition(&description, exit_condition.as_deref())`

## Files to Modify

| File | Change |
|---|---|
| `src/agent.rs` | Add `run_exit_condition` fn + `process_with_exit_condition` method |
| `src/messaging.rs` | Add `exit_condition: Option<String>` to `SpawnChild` |
| `src/loaders/agents.rs` | Extract `exit_condition`; update `execute_inline`; thread through SpawnChild |
| `src/loaders/task.rs` | Add `exit_condition: None` to SpawnChild construction |
| `src/ui/agent_stack.rs` | Add exit_condition + retry fields to `AgentHandle`; update `push_with_result_tx`; add accessors |
| `src/ui/simple_tui.rs` | Pass exit_condition through spawn; check in Response handler before pop |
| `src/ui/headless_backend.rs` | Thread exit_condition through inline spawn |

`src/ui/app.rs` uses `SpawnChild { .. }` (wildcard pattern), so no change needed there.

## Verification

1. `cargo build` — confirms no compile errors across all SpawnChild match sites
2. Run a manus worker task with a known-failing initial attempt to confirm:
   - exit_condition bash command runs after each agent response
   - Error output is fed back as a new user message
   - Worker retries and succeeds on correction
   - Retry count stops at 3
