# Codex TUI Layout and Spatial Organization

This document details the spatial layout, component hierarchy, and rendering mechanics of the Codex TUI. It complements the [UI overview](ui.md) by focusing on the precise arrangement of UI regions, the `Renderable` composition system, and the algorithms that position elements within the terminal.

## 1. High‑Level Layout

The terminal area is divided into three vertical regions, each with distinct behavior:

```
┌─────────────────────────────────────────────┐
│          Transcript Viewport                │
│  • History cells (immutable)                │
│  • Active cell (mutable, streaming)         │
│  • Overlay (Transcript, Static, Pager)      │
│                                             │
├─────────────────────────────────────────────┤
│          Bottom Pane                        │
│  – Status indicator (optional)              │
│  – Queued messages (optional)               │
│  – Pending approvals (optional)             │
│  – Composer / Popup View                    │
│  – Status line (optional)                   │
└─────────────────────────────────────────────┘
```

- **Viewport area** (`Rect`): top region that displays the conversation transcript.
- **Bottom‑pane area** (`Rect`): bottom region that contains input, status, and popups.
- **Overlay area** (`Rect`): when active, occupies the entire terminal screen, replacing both viewport and bottom pane.

## 2. Viewport Mechanics

### 2.1. Custom Terminal (`src/custom_terminal.rs`)

The `CustomTerminal` extends `ratatui::Terminal` to manage an **inline viewport** – a movable rectangle that sits above the bottom pane, preserving scrollback history above it.

Key concepts:

- **History lines**: committed transcript lines that are inserted into the terminal’s scrollback buffer _above_ the viewport, shifting existing terminal content up.
- **Viewport rectangle**: the on‑screen area reserved for rendering the transcript’s _current_ content (history cells + active cell).
- **Cursor tracking**: the viewport’s vertical position is adjusted on terminal resize to keep the cursor (active cell) at a consistent visual location.

### 2.2. Viewport Calculation (`src/tui.rs::draw()`)

The exact viewport rectangle is computed each frame:

1. **Initial bottom‑pane height** = height required by `bottom_pane.required_height()`.
2. **Available height** = terminal height − bottom‑pane height.
3. **Viewport height** = `available_height.max(1)` (at least one row).
4. **Viewport rectangle** = `Rect { x: 0, y: 0, width, height: viewport_height }`.
5. **Bottom‑pane rectangle** = `Rect { x: 0, y: viewport_height, width, height: bottom_pane_height }`.

If an overlay (`Overlay::Transcript`, `Overlay::Static`, `Overlay::Pager`) is active, the viewport and bottom pane are not drawn; the overlay occupies the entire terminal area.

### 2.3. Active‑Cell Positioning

The active (streaming) cell is rendered as the _last_ cell inside the viewport. The viewport scrolls automatically to keep the active cell visible, using `terminal.set_viewport_position()`.

## 3. Bottom‑Pane Structure

### 3.1. Stacked Container

The `BottomPane` (`src/bottom_pane/mod.rs`) is a vertical stack of optional components:

```
BottomPane
├── StatusIndicatorWidget (optional)
├── QueuedMessagesWidget (optional)
├── PendingApprovalsWidget (optional)
├── BottomPaneView (one of):
│   ├── ChatComposer (default)
│   ├── SelectionView (popup list)
│   ├── ApprovalRequestView
│   ├── RequestUserInputView
│   ├── FeedbackNoteView
│   └── StatusLineSetupView
└── StatusLine (optional)
```

Only the `BottomPaneView` is required; other elements appear conditionally based on UI state.

### 3.2. Height Distribution

The bottom pane’s `required_height()` iterates through each visible component and sums its `required_height()`.

- **Status indicator**: fixed 1 line.
- **Queued messages**: 1 line per queued message, up to a limit.
- **Pending approvals**: 1 line per thread with pending approvals.
- **Active view**: variable height (composer expands with content; popup views compute their own height).
- **Status line**: 1 line if any status‑line items are configured.

The total height is capped by the terminal height (the viewport always gets at least 1 row).

### 3.3. Input Routing

Key events are routed to the currently active `BottomPaneView`. The `BottomPane` decides which view receives input based on the view‑stack depth:

