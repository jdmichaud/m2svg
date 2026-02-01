//! m2svg - Convert Mermaid diagrams to ASCII/Unicode art and SVG
//!
//! This library provides functionality to parse Mermaid diagram syntax and render
//! it as ASCII or Unicode box-drawing art, or as SVG.
//!
//! # Example
//!
//! ```rust
//! use m2svg::{render, render_to_svg};
//!
//! let ascii = render("graph LR\n  A --> B", false).unwrap();
//! println!("{}", ascii);
//!
//! let svg = render_to_svg("graph LR\n  A --> B").unwrap();
//! println!("{}", svg);
//! ```
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

/// Render a Mermaid diagram to ASCII/Unicode text.
///
/// # Arguments
/// * `input` - Mermaid diagram text
/// * `use_ascii` - If true, use plain ASCII (+,-,|,>). If false, use Unicode box-drawing (┌,─,│,►)
///
/// # Example
/// ```rust
/// let output = m2svg::render("graph LR\n  A --> B", false).unwrap();
/// ```
pub fn render(input: &str, use_ascii: bool) -> Result<String, String> {
    let opts = AsciiRenderOptions {
        use_ascii,
        ..Default::default()
    };
    render_mermaid_ascii(input, Some(opts))
}

/// Render a Mermaid diagram to SVG text.
///
/// # Arguments
/// * `input` - Mermaid diagram text
///
/// # Example
/// ```rust
/// let svg = m2svg::render_to_svg("graph LR\n  A --> B").unwrap();
/// ```
pub fn render_to_svg(input: &str) -> Result<String, String> {
    let parsed = parse_mermaid(input)?;
    let colors = svg::DiagramColors::default();
    let font = "Inter";
    let transparent = false;
    
    match parsed {
        DiagramType::Flowchart(graph) => {
            Ok(svg::render_mermaid_to_svg(&graph, &colors, font, transparent))
        }
        DiagramType::Sequence(diagram) => {
            Ok(svg::render_sequence_svg(&diagram, &colors, font, transparent))
        }
        DiagramType::Class(diagram) => {
            Ok(svg::render_class_svg(&diagram, &colors, font, transparent))
        }
        DiagramType::Er(diagram) => {
            Ok(svg::render_er_svg(&diagram, &colors, font, transparent))
        }
        DiagramType::GitGraph(graph) => {
            Ok(svg::render_gitgraph_svg(&graph, &colors, font, transparent))
        }
    }
}

/// Configuration options for ASCII rendering
#[derive(Debug, Clone)]
pub struct AsciiRenderOptions {
    /// true = ASCII chars (+,-,|,>), false = Unicode box-drawing (┌,─,│,►). Default: true
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
