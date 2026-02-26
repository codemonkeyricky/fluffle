use ratatui::{buffer::Buffer, layout::Rect};

/// A UI component that can compute its required height and render itself.
pub trait Renderable: Send + Sync {
    /// Return the height (number of terminal rows) needed to display this component
    /// when rendered with the given `width`.
    fn required_height(&self, width: u16) -> u16;

    /// Draw the component into `buf` within the bounding rectangle `area`.
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}