- If the stack has >1 view, the topmost view (a popup) receives input.
- Otherwise, the composer receives input.

Pressing `Esc` pops the topmost view off the stack.

## 4. Overlays

Overlays are full‑screen alternate‑buffer views that temporarily replace the entire UI.

### 4.1. Types (`src/pager_overlay.rs`)

```rust
pub enum Overlay {
    /// Scrollable transcript of all committed cells plus a live tail of the active cell.
    Transcript,
    /// Static content (e.g., side‑by‑side diff) with optional line numbers.
    Static {
        lines: Vec<String>,
        line_numbers: bool,
        left_line_numbers: bool,
    },
    /// Pager for large text blocks (help, logs).
    Pager(PagerState),
}
```

### 4.2. Rendering

When an overlay is active, `tui.draw()` skips the viewport and bottom‑pane rendering and instead calls `overlay.render(frame, terminal_area)`.

- **Transcript overlay** uses the same history‑cell rendering as the viewport, but without width truncation and with a scrollbar.
- **Static overlay** splits the screen for side‑by‑side diffs or shows line‑numbered text.
- **Pager overlay** pages through long text with `less`‑like navigation.

## 5. The `Renderable` Component System

### 5.1. Trait Definition (`src/render/renderable.rs`)

```rust
pub trait Renderable {
    fn required_height(&self, width: u16) -> u16;
    fn render(self, area: Rect, buf: &mut Buffer);
}
```

A `Renderable` can be:

- A simple widget (e.g., a label).
- A column of sub‑renderables (`ColumnRenderable`).
- A flex container that distributes space among children (`FlexRenderable`).

### 5.2. Column and Flex Containers

- **`ColumnRenderable`**: stacks children vertically. Each child’s `required_height` is summed; rendering proceeds top‑down.
- **`FlexRenderable`**: arranges children horizontally, distributing available width according to `FlexItem::Fixed` or `FlexItem::Flex` weights.

These containers are nested to build complex transcript cells (e.g., a timestamp, an icon, a content block, and a trailing status icon).

### 5.3. Usage in History Cells

Each `HistoryCell` implementation produces a `Box<dyn Renderable>` for display in the viewport, and a separate `Vec<String>` for the transcript overlay.

Example: `AgentMessageCell` builds a column containing:

1. A header row (timestamp + “Agent” label).
2. A flex row with an optional avatar icon and the markdown‑rendered content.
3. Optional footer lines (e.g., token‑usage summary).

### 5.4. Helper Functions

- `prefix_lines(prefix, lines)` – adds a prefix to each line (used for indentation).
- `word_wrap_lines(line, width)` – wraps a `Line` (ratatui) into multiple lines using textwrap.
- `simple_renderable(text)` – creates a single‑line renderable from a plain string.

## 6. Spatial Calculations and Constraints

### 6.1. Width Constraints

All width‑sensitive operations (wrapping, flex layout) use the **viewport width** (or overlay width) as the maximum available width.

- **Viewport width** = terminal width.
- **Overlay width** = terminal width (for transcript/pager) or terminal width / 2 (for side‑by‑side diff).

### 6.2. Height‑First Layout

Because the terminal scrolls vertically but not horizontally, layout is height‑first:

1. Compute the bottom‑pane’s required height.
2. Deduct that from terminal height to get viewport height.
3. Inside the viewport, each history cell is asked for its `required_height(viewport_width)`.
4. Cells are rendered sequentially until the viewport height is exhausted; earlier cells may be clipped if the transcript is very long.

### 6.3. Popup‑View Space Awareness

Popup views (e.g., `SelectionView`) adapt their dimensions to the available space:

- They compute a desired height based on item count and optional side‑content width.
- They clamp that height to a maximum proportion of the terminal height (e.g., 80%).
- They center themselves horizontally and vertically within the bottom‑pane area.

Example: `src/bottom_pane/request_user_input/layout.rs` shows a complex layout that splits available width between a label, an input field, and optional side hints.

## 7. Event‑Flow Details

### 7.1. Coordinate Mapping

Mouse events (if supported) are mapped to the appropriate region:

- If an overlay is active, the event is passed to the overlay.
- Otherwise, the y‑coordinate determines whether the event belongs to the viewport or bottom pane.
- Within the bottom pane, the event is dispatched to the currently active view.

