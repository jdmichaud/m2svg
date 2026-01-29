//! SVG renderer - converts positioned graphs into SVG strings.
//!
//! Two modes:
//! 1. `render_svg` - from pre-positioned PositionedGraph (for testing against fixtures)
//! 2. `render_mermaid_to_svg` - directly from parsed MermaidGraph using ASCII layout
//!
//! Pure string building, no DOM manipulation.

mod types;
mod renderer;
mod theme;
mod styles;
mod from_ascii;

pub use types::*;
pub use renderer::render_svg;
pub use theme::DiagramColors;
pub use from_ascii::render_mermaid_to_svg;
