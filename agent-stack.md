# Agent Stack Implementation Plan

## Overview
Implement support for agent stacking, where agents can invoke other agents (e.g., generalist agent can invoke explore agent). Since there's only a singleton UI, agents are implemented as a stack with the agent on top receiving UI events.

Example: Nanocode starts with generalist agent, which can call explore agent. When running explore agent, agent stack is size two with explore agent on the top of stack receiving UI focus.

## Core Requirements
1. **Stack-based UI focus**: Only the topmost agent receives UI events
2. **Child agent spawning**: Parent agents can spawn child agents via task/explore tools
3. **Context isolation**: Child agents start with fresh context (default working directory, no parent permissions)
4. **UI status display**: Status line shows current agent name (profile name)
5. **No manual switching**: Users cannot manually switch between agents (for now)

## Architecture Design

### AgentStack Manager
Central coordinator that maintains a stack of `AgentHandle` instances, each representing an active agent with its communication channels and metadata.

```rust
struct AgentHandle {
    name: String,           // Agent profile name (e.g., "generalist", "explorer")
    ui_to_agent_tx: mpsc::Sender<UiToAgent>,
    agent_to_ui_rx: mpsc::Receiver<AgentToUi>,
    child_result_tx: Option<oneshot::Sender<ToolResult>>, // For returning child results to parent
}

struct AgentStack {
    stack: Vec<AgentHandle>,
}
```

### Message Flow
1. UI events (keyboard input) → `AgentStack.current_tx()` → Top agent
2. Agent messages → `AgentStack.current_rx()` → UI
3. Child spawning: Parent sends `AgentToUi::SpawnChild` → UI/Stack pushes new agent
4. Child completion: Child sends final response → Stack pops child → sends `UiToAgent::ChildResult` to parent

## File Changes Required

### 1. New File: `src/ui/agent_stack.rs`
Contains `AgentStack` and `AgentHandle` implementations with methods:
- `push()`: Add new agent to stack, UI switches to its channel
- `pop()`: Remove top agent, restore previous agent's channel
- `current_tx()` / `current_rx()`: Get channels for current top agent
- `spawn_child()`: Helper for creating and pushing child agents

### 2. Update: `src/messaging.rs`
Add new message types:
```rust
// From agent to UI
AgentToUi::SpawnChild {
    name: String,           // Agent profile name
    description: String,    // Task description
    system_prompt: Option<String>, // Optional custom system prompt
}

// From UI to agent
UiToAgent::ChildResult {
    success: bool,
    output: String,
    error: Option<String>,
}
```

### 3. Update: `src/ui/mod.rs`
- Replace `UiChannels` with `AgentStack` in UI trait
- Update `create_ui` to initialize stack with base agent

### 4. Update: `src/ui/simple_tui.rs`
Major changes:
- Replace `channels: UiChannels` with `stack: AgentStack`
- Update event loop to use `stack.current_rx()` for receiving messages
- Handle `AgentToUi::SpawnChild` by creating child agent and pushing onto stack
- Handle `UiToAgent::ChildResult` by forwarding to parent (via oneshot)
- Update status line to show `stack.current_name()`

### 5. Update: `src/ui/headless_backend.rs`
Simpler approach: Maintain single agent (no stack) but handle `SpawnChild` by running child inline (blocking) and returning result.

### 6. Update: `src/agent_thread.rs`
- Modify `spawn()` to return both `(ui_to_agent_tx, agent_to_ui_rx)` pair
- Or keep as is and create channels before spawning agent

### 7. Update: `src/loaders/task.rs`
Replace current blocking execution with:
1. Send `AgentToUi::SpawnChild` with description and system_prompt
2. Await `ChildResult` via oneshot channel
3. Return tool result based on child outcome

### 8. Update: `src/loaders/agents.rs`
Similar changes as task tool, but use profile name instead of custom system prompt.

### 9. Update: `src/agent.rs`
- Ensure `Agent::new()` creates agents with fresh context (default working directory)
- No parent context propagation

## Implementation Steps

### Phase 1: Core Stack Infrastructure
1. Create `agent_stack.rs` with basic stack management
2. Update messaging types
3. Modify UI trait to use stack

