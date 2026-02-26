use ratatui::{buffer::Buffer, layout::Rect};

use super::Renderable;

/// A child item within a `FlexRenderable`.
pub enum FlexItem {
    /// A fixed‑width component.
    Fixed(u16, Box<dyn Renderable>),
    /// A component that expands proportionally to its weight.
    Flex(u16, Box<dyn Renderable>),
}

/// A horizontal arrangement of components with flexible width distribution.
pub struct FlexRenderable(Vec<FlexItem>);

impl FlexRenderable {
    /// Create a new empty flex container.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Add a fixed‑width child.
    pub fn push_fixed(mut self, width: u16, child: Box<dyn Renderable>) -> Self {
        self.0.push(FlexItem::Fixed(width, child));
        self
    }

    /// Add a flexible child with the given weight.
    pub fn push_flex(mut self, weight: u16, child: Box<dyn Renderable>) -> Self {
        self.0.push(FlexItem::Flex(weight, child));
        self
    }

    /// Add a child (mutating version).
    pub fn push(&mut self, item: FlexItem) {
        self.0.push(item);
    }
}

impl Renderable for FlexRenderable {
    fn required_height(&self, width: u16) -> u16 {
        let widths = compute_widths(width, &self.0);
        self.0
            .iter()
            .zip(widths)
            .map(|(item, w)| match item {
                FlexItem::Fixed(_, child) => child.required_height(w),
                FlexItem::Flex(_, child) => child.required_height(w),
            })
            .max()
            .unwrap_or(0)
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let widths = compute_widths(area.width, &self.0);
        let mut x = area.x;
        let max_x = area.x + area.width;

        for (item, width) in self.0.iter_mut().zip(widths) {
            if x >= max_x {
                break;
            }
            let child = match item {
                FlexItem::Fixed(_, child) => child,
                FlexItem::Flex(_, child) => child,
            };
            let height = child.required_height(width);
            let rect = Rect::new(x, area.y, width, height);
            child.render(rect, buf);
            x += width;
        }
    }
}

/// Distribute `total_width` among flex items.
/// Returns a vector of widths, one per item.
fn compute_widths(total_width: u16, items: &[FlexItem]) -> Vec<u16> {
    let mut fixed_sum = 0;
    let mut flex_weight_sum = 0;
    for item in items {
        match item {
            FlexItem::Fixed(w, _) => fixed_sum += w,
            FlexItem::Flex(w, _) => flex_weight_sum += w,
        }
    }

    let remaining = total_width.saturating_sub(fixed_sum);
    let flex_unit = if flex_weight_sum > 0 {
        remaining / flex_weight_sum
    } else {
        0
    };
    let mut extra = remaining % flex_weight_sum;

    items
        .iter()
        .map(|item| match item {
            FlexItem::Fixed(w, _) => *w,
            FlexItem::Flex(w, _) => {
                let base = flex_unit * w;
                let give = if extra > 0 {
                    extra -= 1;
                    1
                } else {
                    0
                };
                base + give
            }
        })
        .collect()
}
