//! SVG rendering using the ASCII layout algorithm
//!
//! This takes the same grid-based layout as ASCII and converts to SVG.
//! Much simpler than using a separate layout engine like dagre.

use super::renderer::escape_xml;
use super::theme::{build_style_block, svg_open_tag, DiagramColors};
use crate::ascii::grid::create_mapping;
use crate::ascii::types::{
    AsciiConfig, AsciiEdge, AsciiGraph, AsciiNode, AsciiSubgraph, GraphDirection,
};
use crate::types::{Direction as MermaidDirection, MermaidGraph};
use std::collections::HashMap;

/// Scale factor: how many pixels per ASCII character cell
const CHAR_WIDTH: f64 = 8.0;
const CHAR_HEIGHT: f64 = 16.0;

/// Render a MermaidGraph directly to SVG using the ASCII layout algorithm.
///
/// This is the simple path: parse → ASCII layout → SVG output.
/// No external layout engine needed.
pub fn render_mermaid_to_svg(
    parsed: &MermaidGraph,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    if parsed.nodes.is_empty() {
        return String::new();
    }

    // Create ASCII graph and compute layout
    let config = AsciiConfig {
        use_ascii: false,
        padding_x: 2,
        padding_y: 1,
        box_border_padding: 1,
        graph_direction: match parsed.direction {
            MermaidDirection::LR | MermaidDirection::RL => GraphDirection::LR,
            _ => GraphDirection::TD,
        },
    };

    let mut graph = convert_to_ascii_graph(parsed, &config);
    create_mapping(&mut graph);
    calculate_subgraph_bounds(&mut graph);
    offset_drawing_for_subgraphs(&mut graph);

    // Now convert the positioned ASCII graph to SVG
    ascii_graph_to_svg(&graph, colors, font, transparent)
}

/// Convert MermaidGraph to AsciiGraph (copied from flowchart.rs to avoid circular deps)
fn convert_to_ascii_graph(parsed: &MermaidGraph, config: &AsciiConfig) -> AsciiGraph {
    use crate::types::MermaidSubgraph;

    let mut graph = AsciiGraph::new(config.clone());

    // Build node list preserving insertion order
    for (index, id) in parsed.node_order.iter().enumerate() {
        if let Some(m_node) = parsed.nodes.get(id) {
            let ascii_node = AsciiNode::new(id.to_string(), m_node.label.clone(), index);
            graph.nodes.push(ascii_node);
        }
    }

    // Mapping from node ID to index
    let id_to_idx: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.as_str(), i))
        .collect();

    // Build edges
    for m_edge in &parsed.edges {
        if let (Some(&from_idx), Some(&to_idx)) = (
            id_to_idx.get(m_edge.source.as_str()),
            id_to_idx.get(m_edge.target.as_str()),
        ) {
            let edge = AsciiEdge::new(from_idx, to_idx, m_edge.label.clone().unwrap_or_default());
            graph.edges.push(edge);
        }
    }

    // Convert subgraphs
    fn convert_subgraph(
        m_sg: &MermaidSubgraph,
        parent_idx: Option<usize>,
        id_to_idx: &HashMap<&str, usize>,
        all_subgraphs: &mut Vec<AsciiSubgraph>,
    ) -> usize {
        let mut sg = AsciiSubgraph::new(m_sg.label.clone());
        sg.parent_idx = parent_idx;

        for node_id in &m_sg.node_ids {
            if let Some(&idx) = id_to_idx.get(node_id.as_str()) {
                sg.node_indices.push(idx);
            }
        }

        let current_idx = all_subgraphs.len();
        all_subgraphs.push(sg);

        for child in &m_sg.children {
            let child_idx = convert_subgraph(child, Some(current_idx), id_to_idx, all_subgraphs);
            all_subgraphs[current_idx].children_idx.push(child_idx);
        }

        current_idx
    }

    for m_sg in &parsed.subgraphs {
        convert_subgraph(m_sg, None, &id_to_idx, &mut graph.subgraphs);
    }

    graph
}

/// Calculate subgraph bounds (simplified from flowchart.rs)
fn calculate_subgraph_bounds(graph: &mut AsciiGraph) {
    for sg_idx in 0..graph.subgraphs.len() {
        let all_node_indices = collect_all_nodes(sg_idx, &graph.subgraphs);

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for node_idx in all_node_indices {
            let node = &graph.nodes[node_idx];
            if let Some(dc) = node.drawing_coord {
                let box_width = node.display_label.len() as i32 + 4;
                let box_height = 4;

                min_x = min_x.min(dc.x);
                min_y = min_y.min(dc.y);
                max_x = max_x.max(dc.x + box_width);
                max_y = max_y.max(dc.y + box_height);
            }
        }

        if min_x != i32::MAX {
            let padding = 2;
            let label_space = 2;
            graph.subgraphs[sg_idx].min_x = min_x - padding;
            graph.subgraphs[sg_idx].min_y = min_y - padding - label_space;
            graph.subgraphs[sg_idx].max_x = max_x + padding;
            graph.subgraphs[sg_idx].max_y = max_y + padding;
        }
    }
}

