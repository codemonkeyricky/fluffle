use ratatui::{buffer::Buffer, layout::Rect};

use super::Renderable;

/// A vertical stack of renderable components.
pub struct ColumnRenderable(Vec<Box<dyn Renderable>>);

impl ColumnRenderable {
    /// Create a new empty column.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Add a child component to the column.
    pub fn push(mut self, child: Box<dyn Renderable>) -> Self {
        self.0.push(child);
        self
    }

    /// Add a child component to the column (mutating version).
    pub fn push_mut(&mut self, child: Box<dyn Renderable>) {
        self.0.push(child);
    }

    /// Returns the number of children.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no children.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Renderable for ColumnRenderable {
    fn required_height(&self, width: u16) -> u16 {
        self.0
            .iter()
            .map(|child| child.required_height(width))
            .sum()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        let width = area.width;
        let max_y = area.y + area.height;

        for child in &mut self.0 {
            let height = child.required_height(width);
            if y + height > max_y {
                // Not enough space to render this child; skip remaining children.
                break;
            }
            let rect = Rect::new(area.x, y, width, height);
            child.render(rect, buf);
            y += height;
        }
    }
}
