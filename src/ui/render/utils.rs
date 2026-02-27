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

/// Create a simple renderable from plain text, preserving newlines.
pub fn simple_renderable(text: String) -> Box<dyn Renderable> {
    Box::new(SimpleTextRenderable::new(text))
}

struct SimpleTextRenderable {
    lines: Vec<String>,
}

impl SimpleTextRenderable {
    fn new(text: String) -> Self {
        // Handle empty string as zero lines
        if text.is_empty() {
            return Self { lines: Vec::new() };
        }
        // Split on newline, preserving empty lines
        let lines = text
            .split('\n')
            .map(|s| s.trim_end_matches('\r').to_string())
            .collect();
        Self { lines }
    }
}

impl Renderable for SimpleTextRenderable {
    fn required_height(&self, width: u16) -> u16 {
        let mut total = 0;
        for line in &self.lines {
            if line.trim().is_empty() {
                // Blank line occupies one row
                total += 1;
            } else {
                let rat_line = Line::from(line.as_str());
                let wrapped = word_wrap_lines(&rat_line, width);
                total += wrapped.len() as u16;
            }
        }
        total
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        for line in &self.lines {
            if line.trim().is_empty() {
                // Blank line: just advance y
                if y >= area.y + area.height {
                    return;
                }
                y += 1;
            } else {
                let rat_line = Line::from(line.as_str());
                let wrapped = word_wrap_lines(&rat_line, area.width);
                for wrapped_line in wrapped {
                    if y >= area.y + area.height {
                        return;
                    }
                    buf.set_line(area.x, y, &wrapped_line, area.width);
                    y += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_renderable_newlines() {
        let text = "Line 1\nLine 2\n\nLine 4".to_string();
        let renderable = simple_renderable(text);
        // With width large enough, each non-empty line takes 1 row, empty line takes 1 row
        assert_eq!(renderable.required_height(100), 4);
        // With narrow width, lines may wrap
        let text2 = "A very long line that will wrap when width is small".to_string();
        let renderable2 = simple_renderable(text2);
        // Width 10 should cause wrapping
        let height = renderable2.required_height(10);
        assert!(height >= 5);
    }

    #[test]
    fn test_simple_renderable_empty() {
        let renderable = simple_renderable("".to_string());
        assert_eq!(renderable.required_height(100), 0);
    }

    #[test]
    fn test_simple_renderable_single_line() {
        let renderable = simple_renderable("Hello".to_string());
        assert_eq!(renderable.required_height(100), 1);
    }
}
