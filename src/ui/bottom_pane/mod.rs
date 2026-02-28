use crate::ui::render::Renderable;
use ratatui::{buffer::Buffer, layout::Rect};

/// The bottom pane of the UI, containing a stack of views.
pub struct BottomPane {
    /// Stack of views; the topmost view receives input.
    view_stack: Vec<BottomPaneView>,
}

#[allow(unreachable_patterns)]
impl BottomPane {
    pub fn new() -> Self {
        Self {
            view_stack: vec![BottomPaneView::ChatComposer(ChatComposer::default())],
        }
    }

    /// Push a new view onto the stack.
    pub fn push_view(&mut self, view: BottomPaneView) {
        self.view_stack.push(view);
    }

    /// Pop the topmost view from the stack, returning to the previous view.
    pub fn pop_view(&mut self) -> Option<BottomPaneView> {
        if self.view_stack.len() > 1 {
            self.view_stack.pop()
        } else {
            None
        }
    }

    /// Returns a mutable reference to the active view (top of stack).
    pub fn active_view_mut(&mut self) -> &mut BottomPaneView {
        self.view_stack.last_mut().expect("at least one view")
    }

    /// Returns a reference to the active view.
    pub fn active_view(&self) -> &BottomPaneView {
        self.view_stack.last().expect("at least one view")
    }

    /// Returns a mutable reference to the active chat composer, if the active view is a composer.
    pub fn active_composer_mut(&mut self) -> Option<&mut ChatComposer> {
        match self.active_view_mut() {
            BottomPaneView::ChatComposer(composer) => Some(composer),
            _ => None,
        }
    }

    /// Returns a reference to the active chat composer, if the active view is a composer.
    pub fn active_composer(&self) -> Option<&ChatComposer> {
        match self.active_view() {
            BottomPaneView::ChatComposer(composer) => Some(composer),
            _ => None,
        }
    }
}

impl Renderable for BottomPane {
    fn required_height(&self, width: u16) -> u16 {
        // Sum of heights of all visible components.
        // For now, just the active view.
        self.active_view().required_height(width)
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Render only the active view for now.
        self.active_view_mut().render(area, buf);
    }
}

/// A view that can be displayed in the bottom pane.
pub enum BottomPaneView {
    ChatComposer(ChatComposer),
    // TODO: SelectionView, etc.
}

impl Renderable for BottomPaneView {
    fn required_height(&self, width: u16) -> u16 {
        match self {
            BottomPaneView::ChatComposer(composer) => composer.required_height(width),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        match self {
            BottomPaneView::ChatComposer(composer) => composer.render(area, buf),
        }
    }
}

/// The main chat input composer.
#[derive(Default)]
pub struct ChatComposer {
    input: String,
}

impl ChatComposer {
    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut String {
        &mut self.input
    }
}

impl Renderable for ChatComposer {
    fn required_height(&self, _width: u16) -> u16 {
        // Fixed height for now.
        3
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Simple rendering: draw a border and the input text.
        use ratatui::{
            style::Style,
            widgets::{Block, Borders, Paragraph, Widget},
        };
        let paragraph = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"))
            .style(Style::default());
        paragraph.render(area, buf);
    }
}
