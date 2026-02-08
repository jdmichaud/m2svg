//! Flowchart ASCII rendering

use super::canvas::canvas_to_string;
use super::draw::draw_graph;
use super::grid::create_mapping;
use super::types::{AsciiConfig, AsciiEdge, AsciiGraph, AsciiNode, AsciiSubgraph};
use crate::types::{MermaidGraph, MermaidSubgraph};

/// Convert MermaidGraph to AsciiGraph
fn convert_to_ascii_graph(parsed: &MermaidGraph, config: &AsciiConfig) -> AsciiGraph {
    let mut graph = AsciiGraph::new(config.clone());

    // Build node list preserving insertion order from parser
    for (index, id) in parsed.node_order.iter().enumerate() {
        if let Some(m_node) = parsed.nodes.get(id) {
            let ascii_node = AsciiNode::new(id.to_string(), m_node.label.clone(), index);
            graph.nodes.push(ascii_node);
        }
    }

    // Create a mapping from node ID to index
    let id_to_idx: std::collections::HashMap<&str, usize> = graph
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

    // Convert subgraphs recursively
    for m_sg in &parsed.subgraphs {
        convert_subgraph(m_sg, None, &id_to_idx, &mut graph.subgraphs);
    }

    // Deduplicate subgraph node membership - a node belongs only to the first
    // subgraph where it was defined
    deduplicate_subgraph_nodes(&parsed.subgraphs, &mut graph.subgraphs);

    graph
}

fn convert_subgraph(
    m_sg: &MermaidSubgraph,
    parent_idx: Option<usize>,
    id_to_idx: &std::collections::HashMap<&str, usize>,
    all_subgraphs: &mut Vec<AsciiSubgraph>,
) -> usize {
    let mut sg = AsciiSubgraph::new(m_sg.label.clone());
    sg.parent_idx = parent_idx;

    // Resolve node references
    for node_id in &m_sg.node_ids {
        if let Some(&idx) = id_to_idx.get(node_id.as_str()) {
            sg.node_indices.push(idx);
        }
    }

    let current_idx = all_subgraphs.len();
    all_subgraphs.push(sg);

    // Recurse into children
    for child_m_sg in &m_sg.children {
        let child_idx = convert_subgraph(child_m_sg, Some(current_idx), id_to_idx, all_subgraphs);
        all_subgraphs[current_idx].children_idx.push(child_idx);

        // Child nodes are also part of parent subgraphs
        let child_nodes: Vec<usize> = all_subgraphs[child_idx].node_indices.clone();
        for child_node in child_nodes {
            if !all_subgraphs[current_idx]
                .node_indices
                .contains(&child_node)
            {
                all_subgraphs[current_idx].node_indices.push(child_node);
            }
        }
    }

    current_idx
}

/// Deduplicate subgraph node membership.
/// A node belongs only to the first subgraph where it was defined.
/// When a node is referenced in multiple subgraphs, it should be removed
/// from all but the first one (while keeping it in ancestor subgraphs).
fn deduplicate_subgraph_nodes(
    mermaid_subgraphs: &[MermaidSubgraph],
    ascii_subgraphs: &mut [AsciiSubgraph],
) {
    if ascii_subgraphs.is_empty() || mermaid_subgraphs.is_empty() {
        return;
    }

    // Build a list of which node belongs to which subgraph (first claim wins)
    let mut node_owner: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

    // Build a mapping from ascii_subgraph index to mermaid_subgraph
    // We need to process children before parents when claiming nodes
    fn claim_nodes_recursive(
        m_sg: &MermaidSubgraph,
        sg_idx: &mut usize,
        ascii_subgraphs: &[AsciiSubgraph],
        node_owner: &mut std::collections::HashMap<usize, usize>,
    ) {
        let current_sg_idx = *sg_idx;
        *sg_idx += 1;

        // Recurse into children FIRST (so children claim before parents)
        for child in &m_sg.children {
            claim_nodes_recursive(child, sg_idx, ascii_subgraphs, node_owner);
        }

        // Claim unclaimed nodes in this subgraph
        if current_sg_idx < ascii_subgraphs.len() {
            for &node_idx in &ascii_subgraphs[current_sg_idx].node_indices {
                node_owner.entry(node_idx).or_insert(current_sg_idx);
            }
        }
    }

    let mut sg_idx = 0usize;
    for m_sg in mermaid_subgraphs {
        claim_nodes_recursive(m_sg, &mut sg_idx, ascii_subgraphs, &mut node_owner);
    }

    // Now filter each subgraph's nodes - keep only nodes that belong to this subgraph
    // or one of its descendants
    for sg_idx in 0..ascii_subgraphs.len() {
        let original_nodes = ascii_subgraphs[sg_idx].node_indices.clone();
        ascii_subgraphs[sg_idx].node_indices = original_nodes
            .into_iter()
            .filter(|&node_idx| {
                if let Some(&owner_idx) = node_owner.get(&node_idx) {
                    // Keep if this subgraph is the owner or an ancestor of the owner
                    is_ancestor_or_self(ascii_subgraphs, sg_idx, owner_idx)
                } else {
                    true // not claimed by anyone - keep
                }
            })
            .collect();
    }
}

