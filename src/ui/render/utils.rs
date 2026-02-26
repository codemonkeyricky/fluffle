use ratatui::{buffer::Buffer, layout::Rect, text::Line};

use super::Renderable;

/// Add a prefix to each line of text.
pub fn prefix_lines(prefix: &str, lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| format!("{}{}", prefix, line))
        .collect()
}

/// Wrap a line of text to fit within the given width.
/// Returns a vector of lines, each not exceeding `width`.
pub fn word_wrap_lines(line: &Line, width: u16) -> Vec<Line<'static>> {
    // Simple implementation: split on spaces.
    // TODO: Use textwrap crate for better handling.
    let text = line.to_string();
    let mut result = Vec::new();
    let mut current = String::new();
    let mut current_len = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count() as u16;
        if current_len + word_len + if current.is_empty() { 0 } else { 1 } > width {
            if !current.is_empty() {
                result.push(Line::from(current));
                current = String::new();
                current_len = 0;
            }
            // If a single word exceeds width, split the word.
            if word_len > width {
                let mut chars = word.chars();
                while current_len < width {
                    if let Some(c) = chars.next() {
                        current.push(c);
                        current_len += 1;
                    } else {
                        break;
                    }
                }
                result.push(Line::from(current));
                current = String::from_iter(chars);
                current_len = current.chars().count() as u16;
            } else {
                current = word.to_string();
                current_len = word_len;
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
                current_len += 1;
            }
            current.push_str(word);
            current_len += word_len;
        }
    }
    if !current.is_empty() {
        result.push(Line::from(current));
    }
    result
}

/// Create a simple single‑line renderable from plain text.
pub fn simple_renderable(text: String) -> Box<dyn Renderable> {
    Box::new(SimpleTextRenderable(text))
}

struct SimpleTextRenderable(String);

impl Renderable for SimpleTextRenderable {
    fn required_height(&self, width: u16) -> u16 {
        // Single line, but may wrap if longer than width.
        let line = Line::from(self.0.as_str());
        let wrapped = word_wrap_lines(&line, width);
        wrapped.len() as u16
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let line = Line::from(self.0.as_str());
        let wrapped = word_wrap_lines(&line, area.width);
        let mut y = area.y;
        for line in wrapped {
            if y >= area.y + area.height {
                break;
            }
            buf.set_line(area.x, y, &line, area.width);
            y += 1;
        }
    }
}
