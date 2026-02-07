//! ASCII rendering module

pub mod canvas;
pub mod types;
pub mod grid;
pub mod draw;
pub mod pathfinder;
pub mod flowchart;
pub mod sequence;
pub mod class_diagram;
pub mod er_diagram;
pub mod gitgraph;

use crate::parser;
use crate::types::DiagramType;
use crate::AsciiRenderOptions;
use types::AsciiConfig;

/// Parse configuration from input text (lines like paddingX=2, paddingY=1)
fn parse_config_from_text(text: &str, base_opts: AsciiRenderOptions) -> AsciiRenderOptions {
    let mut opts = base_opts;
    
    for line in text.lines() {
        let line = line.trim().to_lowercase();
        if line.starts_with("paddingx=") {
            if let Some(val) = line.strip_prefix("paddingx=") {
                if let Ok(n) = val.parse::<usize>() {
                    opts.padding_x = n;
                }
            }
        } else if line.starts_with("paddingy=") {
            if let Some(val) = line.strip_prefix("paddingy=") {
                if let Ok(n) = val.parse::<usize>() {
                    opts.padding_y = n;
                }
            }
        }
    }
    
    opts
}

/// Render Mermaid diagram text to an ASCII/Unicode string.
///
/// Synchronous — no async layout engine needed.
/// Auto-detects diagram type from the header line and dispatches to
/// the appropriate renderer.
pub fn render_mermaid_ascii(text: &str, options: Option<AsciiRenderOptions>) -> Result<String, String> {
    let base_opts = options.unwrap_or_default();
    // Parse any config lines from the input
    let opts = parse_config_from_text(text, base_opts);
    
    let config = AsciiConfig {
        use_ascii: opts.use_ascii,
        padding_x: opts.padding_x,
        padding_y: opts.padding_y,
        box_border_padding: opts.box_border_padding,
        graph_direction: types::GraphDirection::TD,
    };
    
    let diagram = parser::parse_mermaid(text)?;
    
    match diagram.diagram {
        DiagramType::Flowchart(graph) => {
            let mut config = config;
            if graph.direction == crate::types::Direction::LR 
                || graph.direction == crate::types::Direction::RL {
                config.graph_direction = types::GraphDirection::LR;
            } else {
                config.graph_direction = types::GraphDirection::TD;
            }
            
            let result = flowchart::render_flowchart_ascii(&graph, &config);
            
            // BT: flip the finished canvas vertically
            if graph.direction == crate::types::Direction::BT {
                Ok(canvas::flip_canvas_vertically(&result))
            } else {
                Ok(result)
            }
        }
        DiagramType::Sequence(diagram) => {
            sequence::render_sequence_ascii(&diagram, &config)
        }
        DiagramType::Class(diagram) => {
            class_diagram::render_class_ascii(&diagram, &config)
        }
        DiagramType::Er(diagram) => {
            er_diagram::render_er_ascii(&diagram, &config)
        }
        DiagramType::GitGraph(graph) => {
            Ok(gitgraph::render_gitgraph(&graph, config.use_ascii))
        }
    }
}
