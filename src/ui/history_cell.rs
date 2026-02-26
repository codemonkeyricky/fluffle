use crate::ui::render::{simple_renderable, ColumnRenderable, Renderable};

/// A cell in the conversation transcript that can be rendered in the viewport
/// and exported as plain text for the transcript overlay.
pub trait HistoryCell: Send + Sync {
    /// Create a renderable representation of this cell for the given width.
    fn as_renderable(&self, _width: u16) -> Box<dyn Renderable>;

    /// Return plain‑text lines suitable for the transcript overlay.
    fn transcript_lines(&self) -> Vec<String>;
}

/// A simple history cell containing plain text.
pub struct PlainHistoryCell {
    text: String,
}

impl PlainHistoryCell {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

impl HistoryCell for PlainHistoryCell {
    fn as_renderable(&self, _width: u16) -> Box<dyn Renderable> {
        simple_renderable(self.text.clone())
    }

    fn transcript_lines(&self) -> Vec<String> {
        vec![self.text.clone()]
    }
}

/// A history cell for agent messages (supports markdown rendering).
pub struct AgentMessageCell {
    content: String,
    // TODO: add fields for timestamp, avatar, etc.
}

impl AgentMessageCell {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

impl HistoryCell for AgentMessageCell {
    fn as_renderable(&self, _width: u16) -> Box<dyn Renderable> {
        // Build a column with a header and content.
        let mut column = ColumnRenderable::new();
        // Header: "Agent" label (placeholder)
        column.push_mut(simple_renderable("Agent".to_string()));
        // Content lines wrapped
        let content_renderable = simple_renderable(self.content.clone());
        column.push_mut(content_renderable);
        Box::new(column)
    }

    fn transcript_lines(&self) -> Vec<String> {
        vec![self.content.clone()]
    }
}
