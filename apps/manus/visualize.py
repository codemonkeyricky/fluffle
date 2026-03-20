#!/usr/bin/env python3
"""Visualize manus run exploration paths from cid-N-agentname.log files."""

import re
import glob
import json
import sys
import argparse
from datetime import datetime, timezone
from dataclasses import dataclass, field
from typing import Optional


# ── Data classes ──────────────────────────────────────────────────────────────

@dataclass
class LogInfo:
    path: str
    cid: int
    agent_type: str
    start_ts: Optional[float]
    end_ts: Optional[float]
    total_tokens: int
    iterations: int
    outcome: str  # "OK", "FAIL", "BLOCKED", "?"


@dataclass
class SpawnCall:
    ts: float
    tool: str       # "worker" or "explorer"
    description: str


@dataclass
class RunStep:
    spawn: SpawnCall
    child: Optional[LogInfo]


@dataclass
class Run:
    index: int
    orchestrator: LogInfo
    steps: list  # list[RunStep]


# ── Parsing ───────────────────────────────────────────────────────────────────

TS_RE = re.compile(r"^\[(\d+(?:\.\d+)?)\]")
TOKEN_TOTAL_RE = re.compile(r"Request total: prompt: (\d+), completion: (\d+), total: (\d+)")
TOOL_CALL_RE = re.compile(r"=== TOOL CALL: (worker|explorer|planner|loop|bash_exec|file_read|file_edit|file_write|bash|[\w_]+) ===")
TOKEN_USAGE_RE = re.compile(r"=== TOKEN USAGE ===")


def _ts(line: str) -> Optional[float]:
    m = TS_RE.match(line)
    return float(m.group(1)) if m else None