fn collect_all_nodes(sg_idx: usize, subgraphs: &[AsciiSubgraph]) -> Vec<usize> {
    let mut nodes = subgraphs[sg_idx].node_indices.clone();
    for &child_idx in &subgraphs[sg_idx].children_idx {
        nodes.extend(collect_all_nodes(child_idx, subgraphs));
    }
    nodes
}

fn offset_drawing_for_subgraphs(graph: &mut AsciiGraph) {
    if graph.subgraphs.is_empty() {
        return;
    }

    let mut min_x = 0;
    let mut min_y = 0;

    for sg in &graph.subgraphs {
        if sg.node_indices.is_empty() && sg.children_idx.is_empty() {
            continue;
        }
        min_x = min_x.min(sg.min_x);
        min_y = min_y.min(sg.min_y);
    }

    let offset_x = -min_x;
    let offset_y = -min_y;

    if offset_x == 0 && offset_y == 0 {
        return;
    }

    for sg in &mut graph.subgraphs {
        sg.min_x += offset_x;
        sg.min_y += offset_y;
        sg.max_x += offset_x;
        sg.max_y += offset_y;
    }

    for node in &mut graph.nodes {
        if let Some(ref mut dc) = node.drawing_coord {
            dc.x += offset_x;
            dc.y += offset_y;
        }
    }
}

