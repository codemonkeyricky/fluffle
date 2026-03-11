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
//! - `event` — one of: `agent_start`, `tool_call`, `tool_result`,
//!              `agent_spawn`, `agent_complete`, `thinking`
//!
//! Event-specific fields:
//!
//! ```json
//! {"event":"agent_start", "profile":"planner", "tools":["file_read","loop"], "message":"..."}
//! {"event":"tool_call",   "tool":"bash_exec",  "params":{"command":"ls -la"}}
//! {"event":"tool_result", "tool":"bash_exec",  "success":true,  "output":"...", "duration_ms":12}
//! {"event":"agent_spawn", "profile":"worker",  "description":"..."}
//! {"event":"agent_complete","profile":"worker","success":true,  "output":"...", "duration_ms":4201}
//! {"event":"thinking",    "content":"..."}
//! ```

use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;

const MAX_STR_LEN: usize = 1000;

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
pub fn agent_start(dir: &Path, agent: &str, cid: Option<u64>, tools: &[&str], message: &str) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "agent_start",
            "profile": agent,
            "tools": tools,
            "message": trunc(message),
        }),
    );
}

/// LLM emitted a tool call.
pub fn tool_call(dir: &Path, agent: &str, cid: Option<u64>, tool: &str, params: &Value) {
    append(
        dir,
        agent,
        cid,
        json!({
            "event": "tool_call",
            "tool": tool,
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