### 7.2. Focus Management

Only one component has input focus at a time:

- Overlay > bottom‑pane view > composer.
- Focus is implicit; there is no visible focus indicator (the active view is the one that responds to keys).

### 7.3. Redraw Scheduling

Components request redraws via `FrameRequester`. A `Draw` event is emitted on the `TuiEventStream`, triggering `tui.draw()`.

Because the viewport inserts history lines into the terminal’s scrollback, a redraw may also cause the terminal to physically scroll. The `CustomTerminal` hides this complexity from the widget‑level drawing code.

## 8. Implementation Guidance

### 8.1. Building a Minimal Layout

To replicate the core layout without the full Codex codebase:

1. **Set up a ratatui terminal** with raw mode and alternate‑screen support.
2. **Create a viewport rectangle** that leaves room for a bottom pane.
3. **Implement a simple transcript renderer**:
   - Maintain a list of strings (history lines).
   - On each frame, render the lines that fit into the viewport rectangle.
4. **Build a bottom‑pane struct** that can switch between a text input and a popup list.
5. **Compute heights each frame**:
   ```rust
   let bottom_height = bottom_pane.required_height();
   let viewport_height = terminal_height.saturating_sub(bottom_height).max(1);
   let viewport_rect = Rect::new(0, 0, terminal_width, viewport_height);
   let bottom_rect = Rect::new(0, viewport_height, terminal_width, bottom_height);
   ```
6. **Route key events** to the bottom pane; let the bottom pane delegate to its active view.

### 8.2. Adding the Renderable System

Start with a basic `Renderable` trait and two containers:

```rust
struct ColumnRenderable(Vec<Box<dyn Renderable>>);
struct FlexRenderable(Vec<FlexItem>);
```

Use them to build transcript cells that combine timestamps, icons, and wrapped text.

### 8.3. Simulating the Inline Viewport

Without `CustomTerminal`, you can approximate the inline viewport by:

- Using the alternate screen (no scrollback).
- Or, simulating scrollback by prepending history lines to the transcript list and always rendering the most recent lines.

The real `CustomTerminal` is necessary only if you need to preserve the terminal’s native scrollback buffer while the UI is active.

### 8.4. Testing Layouts

- Use `insta` snapshot tests to capture rendered output for given terminal dimensions.
- Write unit tests that verify `required_height()` calculations for various widths.
- For interactive tests, enable the `vt100‑tests` feature to simulate terminal resizes and key presses.

## 9. Key Files for Reference

| File                                           | Purpose                                                         |
| ---------------------------------------------- | --------------------------------------------------------------- |
| `src/custom_terminal.rs`                       | Inline viewport implementation, scrollback insertion.           |
| `src/tui.rs::draw()`                           | Layout calculation, region splitting.                           |
| `src/chatwidget.rs::as_renderable()`           | Builds the viewport’s column of history cells.                  |
| `src/bottom_pane/mod.rs::as_renderable()`      | Builds the bottom‑pane stack.                                   |
| `src/render/renderable.rs`                     | `Renderable`, `ColumnRenderable`, `FlexRenderable` definitions. |
| `src/bottom_pane/request_user_input/layout.rs` | Example of space‑aware popup layout.                            |
| `src/pager_overlay.rs`                         | Overlay rendering and navigation.                               |
| `src/history_cell.rs`                          | How each cell type produces a `Renderable`.                     |

## 10. Summary

The Codex TUI layout is a **height‑first, region‑split** system:

1. **Terminal area** is partitioned into viewport, bottom pane, and optionally an overlay.
2. **Viewport** displays a scrollable column of history cells, with an active cell at the bottom.
3. **Bottom pane** is a vertical stack of conditional components, with a single active view that receives input.
4. **Overlays** replace the entire screen for focused viewing (transcript, diff, pager).
5. **`Renderable` trait** enables flexible composition of widgets within fixed‑width, variable‑height rectangles.

All spatial decisions are made anew each frame based on terminal size and component‑reported heights, ensuring the UI adapts to any terminal dimension.

---

_Based on the code in `codex-rs/tui/` as of February 2026._
