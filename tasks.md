# Remaining Tasks for Iterative Tool Call UI Updates

**Progress so far:** Tasks 1-5 completed (SharedMessages struct, App updates, Agent field, message formatting helpers, process() integration)

**Pending tasks:** None (All tasks 6-9 completed ✅)

**Implementation plan reference:** `docs/plans/2026-02-24-iterative-tool-call-ui-updates-consolidated-implementation-plan.md`

---

## Task 6: ✅ Update UI components to use shared messages

**Goal:** Update render_chat_history function to use app.shared_messages instead of app.messages for live UI updates.

**Files to modify:**
- `src/ui/components.rs:27-38` (render_chat_history function)
- `src/ui/components.rs:40-46` (render_tool_output function - optional)

**Key steps:**
1. Write failing test for updated render function in `tests/ui_components_test.rs`
2. Run test to verify it fails
3. Update `render_chat_history` function to use `app.shared_messages.take_messages()`
4. Fix scroll calculation to store message count (avoid calling `take_messages()` twice)
5. Optional: Update or remove `render_tool_output` function
6. Run compilation check
7. Commit changes

**Implementation details:**
- Messages should be taken once and stored: `let messages = app.shared_messages.take_messages();`
- Store message count: `let message_count = messages.len();`
- Use stored count for scroll calculation
- Tool output panel can remain empty for backward compatibility

---

## Task 7: ✅ Update main.rs to handle dirty flag for redraws

**Goal:** Update tick event handler to check dirty flag and force UI redraw when messages are updated.

**Files to modify:**
- `src/main.rs:92-126` (main loop)

**Key steps:**
1. Check current tick event handler (does nothing currently)
2. Update Tick handler to check `app.shared_messages.is_dirty()`
3. Force redraw by breaking out of match with `continue` when dirty
4. Run compilation check
5. Commit changes

**Implementation details:**
- Current tick handler: `Ok(Event::Tick) => { /* Update status or other periodic tasks */ }`
- Updated handler should check dirty flag and trigger redraw
- This ensures UI updates within 250ms tick rate

---

## Task 8: ✅ Add integration test for complete flow

**Goal:** Create integration test file to verify shared messages integration works end-to-end.

**Files to create:**
- `tests/iterative_ui_updates_test.rs`

**Key steps:**
1. Create integration test file with two tests:
   - `test_shared_messages_integration`: Verify agent accepts shared messages
   - `test_agent_process_without_api_key_still_works`: Verify process() fails gracefully without API key
2. Run integration tests
3. Commit test file

**Implementation details:**
- First test creates agent with shared messages and verifies setup
- Second test ensures agent.process() fails with API error (not panic) when no API key
- Both tests should pass

---

## Task 9: ✅ Clean up unused code and final validation

**Goal:** Remove unused code, run full test suite, fix warnings, finalize implementation.

**Files to check:**
- `src/ui/app.rs` for unused `messages` field references
- `src/ui/components.rs` for unused `tool_output` rendering
- Run full test suite and fix any warnings

**Key steps:**
1. Remove old `messages` field references from App struct (already replaced by `shared_messages`)
2. Check `tool_output` panel usage (keep as is for backward compatibility)
3. Run full test suite
4. Fix any compilation warnings with `cargo check --all-targets`
5. Final commit

**Implementation details:**
- Verify no references to old `messages` field remain
- `tool_output` field can stay (shows empty panel)
- All tests should pass (some may require API keys and fail)
- Fix any unused code warnings

---

## Notes

- **Current status:** All iterative tool call UI updates implemented ✅
- UI now shows tool calls as `Tool: name(args)` and results as `Result: output` or `Error: message`
- Updates appear incrementally during iterative tool calling (within 250ms tick rate)
- Backward compatibility maintained for user messages and final responses
- SharedMessages consolidated into separate module (`src/ui/shared_messages.rs`)
- Unused tool_output panel removed (3-panel layout now)
- Integration tests verify shared messages functionality

**Completed tasks (1-9):**
1. ✅ Create SharedMessages struct in ui/app.rs
2. ✅ Update App struct to use SharedMessages
3. ✅ Add shared_messages field to Agent struct
4. ✅ Add message formatting helpers to Agent
5. ✅ Integrate message pushing in process() method
6. ✅ Consolidate SharedMessages into separate module (`src/ui/shared_messages.rs`)
7. ✅ Remove unused tool_output panel from UI
8. ✅ Update main.rs tick handler to force redraw when messages dirty
9. ✅ Add integration tests for shared messages functionality

**Next steps after completion:**
- Manual testing with actual tool plugins
- Potential enhancements: color coding, instant updates, improved test coverage