/// Convert positioned ASCII graph to SVG string
fn ascii_graph_to_svg(
    graph: &AsciiGraph,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    // Calculate SVG dimensions from ASCII character grid
    let (canvas_width, canvas_height) = calculate_canvas_size(graph);
    let svg_width = (canvas_width as f64) * CHAR_WIDTH + 40.0; // padding
    let svg_height = (canvas_height as f64) * CHAR_HEIGHT + 40.0;

    let mut parts: Vec<String> = Vec::new();

    // SVG header
    parts.push(svg_open_tag(svg_width, svg_height, colors, transparent));
    parts.push(build_style_block(font));
    parts.push(arrow_defs());

    // 1. Render subgraphs (backgrounds)
    for sg in &graph.subgraphs {
        if sg.min_x == 0 && sg.max_x == 0 {
            continue; // Empty subgraph
        }
        parts.push(render_subgraph_svg(sg));
    }

    // 2. Render edges
    for edge in &graph.edges {
        let from_node = &graph.nodes[edge.from_idx];
        let to_node = &graph.nodes[edge.to_idx];
        if let (Some(from_dc), Some(to_dc)) = (from_node.drawing_coord, to_node.drawing_coord) {
            parts.push(render_edge_svg(
                from_dc,
                to_dc,
                from_node,
                to_node,
                &edge.text,
                &graph.config,
            ));
        }
    }

    // 3. Render nodes
    for node in &graph.nodes {
        if let Some(dc) = node.drawing_coord {
            parts.push(render_node_svg(dc, &node.display_label));
        }
    }

    parts.push("</svg>".to_string());
    parts
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn calculate_canvas_size(graph: &AsciiGraph) -> (i32, i32) {
    let mut max_x = 0i32;
    let mut max_y = 0i32;

    for node in &graph.nodes {
        if let Some(dc) = node.drawing_coord {
            let box_width = node.display_label.len() as i32 + 4;
            max_x = max_x.max(dc.x + box_width);
            max_y = max_y.max(dc.y + 5);
        }
    }

    for sg in &graph.subgraphs {
        max_x = max_x.max(sg.max_x);
        max_y = max_y.max(sg.max_y);
    }

    (max_x, max_y)
}

fn arrow_defs() -> String {
    r#"<defs>
  <marker id="arrowhead" markerWidth="8" markerHeight="4.8" refX="8" refY="2.4" orient="auto">
    <polygon points="0 0, 8 2.4, 0 4.8" fill="var(--_arrow)" />
  </marker>
</defs>"#
        .to_string()
}

fn render_subgraph_svg(sg: &AsciiSubgraph) -> String {
    let x = (sg.min_x as f64) * CHAR_WIDTH + 20.0;
    let y = (sg.min_y as f64) * CHAR_HEIGHT + 20.0;
    let width = ((sg.max_x - sg.min_x) as f64) * CHAR_WIDTH;
    let height = ((sg.max_y - sg.min_y) as f64) * CHAR_HEIGHT;
    let header_height = 28.0;

    format!(
        r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="0" ry="0" fill="var(--_group-fill)" stroke="var(--_node-stroke)" stroke-width="1" />
<rect x="{x}" y="{y}" width="{width}" height="{header_height}" rx="0" ry="0" fill="var(--_group-hdr)" stroke="var(--_node-stroke)" stroke-width="1" />
<text x="{label_x}" y="{label_y}" dy="0.35em" font-size="12" font-weight="600" fill="var(--_text-sec)">{label}</text>"#,
        x = x,
        y = y,
        width = width,
        height = height,
        header_height = header_height,
        label_x = x + 12.0,
        label_y = y + header_height / 2.0,
        label = escape_xml(&sg.name),
    )
}

fn render_node_svg(dc: crate::ascii::types::DrawingCoord, label: &str) -> String {
    let x = (dc.x as f64) * CHAR_WIDTH + 20.0;
    let y = (dc.y as f64) * CHAR_HEIGHT + 20.0;
    let width = (label.len() as f64 + 4.0) * CHAR_WIDTH;
    let height = 4.0 * CHAR_HEIGHT;
    let text_x = x + width / 2.0;
    let text_y = y + height / 2.0;

    format!(
        r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="0" ry="0" fill="var(--_node-fill)" stroke="var(--_node-stroke)" stroke-width="0.75" />
<text x="{text_x}" y="{text_y}" text-anchor="middle" dy="0.35em" font-size="13" font-weight="500" fill="var(--_text)">{label}</text>"#,
        x = x,
        y = y,
        width = width,
        height = height,
        text_x = text_x,
        text_y = text_y,
        label = escape_xml(label),
    )
}

fn render_edge_svg(
    from_dc: crate::ascii::types::DrawingCoord,
    to_dc: crate::ascii::types::DrawingCoord,
    from_node: &AsciiNode,
    to_node: &AsciiNode,
    label: &str,
    config: &AsciiConfig,
) -> String {
    // Calculate node centers and sizes
    let from_w = (from_node.display_label.len() as f64 + 4.0) * CHAR_WIDTH;
    let from_h = 4.0 * CHAR_HEIGHT;
    let to_w = (to_node.display_label.len() as f64 + 4.0) * CHAR_WIDTH;
    let _to_h = 4.0 * CHAR_HEIGHT;

    let from_center_x = (from_dc.x as f64) * CHAR_WIDTH + 20.0 + from_w / 2.0;
    let from_center_y = (from_dc.y as f64) * CHAR_HEIGHT + 20.0 + from_h / 2.0;
    let to_center_x = (to_dc.x as f64) * CHAR_WIDTH + 20.0 + to_w / 2.0;
    let to_center_y = (to_dc.y as f64) * CHAR_HEIGHT + 20.0 + _to_h / 2.0;

    // Determine connection points based on graph direction
    let (x1, y1, x2, y2) = match config.graph_direction {
        GraphDirection::LR => {
            // Connect right side of from to left side of to
            let x1 = (from_dc.x as f64) * CHAR_WIDTH + 20.0 + from_w;
            let y1 = from_center_y;
            let x2 = (to_dc.x as f64) * CHAR_WIDTH + 20.0;
            let y2 = to_center_y;
            (x1, y1, x2, y2)
        }
        GraphDirection::TD => {
            // Connect bottom of from to top of to
            let x1 = from_center_x;
            let y1 = (from_dc.y as f64) * CHAR_HEIGHT + 20.0 + from_h;
            let x2 = to_center_x;
            let y2 = (to_dc.y as f64) * CHAR_HEIGHT + 20.0;
            (x1, y1, x2, y2)
        }
    };

    let mut svg = format!(
        r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="var(--_line)" stroke-width="0.75" marker-end="url(#arrowhead)" />"#,
        x1 = x1,
        y1 = y1,
        x2 = x2,
        y2 = y2,
    );

    // Add label if present
    if !label.is_empty() {
        let label_x = (x1 + x2) / 2.0;
        let label_y = (y1 + y2) / 2.0 - 8.0;
        svg.push_str(&format!(
            r#"
<text x="{}" y="{}" text-anchor="middle" dy="0.35em" font-size="11" fill="var(--_text-sec)">{}</text>"#,
            label_x, label_y, escape_xml(label),
        ));
    }

    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_mermaid;
    use crate::types::DiagramType;

    #[test]
    fn test_simple_graph_to_svg() {
        let input = "graph LR\n  A --> B";
        let parsed = parse_mermaid(input).unwrap();
        let graph = match parsed.diagram {
            DiagramType::Flowchart(g) => g,
            _ => panic!("Expected flowchart"),
        };
        let colors = DiagramColors::default();
        let svg = render_mermaid_to_svg(&graph, &colors, "Inter", false);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains(">A<"));
        assert!(svg.contains(">B<"));
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_td_graph_to_svg() {
        let input = "graph TD\n  Start --> End";
        let parsed = parse_mermaid(input).unwrap();
        let graph = match parsed.diagram {
            DiagramType::Flowchart(g) => g,
            _ => panic!("Expected flowchart"),
        };
        let colors = DiagramColors::default();
        let svg = render_mermaid_to_svg(&graph, &colors, "Inter", false);

        assert!(svg.contains(">Start<"));
        assert!(svg.contains(">End<"));
    }
}
