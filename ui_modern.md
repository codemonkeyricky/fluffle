# Nanocode UI Modernization Plan

## 1. Current State

Nanocode's UI is a minimal TUI built with `ratatui` and `crossterm`. It provides:

- A transcript viewport showing history cells (plain text and agent messages)
- A bottom pane with a simple chat composer (single‑line input, no popups)
- A status line with model/provider/token info
- Basic event handling (keyboard input, tick events)
- A pluggable UI trait that supports headless mode

The implementation consists of:

- `src/ui/advanced_tui.rs` – main UI loop, layout, and drawing
- `src/ui/bottom_pane/` – composer and view stack (only composer implemented)
- `src/ui/history_cell.rs` – `PlainHistoryCell` and `AgentMessageCell`
- `src/ui/render/` – `Renderable` trait, column/flex containers
- `src/ui/event.rs` – event polling and channel
- `src/ui/ui_trait.rs` – UI backend abstraction

Missing features compared to a mature Codex‑like TUI:

- No markdown/syntax highlighting
- No tool‑call, exec‑output, MCP, or web‑search cells
- No transcript overlay (`Ctrl+T`)
- No popup views (selection lists, approval requests, user‑input prompts)
- No slash‑command completion, image attachments, or mentions
- No status indicator (spinner + elapsed time)
- No queued‑messages or pending‑approval displays
- No unified‑exec footer
- No external editor support (`Ctrl+E`)
- No diff/pager overlays
- No multi‑agent thread switching
- No configurable status line items (git branch, directory, etc.)
- No desktop‑notification integration
- No mouse or resize event handling
- No custom terminal viewport (scrollback‑aware inline mode)

## 2. Comparison with Codex UI

Codex’s TUI (`codex-rs/tui/`) is a full‑featured terminal interface described in `ui.md` and `layout.md`. Key differences:

| Feature | Nanocode | Codex |
|---------|----------|-------|
| **Transcript cells** | Plain text, simple agent message | Agent messages (markdown), exec cells, tool calls, web search, session info, unified exec, etc. |
| **Rendering** | Basic column layout | `Renderable` system with flex containers, icons, timestamps, side‑by‑side diffs |
| **Bottom‑pane stack** | Single composer | Composer + popup views (selection, approval, input, feedback, status‑line setup) |
| **Overlays** | None | Transcript, static (diff), pager |
| **Input features** | Plain text | Slash commands, image attachments, mentions, external editor, paste‑burst detection |
| **Status indicator** | None | Spinner + elapsed time + detail text |
| **Multi‑agent** | None | Thread picker, pending approvals, queued messages |
| **Terminal mode** | Alternate screen only | Alternate‑screen / inline detection (Zellij‑aware), scrollback insertion |
| **Styling** | Hard‑coded colors | ANSI‑color‑only style guide, theme support |
| **Testing** | None | Snapshot tests, VT100 emulation, unit tests |

## 3. Modernization Goals

Bring nanocode’s UI closer to Codex’s level of polish and functionality while keeping the codebase manageable. Prioritize features that:

1. Improve user experience (transcript overlay, popup selection, slash commands)
2. Enable richer content display (markdown, syntax highlighting, diff rendering)
3. Maintain compatibility with the existing plugin/tool architecture
4. Follow the same layout and rendering patterns as Codex for easier future integration

## 4. Phase 1 – Foundation and Essential UX

**Objective:** Implement the core UI infrastructure missing from nanocode, focusing on layout, rendering, and basic interactivity.

### 4.1. Custom Terminal Viewport (`src/ui/custom_terminal.rs`)
- Port the `CustomTerminal` from Codex (or write a simplified version) that supports inline viewport with scrollback insertion.
- Detect multiplexers (Zellij) and adjust alternate‑screen behavior accordingly.
- Add bracketed‑paste and keyboard‑enhancement flags.

### 4.2. Enhanced History Cells
- Extend `HistoryCell` trait with `display_lines()` and `transcript_lines()` separation.
- Implement `ExecCell`, `ToolCallCell`, `McpToolCallCell`, `PlainHistoryCell` with styling.
- Add markdown rendering (`src/ui/markdown.rs`) using `pulldown‑cmark` + ANSI conversion.
- Add syntax highlighting for code blocks (leverage `syntect` or `two‑face`).

### 4.3. Renderable System Completion
- Ensure `ColumnRenderable` and `FlexRenderable` match Codex’s API.
- Add helper functions: `prefix_lines`, `word_wrap_lines`, `simple_renderable`.
- Use the renderable system to build cells with headers, icons, and flexible spacing.

### 4.4. Bottom‑Pane Popup Views
- Implement `SelectionView` (generic filter‑as‑you‑type list with side content).
- Implement `ApprovalRequestView` (Yes/No/Always/Never overlay).
- Implement `RequestUserInputView` (free‑text prompt).
- Extend `BottomPaneView` enum to include these variants.
- Add view‑stack management (`push_view`/`pop_view`) and input routing.

### 4.5. Transcript Overlay (`Ctrl+T`)
- Add `Overlay` enum (`Transcript`, `Static`, `Pager`).
- Integrate overlay rendering into `AdvancedTerminalUi::run()`.
- Add key binding (`Ctrl+T`) to toggle transcript overlay.