/// Check if `candidate` is the same as or an ancestor of `target`.
fn is_ancestor_or_self(subgraphs: &[AsciiSubgraph], candidate: usize, target: usize) -> bool {
    let mut current = Some(target);
    while let Some(idx) = current {
        if idx == candidate {
            return true;
        }
        current = subgraphs[idx].parent_idx;
    }
    false
}

/// Calculate subgraph bounding boxes based on node drawing coordinates
fn calculate_subgraph_bounds(graph: &mut AsciiGraph) {
    // Process in reverse order to handle nested subgraphs (children before parents)
    let sg_count = graph.subgraphs.len();
    for sg_idx in (0..sg_count).rev() {
        if graph.subgraphs[sg_idx].node_indices.is_empty() {
            continue;
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        // Include children's bounding boxes first
        for &child_idx in &graph.subgraphs[sg_idx].children_idx.clone() {
            if graph.subgraphs[child_idx].node_indices.is_empty() {
                continue;
            }
            let child = &graph.subgraphs[child_idx];
            min_x = min_x.min(child.min_x);
            min_y = min_y.min(child.min_y);
            max_x = max_x.max(child.max_x);
            max_y = max_y.max(child.max_y);
        }

        // Include node positions using their actual drawing coordinates
        for &node_idx in &graph.subgraphs[sg_idx].node_indices {
            let node = &graph.nodes[node_idx];
            if let Some(dc) = node.drawing_coord {
                // Get the node's drawn box size
                let box_width = if let Some(ref drawing) = node.drawing {
                    drawing.len() as i32 - 1
                } else {
                    let label_len = node.display_label.len() as i32;
                    label_len + 4 // border + padding
                };
                let box_height = if let Some(ref drawing) = node.drawing {
                    if drawing.is_empty() {
                        4
                    } else {
                        drawing[0].len() as i32 - 1
                    }
                } else {
                    4
                };

                let node_min_x = dc.x;
                let node_min_y = dc.y;
                let node_max_x = dc.x + box_width;
                let node_max_y = dc.y + box_height;

                min_x = min_x.min(node_min_x);
                min_y = min_y.min(node_min_y);
                max_x = max_x.max(node_max_x);
                max_y = max_y.max(node_max_y);
            }
        }

        if min_x != i32::MAX {
            // Add padding for subgraph border and label
            let subgraph_padding = 2;
            let subgraph_label_space = 2; // Space for the label at top

            graph.subgraphs[sg_idx].min_x = min_x - subgraph_padding;
            graph.subgraphs[sg_idx].min_y = min_y - subgraph_padding - subgraph_label_space;
            graph.subgraphs[sg_idx].max_x = max_x + subgraph_padding;
            graph.subgraphs[sg_idx].max_y = max_y + subgraph_padding;
        }
    }
}

/// Offset all drawing coordinates so subgraph borders don't go negative
fn offset_drawing_for_subgraphs(graph: &mut AsciiGraph) {
    if graph.subgraphs.is_empty() {
        return;
    }

    let mut min_x = 0;
    let mut min_y = 0;

    for sg in &graph.subgraphs {
        if sg.node_indices.is_empty() {
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

    graph.offset_x = offset_x;
    graph.offset_y = offset_y;

    // Offset subgraph bounds
    for sg in &mut graph.subgraphs {
        sg.min_x += offset_x;
        sg.min_y += offset_y;
        sg.max_x += offset_x;
        sg.max_y += offset_y;
    }

    // Offset node drawing coordinates
    for node in &mut graph.nodes {
        if let Some(ref mut dc) = node.drawing_coord {
            dc.x += offset_x;
            dc.y += offset_y;
        }
    }
}

/// Render a flowchart to ASCII
pub fn render_flowchart_ascii(parsed: &MermaidGraph, config: &AsciiConfig) -> String {
    if parsed.nodes.is_empty() {
        return String::new();
    }

    let mut graph = convert_to_ascii_graph(parsed, config);

    create_mapping(&mut graph);
    calculate_subgraph_bounds(&mut graph);
    offset_drawing_for_subgraphs(&mut graph);
    draw_graph(&mut graph);

    canvas_to_string(&graph.canvas)
}
