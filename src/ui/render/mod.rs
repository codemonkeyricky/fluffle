//! Rendering primitives for flexible UI layout.
//!
//! This module provides the `Renderable` trait that allows widgets to declare their height requirements and
//! render themselves within a fixed‑width rectangle.

mod renderable;

pub use renderable::Renderable;
