# Task Plan: Implement Explore Subagent Feature

## Goal
Implement a generic subagent system in the AI SDK, plus a dedicated explore subagent tool with predefined system prompt for exploring codebases. The explore subagent should run with its own prompt and conversation history, returning a summary back to the main context.

## Current Phase
Phase 6 (Complete)

## Phases

### Phase 1: Requirements & Discovery
- [x] Understand user intent and scope (done via questions)
- [x] Analyze existing codebase structure and subagent plan
- [x] Identify required changes to AI providers for system prompts (decided: prepend user message)
- [x] Document findings in findings.md
- [x] Write detailed implementation plan to docs/plan/explore-subagent-implementation.md
- **Status:** complete

### Phase 2: Design & Planning
- [x] Design subagent configuration and creation API (extend Agent with system_prompt field)
- [x] Design system prompt injection mechanism (prepend as user message)
- [x] Design plugin filtering and tool selection (no filtering initially, all tools)
- [x] Design TaskTool and ExploreTool parameters (task: description, system_prompt; explore: description)
- [x] Design conversation summarization (return final assistant response)
- [x] Document decisions with rationale (in findings.md and implementation plan)
- **Status:** complete

### Phase 3: Core Infrastructure
- [x] Add system_prompt field to Agent struct
- [x] Add Agent::with_system_prompt() method for subagent creation
- [x] Add Agent::with_context() and set_context() methods
- [x] Add Clone derive to ToolContext
- [x] Fix missing ToolResult struct definition
- **Status:** complete

### Phase 4: Task Tool Implementation
- [x] Create src/plugins/task.rs with TaskTool implementation
- [x] Create src/plugins/explore.rs with ExploreTool implementation
- [x] Register TaskPlugin and ExplorePlugin in plugins/mod.rs
- [x] Write unit tests for new plugins
- [x] Add agent tests for tool discovery and system prompt
- **Status:** complete

### Phase 5: Integration & Testing
- [x] Run existing tests to ensure no regressions (all tests pass)
- [x] Add agent tests for tool discovery and system prompt
- [x] Verify subagent creation works via unit tests
- [ ] Optional: Write integration tests with real AI provider (requires API key)
- **Status:** complete

### Phase 6: Documentation & Cleanup
- [x] Update README with subagent system documentation
- [x] Ensure code style consistency (run cargo fmt)
- [x] Final review and handoff
- **Status:** complete

## Key Questions (Answered)
1. **Subagent implementation**: Configured Agent instance with system_prompt field (extend Agent)
2. **System prompt injection**: Prepend as first user message (simpler, no provider changes)
3. **Default explore prompt**: Drafted in implementation plan (expert codebase explorer prompt)
4. **Plugin filtering**: No filtering initially, all tools available to subagents
5. **Config inheritance**: Subagents inherit parent config (temperature, max_tool_iterations, etc.)
6. **Explore tool design**: Separate ExploreTool that calls TaskTool with predefined prompt

## Decisions Made
| Decision | Rationale |
|----------|-----------|
| Implement generic subagent system + separate explore tool | User chose this option |
| Subagents run synchronously | User chose synchronous execution |
| Subagents have isolated history | User chose isolated history |
| No timeout for subagents | User chose no timeout |
| Prepend system prompt as user message | Simpler, no AI provider changes required |
| Subagents inherit parent config | Consistent behavior, can be extended later |
| All tools for subagents (no filtering) | Simpler initial implementation |
| Generic TaskTool first, then ExploreTool wrapper | Reusable infrastructure |
| Draft default explore system prompt | Provided in implementation plan |

## Errors Encountered
| Error | Attempt | Resolution |
|-------|---------|------------|
|       | 1       |            |

## Notes
- Update phase status as you progress: pending → in_progress → complete
- Re-read this plan before major decisions (attention manipulation)
- Log ALL errors - they help avoid repetition
- Never repeat a failed action - mutate your approach instead