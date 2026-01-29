//! Mermaid ASCII - Convert Mermaid diagrams to ASCII/Unicode art and SVG
//!
//! This library provides functionality to parse Mermaid diagram syntax and render
//! it as ASCII or Unicode box-drawing art, or as SVG.
//!
//! # Supported Diagram Types
//!
//! - Flowcharts (graph TD / flowchart LR)
//! - State diagrams (stateDiagram-v2)
//! - Sequence diagrams (sequenceDiagram)
//! - Class diagrams (classDiagram)
//! - ER diagrams (erDiagram)

pub mod types;
pub mod parser;
pub mod ascii;
pub mod svg;

pub use ascii::render_mermaid_ascii;
pub use types::*;
pub use parser::parse_mermaid;

/// Configuration options for ASCII rendering
#[derive(Debug, Clone)]
pub struct AsciiRenderOptions {
    /// true = ASCII chars (+,-,|,>), false = Unicode box-drawing (┌,─,│,►). Default: false
    pub use_ascii: bool,
    /// Horizontal spacing between nodes. Default: 5
    pub padding_x: usize,
    /// Vertical spacing between nodes. Default: 5
    pub padding_y: usize,
    /// Padding inside node boxes. Default: 1
    pub box_border_padding: usize,
}

impl Default for AsciiRenderOptions {
    fn default() -> Self {
        Self {
            use_ascii: true,
            padding_x: 5,
            padding_y: 5,
            box_border_padding: 1,
        }
    }
}