### Phase 2: UI Integration
1. Update `simple_tui.rs` to use `AgentStack`
2. Implement child spawning logic in UI
3. Update status display for agent name

### Phase 3: Tool Updates
1. Update task tool to use new spawn mechanism
2. Update profile tool similarly
3. Ensure child agents start with fresh context

### Phase 4: Headless Mode
1. Update headless backend to handle `SpawnChild` (inline execution)
2. Maintain backward compatibility

### Phase 5: Testing & Validation
1. Test nested agent invocation
2. Verify UI focus switches correctly
3. Ensure no message leaks between stacked agents
4. Run existing tests

## Detailed Implementation Notes

### AgentStack Implementation
```rust
impl AgentStack {
    pub fn new(base_agent_name: String, base_tx: mpsc::Sender<UiToAgent>, base_rx: mpsc::Receiver<AgentToUi>) -> Self;
    
    pub fn push(&mut self, name: String, tx: mpsc::Sender<UiToAgent>, rx: mpsc::Receiver<AgentToUi>) -> oneshot::Receiver<ToolResult>;
    
    pub fn pop(&mut self, result: ToolResult) -> Option<oneshot::Sender<ToolResult>>;
    
    pub fn current_tx(&self) -> Option<&mpsc::Sender<UiToAgent>>;
    
    pub fn current_rx(&mut self) -> Option<&mut mpsc::Receiver<AgentToUi>>;
    
    pub fn current_name(&self) -> Option<&str>;
}
```

### Child Spawning Flow
1. Parent agent tool calls execute → sends `AgentToUi::SpawnChild`
2. UI receives `SpawnChild` → creates child agent via `agent_thread::spawn`
3. UI pushes child onto stack → UI now routes events to child
4. Child processes task → sends final `Response` or `Error`
5. UI detects child completion → pops stack with result
6. Result sent to parent via oneshot channel
7. Parent receives `ChildResult` → continues tool execution

### Context Management
- Child agents use `Agent::new(config)` which creates fresh context
- Working directory: current directory at spawn time (not parent's directory)
- Permissions: empty list (no inherited permissions)
- Agent-to-UI channel: set by UI when pushing to stack
- **Context IDs**: Root agent has CID (context ID) of 1 and is the only long-running agent. Any agent spawned by the root agent can call ask-user tool to collect user input, but terminates when its done and is popped off the stack.

## Testing Strategy

### Unit Tests
1. `AgentStack` push/pop operations
2. Message type serialization/deserialization
3. Tool parameter validation

### Integration Tests
1. Generalist → Explorer → Generalist chain
2. Verify UI status updates correctly
3. Ensure child results propagate to parent
4. Test error handling in child agents

### Manual Testing
1. Launch nanocode with generalist agent
2. Use task tool to spawn explorer
3. Verify status line shows "explorer"
4. Explorer completes → status returns to "generalist"
5. Verify child result appears in parent's output

## Edge Cases & Error Handling

### Child Agent Failure
- Child agent errors should be captured as `ToolResult::error`
- Error propagated to parent via `ChildResult`
- Parent can handle error in next iteration

### Parent Agent Termination While Child Active
- If parent terminates (shutdown), child should also be terminated
- Stack cleanup on shutdown

### Multiple Nested Children
- Stack can handle arbitrary depth
- Each child gets fresh context
- Results propagate up chain correctly

## Backward Compatibility
- Existing single-agent workflows unchanged
- Headless mode falls back to inline execution
- Task/explore tools maintain same interface

## Future Extensions
1. Manual agent switching (keyboard shortcuts)
2. Agent context inheritance (optional)
3. Stack visualization in UI
4. Agent pause/resume capabilities

## Estimated Effort
- Core stack: 2-3 hours
- UI integration: 2-3 hours
- Tool updates: 1-2 hours
- Testing: 1-2 hours
- **Total**: 6-10 hours

## Dependencies
- No new external dependencies
- Uses existing `tokio::sync` channels
- Maintains current plugin architecture

This plan provides a complete roadmap for implementing agent stack support while maintaining backward compatibility and clean separation of concerns.