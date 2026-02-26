//! Rendering primitives for flexible UI layout.
//!
//! This module provides the `Renderable` trait and container types (`ColumnRenderable`,
//! `FlexRenderable`) that allow widgets to declare their height requirements and
//! render themselves within a fixed‑width rectangle.

mod renderable;
mod column;
mod flex;
mod utils;

pub use renderable::Renderable;
pub use column::ColumnRenderable;
pub use flex::{FlexRenderable, FlexItem};
pub use utils::{prefix_lines, word_wrap_lines, simple_renderable};