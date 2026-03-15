//! Structured debug logging for agent and tool calls.
//!
//! Writes one JSONL entry per event to `debug.log` in the working directory.
//! Each entry identifies the agent profile and tool by name so log sections
//! can be pasted back to tune the corresponding `.json` definition files.
//!
//! # Log format
//!
//! Every entry includes:
//! - `ts`    — Unix timestamp in milliseconds
//! - `agent` — agent profile name (matches `apps/<app>/agents/<name>.json`)
//! - `cid`   — conversation/context ID linking parent and child agents (optional)
//! - `event` — one of: `agent_start`, `agent_end`, `agent_error`,
//!              `tool_call`, `tool_result`, `agent_spawn`, `agent_complete`, `thinking`
//!
//! Event-specific fields:
//!
//! ```json
//! {"event":"agent_start",    "profile":"planner", "model":"claude-opus-4-6", "temperature":0.7, "max_iterations":30, "tools":["file_read","worker"], "message":"..."}
//! {"event":"agent_end",      "profile":"planner", "response":"...", "iterations":5, "prompt_tokens":1200, "completion_tokens":300}
//! {"event":"agent_error",    "profile":"planner", "error":"ToolIterationLimit(31)", "iterations":31}
//! {"event":"tool_call",      "tool":"bash_exec",  "iter":3, "params":{"command":"cargo build"}}
//! {"event":"tool_result",    "tool":"bash_exec",  "iter":3, "success":true, "output":"...", "duration_ms":12}
//! {"event":"agent_spawn",    "profile":"worker",  "description":"..."}
//! {"event":"agent_complete", "profile":"worker",  "success":true, "output":"...", "duration_ms":4201, "prompt_tokens":800, "completion_tokens":150}
//! {"event":"thinking",       "content":"..."}
//! ```

use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;

const MAX_STR_LEN: usize = 3000;

/// Truncate a string to `MAX_STR_LEN` chars, appending `…` if trimmed.
fn trunc(s: &str) -> String {
    if s.len() <= MAX_STR_LEN {
        s.to_string()
    } else {
        let mut out = s.chars().take(MAX_STR_LEN).collect::<String>();
        out.push('…');
        out
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Append a JSONL entry to `<dir>/debug.log`.
/// Fields `ts`, `agent`, and `cid` are injected automatically.
fn append(dir: &Path, agent: &str, cid: Option<u64>, mut entry: Value) {
    let obj = match entry.as_object_mut() {
        Some(o) => o,
        None => return,
    };
    obj.insert("ts".into(), now_ms().into());
    obj.insert("agent".into(), agent.into());
    if let Some(c) = cid {
        obj.insert("cid".into(), c.into());
    }

    let path = dir.join("debug.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        if let Ok(line) = serde_json::to_string(&entry) {
            let _ = writeln!(file, "{}", line);
        }
    }
}

// ---------------------------------------------------------------------------
// Public logging functions — one per event type
// ---------------------------------------------------------------------------

/// Agent began processing a user message.
pub fn agent_start(
    dir: &Path,
    agent: &str,
    cid: Option<u64>,
    tools: &[&str],
    message: &str,
    model: &str,
    temperature: f32,
    max_iterations: u32,
) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "agent_start",
            "profile": agent,
            "model": model,
            "temperature": temperature,
            "max_iterations": max_iterations,
            "tools": tools,
            "message": trunc(message),
        }),
    );
}

/// Agent finished successfully, returning a final response.
pub fn agent_end(
    dir: &Path,
    agent: &str,
    cid: Option<u64>,
    response: &str,
    iterations: u32,
    prompt_tokens: u64,
    completion_tokens: u64,
) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "agent_end",
            "profile": agent,
            "response": trunc(response),
            "iterations": iterations,
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
        }),
    );
}

/// Agent exited with an error (e.g. iteration limit exceeded).
pub fn agent_error(
    dir: &Path,
    agent: &str,
    cid: Option<u64>,
    error: &str,
    iterations: u32,
) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "agent_error",
            "profile": agent,
            "error": error,
            "iterations": iterations,
        }),
    );
}

/// LLM emitted a tool call.
pub fn tool_call(
    dir: &Path,
    agent: &str,
    cid: Option<u64>,
    tool: &str,
    iter: u32,
    params: &Value,
) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "tool_call",
            "tool": tool,
            "iter": iter,
            "params": params,
        }),
    );
}

/// Tool execution finished.
pub fn tool_result(
    dir: &Path,
    agent: &str,
    cid: Option<u64>,
    tool: &str,
    iter: u32,
    success: bool,
    output: &str,
    duration_ms: u128,
) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "tool_result",
            "tool": tool,
            "iter": iter,
            "success": success,
            "output": trunc(output),
            "duration_ms": duration_ms,
        }),
    );
}

/// A child agent was spawned.
pub fn agent_spawn(
    dir: &Path,
    agent: &str,
    cid: Option<u64>,
    profile: &str,
    description: &str,
) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "agent_spawn",
            "profile": profile,
            "description": trunc(description),
        }),
    );
}

/// A child agent finished.
pub fn agent_complete(
    dir: &Path,
    agent: &str,
    cid: Option<u64>,
    profile: &str,
    success: bool,
    output: &str,
    duration_ms: u128,
    prompt_tokens: u64,
    completion_tokens: u64,
) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "agent_complete",
            "profile": profile,
            "success": success,
            "output": trunc(output),
            "duration_ms": duration_ms,
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
        }),
    );
}

/// Agent emitted reasoning / thinking text.
pub fn thinking(dir: &Path, agent: &str, cid: Option<u64>, content: &str) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "thinking",
            "content": trunc(content),
        }),
    );
}
