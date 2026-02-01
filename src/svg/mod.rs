//! SVG renderer - converts diagrams into SVG strings.
//!
//! Supports all diagram types:
//! - Flowcharts (render_mermaid_to_svg)
//! - Sequence diagrams (render_sequence_svg)
//! - Class diagrams (render_class_svg)
//! - ER diagrams (render_er_svg)
//! - GitGraph (render_gitgraph_svg)
//!
//! Pure string building, no DOM manipulation.

mod types;
mod renderer;
mod theme;
mod styles;
mod from_ascii;
mod sequence;
mod class_diagram;
mod er_diagram;
mod gitgraph;

pub use types::*;
pub use renderer::render_svg;
pub use theme::DiagramColors;
pub use from_ascii::render_mermaid_to_svg;
pub use sequence::render_sequence_svg;
pub use class_diagram::render_class_svg;
pub use er_diagram::render_er_svg;
pub use gitgraph::render_gitgraph_svg;
