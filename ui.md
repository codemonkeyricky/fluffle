# Codex TUI UI Stack and Layout Summary

This document provides a comprehensive overview of the UI architecture, layout, and components in the `codex-rs/tui/` module. It is intended to give an agent enough context to implement a reasonable approximation of the Codex terminal user interface.

## 1. Overview

The Codex TUI is a terminal‑based chat interface for interacting with the Codex agent. It is built on the Rust [`ratatui`](https://github.com/ratatui-rs/ratatui) library with [`crossterm`](https://github.com/crossterm-rs/crossterm) as the backend. The UI is designed to work both in a full‑screen alternate buffer and inline (scrollback‑friendly) mode, with support for multiplexer detection (e.g., Zellij) to adjust behavior.

Core responsibilities:

- Display a scrollable transcript of conversation turns, tool calls, command output, and system messages.
- Provide an editable prompt input with support for images, mentions, and slash commands.
- Show status indicators, running‑task spinners, and approval overlays.
- Manage multiple agent threads (primary + sub‑agents) with a thread‑picker overlay.
- Render syntax‑highlighted code, markdown, diffs, and animated progress indicators.

## 2. UI Stack

### 2.1. Dependencies

- **ratatui** – terminal UI widget library (with custom extensions in `custom_terminal.rs`).
- **crossterm** – terminal I/O, raw mode, alternate screen, keyboard events.
- **syntect** – syntax highlighting (via `two‑face`).
- **textwrap** – line wrapping utilities.
- **pulldown‑cmark** – markdown parsing for rendering.
- **image** – image attachment thumbnail generation.

### 2.2. Terminal Modes

- **Raw mode** – enabled for direct key handling.
- **Bracketed paste** – enabled for safe paste handling.
- **Keyboard enhancement flags** – enabled where supported (disambiguates modifier keys).
- **Alternate screen** – configurable (`always`, `never`, `auto`); auto‑detects Zellij to disable alternate screen and preserve scrollback.
- **Focus events** – used to suppress desktop notifications while terminal is focused.

### 2.3. Custom Terminal Extensions

The `custom_terminal` module (`src/custom_terminal.rs`) extends `ratatui::Terminal` with:

- **Inline viewport** – a movable rectangle that sits above the bottom pane, leaving scrollback history above it.
- **Viewport tracking** – adjusts viewport position on terminal resize to keep the cursor in a consistent place.
- **Scrollback‑aware clearing** – `clear_scrollback_and_visible_screen_ansi()` for inline mode.
- **Synchronized updates** – batches drawing commands to reduce flicker.

## 3. Layout Structure

The UI is divided into three vertical regions:

```
┌─────────────────────────────────────────────┐
│          Transcript (viewport)              │
│                                             │
│  • History cells (committed output)         │
│  • Active cell (streaming/exec in progress) │
│  • Overlay (Ctrl+T transcript, diff, pager) │
│                                             │
├─────────────────────────────────────────────┤
│          Bottom Pane                        │
│  – Status indicator (when task running)     │
│  – Queued messages (if turn active)         │
│  – Pending thread approvals (multi‑agent)   │
│  – Composer (prompt input)                  │
│  – Status line (optional footer)            │
└─────────────────────────────────────────────┘
```

### 3.1. Transcript Viewport

- Occupies the top portion of the screen; its height expands to fit content up to the available terminal height.
- Contains **history cells** – immutable rendered blocks representing past turns, tool calls, warnings, etc.
- Includes an **active cell** that mutates in‑place during streaming (e.g., exec output, agent reasoning).
- Supports an **overlay** (`Ctrl+T`) that shows a scrollable transcript of all committed cells plus a live tail of the active cell.

### 3.2. Bottom Pane

A stacked container that can show one of several views:

1. **Composer** (`ChatComposer`) – the default view, a multi‑line text input with:
   - Slash‑command completion.
   - Image attachment placeholders (`[Image #1]`).
   - Mention autocomplete (`$skill`, `$app`).
   - Paste‑burst detection (rapid pastes are collapsed).
   - Support for external editor (`Ctrl+E`).

2. **Popup views** (`BottomPaneView`) – transient modal overlays that replace the composer:
   - Selection lists (e.g., skill picker, model picker, thread picker).
   - Approval request overlays (Yes/No/Always/Never).
   - User‑input prompts (free‑form text, choices).
   - Feedback note composer.
   - Status‑line setup.

3. **Status indicator** (`StatusIndicatorWidget`) – appears above the composer when a task is running (agent turn or MCP startup). Shows a spinner, elapsed time, and optional detail text.

4. **Queued user messages** – when a turn is already in progress, additional user messages are queued and displayed as a small stack above the composer.

5. **Pending thread approvals** – shows a list of inactive agent threads that have outstanding approval requests.

6. **Unified exec footer** – when a unified‑exec session is active, shows a summary of running processes.

7. **Status line** – an optional footer line that displays configurable items (model, directory, git branch, token usage, etc.). Rendered only when at least one item is configured.

### 3.3. Overlays

- **Transcript overlay** (`Ctrl+T`) – a full‑screen scrollable view of the entire conversation.
- **Diff overlay** – side‑by‑side diff rendering (e.g., after a patch application).
- **Pager overlay** – for viewing large blocks of text (e.g., `?` help).
- **External editor** – temporarily suspends the TUI to open `$EDITOR`.

## 4. Components

### 4.1. `ChatWidget` (`src/chatwidget.rs`)

The central UI controller. Owns:

- **History cells** – a vector of `Box<dyn HistoryCell>` representing the transcript.
- **Active cell** – a mutable history cell for streaming output.
- **Bottom pane** – the `BottomPane` instance.
- **Stream controllers** – manage streaming of agent messages and plan updates.
- **Thread‑event channels** – per‑thread event buffers for multi‑agent switching.
- **State machines** for rate‑limit warnings, model migration prompts, connectors cache, etc.

Key methods:

- `handle_codex_event()` – processes protocol events (e.g., `AgentMessageDelta`, `ExecCommandOutputDelta`).
- `submit_user_message()` – sends a user message to the agent.
- `show_selection_view()` – opens a popup selection list.
- `refresh_status_line()` – recomputes the footer status line.

### 4.2. `BottomPane` (`src/bottom_pane/mod.rs`)

Owns the composer and a stack of views. Handles:

- **Input routing** – decides which view receives key events.
- **View lifecycle** – pushes/pops views on the stack.
- **Time‑based hints** – e.g., “Press again to quit” expiration.
- **Status indicator** – visibility and content updates.

### 4.3. `ChatComposer` (`src/bottom_pane/chat_composer.rs`)

A multi‑line textarea with custom behavior:

- **Enter** – submits the message (unless `Shift+Enter` or `Alt+Enter` for newline).
- **Slash commands** – typed `/` triggers a popup with available commands.
- **Image attachments** – paste an image (or drag‑and‑drop in supporting terminals) creates a temporary PNG and inserts a placeholder.
- **Mentions** – typing `$` triggers autocomplete for skills and apps.
- **External editor** – `Ctrl+E` opens the current draft in `$EDITOR`.
- **Queue‑edit shortcut** – `Alt+Up` (or `Shift+Left` on certain terminals) pops the most recently queued message back into the composer.

### 4.4. `HistoryCell` (`src/history_cell.rs`)

Trait representing a renderable block of transcript content. Implementations:

- `AgentMessageCell` – streaming assistant output with markdown rendering.
- `ExecCell` – shell command execution with animated output.
- `McpToolCallCell` – MCP tool call request/response.
- `WebSearchCell` – web‑search progress and results.
- `PlainHistoryCell` – static lines (e.g., warnings, session headers).
- `SessionInfoCell` – welcome banner, model, directory, sandbox status.
- `UnifiedExecInteractionCell` – unified‑exec command and output.

Each cell produces:

- **Display lines** – for rendering in the viewport (width‑constrained, wrapped).
- **Transcript lines** – for the transcript overlay (full width, no truncation).

### 4.5. `Renderable` Abstraction (`src/render/renderable.rs`)

A flexible layout system that allows widgets to be arranged in columns, rows, or flex containers. Used by history cells to combine labels, icons, and content with appropriate spacing.

### 4.6. `StatusIndicatorWidget` (`src/status_indicator_widget.rs`)

Displays a spinner, elapsed time, and optional detail text. Supports:

- **Capitalization modes** – for detail text (capitalize first, uppercase, etc.).
- **Line limiting** – details can be limited to N lines.
- **Animation** – spinner rotates on every draw tick.

### 4.7. `SelectionView` (`src/bottom_pane/list_selection_view.rs`)

A generic popup list with optional side‑by‑side content (e.g., skill description). Features:

- **Filter‑as‑you‑type**.
- **Keyboard navigation** (Up/Down, Page Up/Down, Home/End).
- **Side‑content width** adjustable based on terminal size.
- **Custom actions** per item.

## 5. Styling Conventions

See `styles.md` for the definitive guide. Highlights:

- **Headers** – `bold`.
- **Primary text** – default foreground.
- **Secondary text** – `dim`.
- **User input tips, selection, status indicators** – `cyan`.
- **Success/additions** – `green`.
- **Errors/failures/deletions** – `red`.
- **Codex branding** – `magenta`.
- **Avoid custom colors** – stick to ANSI colors for theme compatibility.
- **Avoid `white` and `black`** – use default foreground/background.
- **Avoid `blue` and `yellow`** – not used in the current style guide.

Helpers from `ratatui::style::Stylize` are preferred:

```rust
"text".bold()
"text".dim()
"text".cyan()
"text".green()
"text".red()
"text".magenta()
```

For computed styles, use `Span::styled` or `set_style`.

## 6. Event Handling

### 6.1. Event Flow

```
Terminal → crossterm → TuiEventStream → App::handle_tui_event() → ChatWidget → BottomPane → View
```

- **`TuiEvent`** – enum of `Key`, `Paste`, `Draw`.
- **`AppEvent`** – internal UI events (insert history cell, show popup, update status, etc.).
- **Protocol events** (`Event`) – received from codex‑core via thread‑event channels.

### 6.2. Key Bindings (Partial List)

- `Enter` – submit message / confirm selection.
- `Ctrl+C` – interrupt running task / first press of quit shortcut.
- `Ctrl+D` – quit (same double‑press logic as Ctrl+C).
- `Ctrl+T` – toggle transcript overlay.
- `Ctrl+E` – open external editor.
- `Ctrl+R` – retry last turn (when in error state).
- `Esc` – dismiss popup / backtrack through view stack.
- `Tab` – next completion candidate.
- `Shift+Tab` – previous completion candidate.
- `Alt+Up` / `Shift+Left` – edit most recently queued message.
- `Ctrl+Shift+C` – copy last copyable output.
- `Ctrl+Shift+V` – paste from clipboard as text.
- `Ctrl+Shift+P` – paste from clipboard as image.

### 6.3. Double‑Press Quit Shortcut

When `DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED` is `true` (currently `false`), Ctrl+C or Ctrl+D must be pressed twice within `QUIT_SHORTCUT_TIMEOUT` (1 second) to exit. The first press shows a “Press again to quit” hint in the footer.

## 7. Rendering Pipeline

### 7.1. Frame Scheduling

- **`FrameRequester`** – allows any component to request a redraw.
- **`Tui::draw()`** – called on each `Draw` event or when scheduled.
- **Synchronized update** – wraps the entire draw in `crossterm::SynchronizedUpdate` to reduce flicker.

### 7.2. Drawing Steps

1. Compute viewport area based on terminal size and cursor position.
2. Insert pending history lines (from `tui.insert_history_lines()`).
3. Draw history cells into the viewport.
4. Draw the bottom pane (composer or active view).
5. If an overlay is active, draw it over the entire screen.

### 7.3. Custom Terminal Drawing

The `custom_terminal` module handles:

- **Scrollback insertion** – history lines are inserted above the viewport, shifting existing content up.
- **Viewport anchoring** – keeps the viewport at a fixed vertical position unless resizing/moving requires adjustment.
- **Alternate‑screen rendering** – when active, the viewport occupies the whole screen.

## 8. Testing

### 8.1. Snapshot Tests (`insta`)

- UI output is captured as plain‑text snapshots.
- Run with `cargo test -p codex-tui`; update snapshots with `cargo insta accept -p codex-tui`.
- Snapshots live in `src/snapshots/`.

### 8.2. VT100 Tests (`--features vt100-tests`)

- Emulates a terminal using the `vt100` crate.
- Allows testing interactive sequences (key presses, paste events) without a real terminal.

### 8.3. Unit Tests

- Individual components are tested in isolation (e.g., `chat_composer`, `history_cell`).
- Use `pretty_assertions` for readable diff output.

## 9. Key Files and Directories

| Path                             | Purpose                                           |
| -------------------------------- | ------------------------------------------------- |
| `src/app.rs`                     | Main application state machine.                   |
| `src/chatwidget.rs`              | Central UI controller.                            |
| `src/bottom_pane/`               | Composer, popups, status indicator.               |
| `src/history_cell.rs`            | Transcript block implementations.                 |
| `src/tui.rs`                     | Terminal setup, event stream, draw orchestration. |
| `src/custom_terminal.rs`         | Extended terminal with inline viewport.           |
| `src/render/`                    | Layout utilities and `Renderable` trait.          |
| `src/status_indicator_widget.rs` | Spinner + elapsed time widget.                    |
| `src/wrapping.rs`                | Text‑wrapping helpers.                            |
| `src/markdown.rs`                | Markdown→ANSI renderer.                           |
| `src/diff_render.rs`             | Side‑by‑side diff rendering.                      |
| `src/streaming/`                 | Stream animation controllers.                     |
| `src/notifications/`             | Desktop notification backend detection.           |
| `frames/`                        | ASCII‑art frames for session headers.             |
| `styles.md`                      | Color and style guidelines.                       |
| `tooltips.txt`                   | Randomly shown tooltips.                          |

## 10. How to Implement an Approximation

To build a minimal version of the Codex TUI, focus on the following core pieces:

1. **Set up ratatui + crossterm** with raw mode, alternate screen, and bracketed paste.
2. **Implement a custom terminal viewport** that can insert history lines above a fixed bottom pane.
3. **Create a transcript renderer** that can display a list of history cells (simple text blocks to start).
4. **Build a bottom pane** with:
   - A multi‑line text input (basic slash‑command detection).
   - A status indicator that shows “Working…” when a task is running.
5. **Handle key events** for submit, interrupt, quit, and overlay toggling.
6. **Connect to a backend** that provides event streams (agent messages, exec output, etc.) and translate them into history cells.
7. **Add styling** following the ANSI color guidelines.

Omit advanced features initially:

- Multiple agent threads.
- Image attachments.
- Mentions and skill autocomplete.
- Unified exec footer.
- Desktop notifications.
- Rate‑limit warnings.
- Model migration prompts.

Once the basic chat loop works, incrementally add:

- Slash‑command popup.
- Transcript overlay (`Ctrl+T`).
- Selection lists for skills/models.
- Diff rendering for patch output.

Refer to the existing code for patterns on event handling, cell rendering, and layout calculations. The `Renderable` trait is particularly useful for building complex transcript blocks.

---

_This summary is based on the code as of February 2026. For the latest details, consult the source files directly._