### 4.6. Status Indicator Widget
- Create `StatusIndicatorWidget` (spinner, elapsed time, detail text).
- Show it above the composer when a task is running (agent turn, tool execution).

## 5. Phase 2 – Rich Content and Input

**Objective:** Add features that make the UI more expressive and efficient to use.

### 5.1. Chat Composer Enhancements
- Multi‑line input with `Shift+Enter` for newline.
- Slash‑command detection and popup (`/help`, `/clear`, `/model`, etc.).
- Image attachment placeholders (paste/drag‑and‑drop detection).
- Mention autocomplete (`$skill`, `$app`).
- External editor support (`Ctrl+E`).
- Queue‑edit shortcut (`Alt+Up` to recall the most recent queued message).

### 5.2. Diff and Pager Overlays
- Implement side‑by‑side diff rendering (`src/ui/diff_render.rs`).
- Add pager overlay for large text blocks (help, logs).
- Trigger diff overlay after patch applications.

### 5.3. Status Line Configuration
- Replace hard‑coded status line with configurable items (model, provider, directory, git branch, token usage, etc.).
- Add `StatusLine` widget that reads from a configuration list.
- Allow user to toggle items via a setup popup.

### 5.4. Multi‑Agent Support
- Add thread‑event channels for multiple agent threads.
- Implement thread‑picker overlay (`Ctrl+P`).
- Display pending approvals and queued messages.

### 5.5. Desktop Notifications
- Detect desktop‑notification backend (`notify‑rust`, `terminal‑notifier`).
- Suppress notifications when terminal is focused.

## 6. Phase 3 – Polish and Ecosystem Integration

**Objective:** Refine the UI, improve performance, and ensure it works seamlessly with the rest of the nanocode ecosystem.

### 6.1. Styling Conformance
- Adopt the ANSI‑color‑only style guide (no `blue`/`yellow`, avoid `white`/`black`).
- Use `ratatui::style::Stylize` helpers (`bold()`, `dim()`, `cyan()`, etc.).
- Add theme‑picker overlay (optional).

### 6.2. Mouse and Resize Handling
- Add mouse‑event support for click‑to‑focus, scrollbars.
- Handle terminal‑resize events gracefully (reflow content).

### 6.3. Performance Optimizations
- Batch draw calls with `crossterm::SynchronizedUpdate`.
- Cache rendered cells where possible.
- Use `FrameRequester` to limit redraws to when needed.

### 6.4. Testing Infrastructure
- Add snapshot tests (`insta`) for UI components.
- Write VT100 emulation tests for interactive sequences.
- Unit test `required_height()` and rendering of each cell type.

### 6.5. Plugin‑UI Integration
- Ensure new tool‑call and tool‑result cells are automatically displayed.
- Allow plugins to register custom overlay views (e.g., a file‑tree browser).
- Expose UI events to plugins (key bindings, menu additions).

## 7. Implementation Strategy

### 7.1. Incremental Approach
- Start by copying small, self‑contained files from Codex (`custom_terminal.rs`, `status_indicator_widget.rs`).
- Adapt them to nanocode’s error handling and async patterns.
- Extend the existing `Renderable` system rather than replacing it.
- Add one feature at a time, verifying each step works with the existing UI.

### 7.2. Codex Code as Reference
- Use `codex-rs/tui/` as a reference implementation, but do not copy entire modules wholesale.
- Understand the architecture first (`ui.md`, `layout.md`), then implement analogous components.
- Keep dependencies minimal; only add crates that are absolutely needed.

### 7.3. Compatibility with Headless Mode
- Ensure all new UI features are behind the `AdvancedTerminalUi`; headless mode remains unchanged.
- The `Ui` trait should not require changes for headless operation.

## 8. Recommended Priorities

1. **Immediate** (next 1‑2 weeks):
   - Custom terminal viewport (enables inline mode)
   - Enhanced history cells (markdown, exec cells)
   - Transcript overlay (`Ctrl+T`)
   - Selection popup (for slash‑command completion)

2. **Short‑term** (next month):
   - Status indicator widget
   - Diff/pager overlays
   - Chat composer multi‑line & slash commands
   - Configurable status line

3. **Medium‑term** (next 2‑3 months):
   - Multi‑agent support
   - Image attachments & mentions
   - External editor
   - Desktop notifications

4. **Long‑term** (future):
   - Mouse/resize handling
   - Theme picker
   - Plugin‑UI extensions
   - Comprehensive test suite

## 9. Risks and Mitigations

- **Complexity explosion**: Keep each phase small and focused; avoid “big bang” rewrites.
- **Performance degradation**: Profile drawing and event handling; use caching where needed.
- **Dependency bloat**: Evaluate each new crate; prefer lightweight alternatives.
- **Divergence from Codex**: Regularly sync with Codex’s `tui/` changes to stay aligned.

## 10. Conclusion

Modernizing nanocode’s UI will significantly improve user experience and bring it closer to the standard set by Codex. The phased approach allows for incremental delivery while maintaining a stable core. Start with Phase 1 to establish the missing infrastructure, then progressively add richer features.

This plan will evolve as work progresses; revisit it after each major milestone.

---

*Generated on 2026‑02‑26 based on analysis of nanocode `src/ui/` and Codex `codex-rs/tui/`.*