def parse_log_file(path: str) -> LogInfo:
    m = re.search(r"cid-(\d+)-([\w]+)\.log$", path)
    if not m:
        raise ValueError(f"Unexpected log filename: {path}")
    cid = int(m.group(1))
    agent_type = m.group(2)

    start_ts = None
    end_ts = None
    total_tokens = 0
    iterations = 0
    outcome = "?"
    final_content_lines = []
    in_model_response = False
    collecting_content = False

    with open(path, encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    for line in lines:
        ts = _ts(line)
        if ts is not None:
            if start_ts is None:
                start_ts = ts
            end_ts = ts

        if TOKEN_USAGE_RE.search(line):
            iterations += 1
            in_model_response = False
            collecting_content = False

        m2 = TOKEN_TOTAL_RE.search(line)
        if m2:
            total_tokens = int(m2.group(3))

        bare = line.split("] ", 1)[-1] if "] " in line else line
        if "=== MODEL RESPONSE ===" in bare:
            in_model_response = True
            collecting_content = False
            final_content_lines = []
            continue

        if in_model_response and bare.strip().startswith("Content:"):
            collecting_content = True
            content_after = bare.split("Content:", 1)[1].strip()
            final_content_lines = [content_after] if content_after else []
            continue

        if collecting_content:
            if "=== TOOL CALL:" in bare or "Tool calls:" in bare or "=== TOKEN USAGE ===" in bare:
                collecting_content = False
                in_model_response = False
            else:
                final_content_lines.append(bare.rstrip())

    # Determine outcome from final content
    combined = "\n".join(final_content_lines)
    outcome = _detect_outcome(combined)

    return LogInfo(
        path=path,
        cid=cid,
        agent_type=agent_type,
        start_ts=start_ts,
        end_ts=end_ts,
        total_tokens=total_tokens,
        iterations=iterations,
        outcome=outcome,
    )


def _detect_outcome(text: str) -> str:
    if not text.strip():
        return "?"
    upper = text.upper()
    if "BLOCKED:" in upper:
        return "BLOCKED"
    if "FAILED:" in upper or "FAIL:" in upper:
        return "FAIL"
    if "SUCCESS:" in upper or "✅" in text or "## TASK COMPLETE" in upper:
        return "OK"
    # Heuristic: if no tool calls remain and there's substantive content, likely OK
    return "OK"


def parse_spawn_calls(path: str) -> list:
    """Return ordered list of SpawnCall from an orchestrator log."""
    calls = []
    with open(path, encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    i = 0
    while i < len(lines):
        line = lines[i]
        ts = _ts(line)
        m = TOOL_CALL_RE.search(line)
        if m and m.group(1) in ("worker", "explorer"):
            tool = m.group(1)
            spawn_ts = ts if ts is not None else 0.0
            # Collect the Arguments JSON block
            json_lines = []
            i += 1
            if i < len(lines) and "Arguments:" in lines[i]:
                # Start collecting JSON
                args_rest = lines[i].split("Arguments:", 1)[1].strip()
                json_lines.append(args_rest)
                i += 1
                brace_depth = args_rest.count("{") - args_rest.count("}")
                while i < len(lines) and brace_depth > 0:
                    json_lines.append(lines[i].rstrip("\n"))
                    brace_depth += lines[i].count("{") - lines[i].count("}")
                    i += 1
                raw_json = "\n".join(json_lines)
                # Strip leading timestamp if present
                raw_json = re.sub(r"^\[\d+(?:\.\d+)?\] ?", "", raw_json, flags=re.MULTILINE)
                try:
                    args = json.loads(raw_json)
                    desc = args.get("description", "")
                    # Find first meaningful line (skip headings like "## Task", "## Requirements")
                    first_line = ""
                    for line in desc.splitlines():
                        stripped = line.strip()
                        if stripped and not re.match(r"^#+\s*(task|requirements|summary|context|validation|note|goal|objective)s?\s*$", stripped, re.I):
                            first_line = re.sub(r"^#+\s*", "", stripped)
                            break
                    if not first_line:
                        first_line = next((re.sub(r"^#+\s*", "", l.strip()) for l in desc.splitlines() if l.strip()), desc)
                    calls.append(SpawnCall(ts=spawn_ts, tool=tool, description=first_line))
                except (json.JSONDecodeError, ValueError):
                    # Fall back: grab first description-looking line
                    desc_m = re.search(r'"description"\s*:\s*"([^"\\]|\\.)*', raw_json)
                    desc = desc_m.group(0).split(":", 1)[1].strip().strip('"') if desc_m else ""
                    calls.append(SpawnCall(ts=spawn_ts, tool=tool, description=desc[:80]))
            continue
        i += 1

    return calls


MAX_SPAWN_LAG = 120  # seconds: max allowed gap between spawn ts and child start_ts


def group_into_runs(all_logs: list) -> list:
    """Group logs into runs by globally matching spawn calls to child start times."""
    ORCHESTRATOR_TYPES = {"loop", "planner"}
    orchestrators = sorted(
        [l for l in all_logs if l.agent_type in ORCHESTRATOR_TYPES],
        key=lambda l: (l.start_ts or 0, l.cid),
    )
    children = [l for l in all_logs if l.agent_type not in ORCHESTRATOR_TYPES]

    # Collect all spawns across all orchestrators
    orch_spawns = []  # (orchestrator, spawn_call)
    orch_spawn_lists = {}  # orchestrator.path -> [SpawnCall]
    for orch in orchestrators:
        calls = parse_spawn_calls(orch.path)
        orch_spawn_lists[orch.path] = calls
        for sc in calls:
            orch_spawns.append((orch, sc))

    # Sort all spawns by timestamp
    orch_spawns.sort(key=lambda x: x[1].ts)

    # Greedy global matching: for each spawn (in ts order), claim the best available child
    child_assignment = {}  # child.path -> (orchestrator, spawn_call)
    available = list(children)

    for orch, spawn in orch_spawns:
        window = [
            c for c in available
            if (c.start_ts or 0) >= spawn.ts - 2
            and (c.start_ts or 0) <= spawn.ts + MAX_SPAWN_LAG
        ]
        # Prefer matching agent type
        typed = [c for c in window if c.agent_type == spawn.tool]
        candidates = typed or window
        if candidates:
            best = min(candidates, key=lambda c: abs((c.start_ts or 0) - spawn.ts))
            child_assignment[best.path] = (orch, spawn)
            available.remove(best)

    # Build run steps from assignments
    # Map: orchestrator.path -> {spawn_index -> child}
    orch_step_map = {o.path: {} for o in orchestrators}
    for child_path, (orch, spawn) in child_assignment.items():
        spawn_list = orch_spawn_lists[orch.path]
        # Find the spawn index (by identity/timestamp)
        for i, sc in enumerate(spawn_list):
            if sc is spawn:
                orch_step_map[orch.path][i] = next(c for c in children if c.path == child_path)
                break

    runs = []
    for idx, orch in enumerate(orchestrators):
        spawn_list = orch_spawn_lists[orch.path]
        step_map = orch_step_map[orch.path]
        steps = [RunStep(spawn=sc, child=step_map.get(i)) for i, sc in enumerate(spawn_list)]
        runs.append(Run(index=idx + 1, orchestrator=orch, steps=steps))

    return runs


# ── Formatting helpers ─────────────────────────────────────────────────────────

def _fmt_ts(ts: Optional[float]) -> str:
    if ts is None:
        return "?"
    return datetime.fromtimestamp(ts, tz=timezone.utc).strftime("%Y-%m-%d %H:%M")


def _fmt_dur(start: Optional[float], end: Optional[float]) -> str:
    if start is None or end is None:
        return "?"
    secs = int(end - start)
    if secs < 60:
        return f"{secs}s"
    m, s = divmod(secs, 60)
    if m < 60:
        return f"{m}m {s}s"
    h, m = divmod(m, 60)
    return f"{h}h {m}m"


def _fmt_tok(n: int) -> str:
    if n >= 1_000_000:
        return f"{n/1_000_000:.1f}M"
    if n >= 1_000:
        return f"{n/1_000:.1f}k"
    return str(n)


def _short_desc(desc: str, width: int = 44) -> str:
    # Remove markdown headings and leading ##
    desc = re.sub(r"^#+\s*", "", desc).strip()
    if len(desc) <= width:
        return desc
    return desc[:width - 1] + "…"


def _outcome_symbol(outcome: str) -> str:
    return {"OK": "OK", "FAIL": "FAIL", "BLOCKED": "BLOCKED", "?": "?"}.get(outcome, outcome)


# ── Format functions ───────────────────────────────────────────────────────────

def format_run_text(run: Run) -> str:
    o = run.orchestrator
    total_tok = sum(
        (s.child.total_tokens if s.child else 0) for s in run.steps
    ) + o.total_tokens
    failures = sum(1 for s in run.steps if s.child and s.child.outcome == "FAIL")
    decomposed = sum(1 for s in run.steps if "[decomposed]" in s.spawn.description.lower())

    header = (
        f"Run #{run.index}  [CID {o.cid}]  "
        f"started {_fmt_ts(o.start_ts)}  "
        f"duration {_fmt_dur(o.start_ts, o.end_ts)}\n"
    )

    lines = [header]

    # Column widths
    W = {"step": 4, "type": 8, "cid": 5, "desc": 44, "outcome": 7, "time": 7, "tokens": 7, "iters": 5}
    sep = "  " + "─" * (sum(W.values()) + len(W) * 2)

    def row(*vals):
        cols = list(W.values())
        return "  " + "  ".join(str(v).ljust(c) for v, c in zip(vals, cols))

    lines.append(row("Step", "Type", "CID", "Description", "Outcome", "Time", "Tokens", "Iters"))
    lines.append(sep)

    for i, step in enumerate(run.steps, 1):
        c = step.child
        cid_s = str(c.cid) if c else "?"
        desc = _short_desc(step.spawn.description)
        outcome = c.outcome if c else "?"
        time_s = _fmt_dur(c.start_ts, c.end_ts) if c else "?"
        tok_s = _fmt_tok(c.total_tokens) if c else "?"
        iters_s = str(c.iterations) if c else "?"
        lines.append(row(i, step.spawn.tool, cid_s, desc, outcome, time_s, tok_s, iters_s))

    totals = (
        f"\n  Totals: {_fmt_dur(o.start_ts, o.end_ts)}  |  "
        f"{len(run.steps)} spawns  |  "
        f"{failures} failure{'s' if failures != 1 else ''}  |  "
        f"{decomposed} decomposition{'s' if decomposed != 1 else ''}  |  "
        f"{_fmt_tok(total_tok)} tokens"
    )
    lines.append(totals)
    return "\n".join(lines)


def format_run_mermaid(run: Run) -> str:
    o = run.orchestrator
    oid = f"O{o.cid}"
    lines = ["```mermaid", "graph LR"]
    lines.append(f'  {oid}["{o.agent_type} CID:{o.cid}"]')

    for i, step in enumerate(run.steps, 1):
        c = step.child
        cid = c.cid if c else f"x{i}"
        prefix = "E" if step.spawn.tool == "explorer" else "W"
        nid = f"{prefix}{cid}"
        desc = _short_desc(step.spawn.description, 24).replace('"', "'")
        label = f"{step.spawn.tool} CID:{cid}\\n{desc}"
        edge_label = str(i)
        if c and c.outcome == "FAIL":
            edge_label += " FAIL"
        lines.append(f'  {oid} -->|"{edge_label}"| {nid}["{label}"]')
        if c and c.outcome == "FAIL":
            lines.append(f"  style {nid} fill:#ff9999")

    lines.append(f"  style {oid} fill:#aaddff")
    lines.append("```")
    return "\n".join(lines)


def format_compare_text(run_a: Run, run_b: Run) -> str:
    def run_header(r: Run) -> str:
        total = sum((s.child.total_tokens if s.child else 0) for s in r.steps)
        return f"Run {r.index} ({_fmt_dur(r.orchestrator.start_ts, r.orchestrator.end_ts)}, {_fmt_tok(total)} tok)"

    h_a = run_header(run_a)
    h_b = run_header(run_b)
    col_w = 42
    header = f"  {'  ' + h_a:<{col_w}}  {h_b}"
    lines = [header]

    max_steps = max(len(run_a.steps), len(run_b.steps))

    def step_col(run: Run, i: int) -> str:
        if i >= len(run.steps):
            return " " * col_w
        s = run.steps[i]
        c = s.child
        outcome = c.outcome if c else "?"
        desc = _short_desc(s.spawn.description, 22)
        return f"{s.spawn.tool:<8}  {desc:<22}  {outcome:<7}"

    for i in range(max_steps):
        col_a = step_col(run_a, i)
        col_b = step_col(run_b, i)
        lines.append(f"  Step {i+1:<2}  {col_a}  {col_b}")

    return "\n".join(lines)


# ── CLI ───────────────────────────────────────────────────────────────────────

def load_runs(directory: str) -> list:
    pattern = f"{directory}/cid-*.log"
    paths = glob.glob(pattern)
    if not paths:
        print(f"No cid-*.log files found in {directory}", file=sys.stderr)
        sys.exit(1)

    logs = []
    for path in paths:
        try:
            logs.append(parse_log_file(path))
        except Exception as e:
            print(f"Warning: skipping {path}: {e}", file=sys.stderr)

    runs = group_into_runs(logs)
    if not runs:
        print("No orchestrator logs (loop/planner) found.", file=sys.stderr)
        sys.exit(1)
    return runs


def main():
    parser = argparse.ArgumentParser(description="Visualize manus run exploration paths.")
    parser.add_argument("dir", nargs="?", default=".", help="Directory with cid-*.log files")
    parser.add_argument("--run", type=int, metavar="N", help="Show only run N (1-indexed)")
    parser.add_argument("--mermaid", action="store_true", help="Output Mermaid diagrams")
    parser.add_argument("--compare", type=int, nargs=2, metavar=("N", "M"), help="Compare runs N and M")
    args = parser.parse_args()

    runs = load_runs(args.dir)

    def get_run(n: int) -> Run:
        if n < 1 or n > len(runs):
            print(f"Run {n} not found (have {len(runs)} runs)", file=sys.stderr)
            sys.exit(1)
        return runs[n - 1]

    if args.compare:
        n, m = args.compare
        print(format_compare_text(get_run(n), get_run(m)))
        return

    target_runs = [get_run(args.run)] if args.run else runs
    for run in target_runs:
        if args.mermaid:
            print(format_run_mermaid(run))
        else:
            print(format_run_text(run))
        print()


if __name__ == "__main__":
    main()
