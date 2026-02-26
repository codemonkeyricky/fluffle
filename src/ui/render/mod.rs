//! Rendering primitives for flexible UI layout.
//!
//! This module provides the `Renderable` trait and container types (`ColumnRenderable`,
//! `FlexRenderable`) that allow widgets to declare their height requirements and
//! render themselves within a fixed‑width rectangle.

mod column;
mod flex;
mod renderable;
mod utils;

pub use column::ColumnRenderable;
pub use flex::{FlexItem, FlexRenderable};
pub use renderable::Renderable;
pub use utils::{prefix_lines, simple_renderable, word_wrap_lines};
