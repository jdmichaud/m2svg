//! SVG renderer - converts positioned graphs into SVG strings.
//!
//! Pure string building, no DOM manipulation.
//! Renders back-to-front: groups → edges → edge labels → nodes → node labels.

mod types;
mod renderer;
mod theme;
mod styles;

pub use types::*;
pub use renderer::render_svg;
pub use theme::DiagramColors;
