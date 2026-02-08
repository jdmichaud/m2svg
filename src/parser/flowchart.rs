//! Flowchart and state diagram parser

use crate::types::{
    Direction, EdgeStyle, MermaidEdge, MermaidGraph, MermaidNode, MermaidSubgraph, NodeShape,
};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    static ref RE_HEADER: Regex = Regex::new(r"(?i)^(?:graph|flowchart)\s+(TD|TB|LR|BT|RL)\s*$").unwrap();
    static ref RE_CLASSDEF: Regex = Regex::new(r"^classDef\s+(\w+)\s+(.+)$").unwrap();
    static ref RE_CLASS: Regex = Regex::new(r"^class\s+([\w,-]+)\s+(\w+)$").unwrap();
    static ref RE_STYLE: Regex = Regex::new(r"^style\s+([\w,-]+)\s+(.+)$").unwrap();
    static ref RE_DIRECTION: Regex = Regex::new(r"(?i)^direction\s+(TD|TB|LR|BT|RL)\s*$").unwrap();
    static ref RE_SUBGRAPH: Regex = Regex::new(r"^subgraph\s+(.+)$").unwrap();
    static ref RE_SUBGRAPH_BRACKET: Regex = Regex::new(r"^([\w-]+)\s*\[(.+)\]$").unwrap();
    static ref RE_STATE_BLOCK: Regex = Regex::new(r#"^state\s+(?:"([^"]+)"\s+as\s+)?(\w+)\s*\{$"#).unwrap();
    static ref RE_STATE_LABEL: Regex = Regex::new(r#"^state\s+"([^"]+)"\s+as\s+(\w+)\s*$"#).unwrap();
    static ref RE_STATE_TRANS: Regex = Regex::new(r"^(\[\*\]|[\w-]+)\s*(-->)\s*(\[\*\]|[\w-]+)(?:\s*:\s*(.+))?$").unwrap();
    static ref RE_NODE_LABEL: Regex = Regex::new(r"^([\w-]+)\s*:\s*(.+)$").unwrap();
    static ref RE_ARROW: Regex = Regex::new(r"^(<)?(-->|-.->|==>|---|-\.-|===)(?:\|([^|]*)\|)?").unwrap();
    static ref RE_CLASS_SUFFIX: Regex = Regex::new(r"^:::([\w][\w-]*)").unwrap();
    static ref RE_BARE_ID: Regex = Regex::new(r"^([\w-]+)").unwrap();

    // Node shape patterns (in order of specificity - triple, double, single delimiters)
    static ref RE_NODE_DOUBLE_CIRCLE: Regex = Regex::new(r"^([\w-]+)\(\(\((.+?)\)\)\)").unwrap();
    static ref RE_NODE_STADIUM: Regex = Regex::new(r"^([\w-]+)\(\[(.+?)\]\)").unwrap();
    static ref RE_NODE_CIRCLE: Regex = Regex::new(r"^([\w-]+)\(\((.+?)\)\)").unwrap();
    static ref RE_NODE_SUBROUTINE: Regex = Regex::new(r"^([\w-]+)\[\[(.+?)\]\]").unwrap();
    static ref RE_NODE_CYLINDER: Regex = Regex::new(r"^([\w-]+)\[\((.+?)\)\]").unwrap();
    static ref RE_NODE_TRAPEZOID: Regex = Regex::new(r"^([\w-]+)\[/(.+?)\\\]").unwrap();
    static ref RE_NODE_TRAPEZOID_ALT: Regex = Regex::new(r"^([\w-]+)\[\\(.+?)/\]").unwrap();
    static ref RE_NODE_ASYMMETRIC: Regex = Regex::new(r"^([\w-]+)>(.+?)\]").unwrap();
    static ref RE_NODE_HEXAGON: Regex = Regex::new(r"^([\w-]+)\{\{(.+?)\}\}").unwrap();
    static ref RE_NODE_RECTANGLE: Regex = Regex::new(r"^([\w-]+)\[(.+?)\]").unwrap();
    static ref RE_NODE_ROUNDED: Regex = Regex::new(r"^([\w-]+)\((.+?)\)").unwrap();
    static ref RE_NODE_DIAMOND: Regex = Regex::new(r"^([\w-]+)\{(.+?)\}").unwrap();
}

/// Parse a flowchart/graph diagram
pub fn parse_flowchart(lines: &[&str]) -> Result<MermaidGraph, String> {
    let header = lines[0];

    // Match "graph TD" or "flowchart LR" etc
    let caps = RE_HEADER.captures(header).ok_or_else(|| {
        format!(
            "Invalid mermaid header: \"{}\". Expected \"graph TD\", \"flowchart LR\", etc.",
            header
        )
    })?;

    let direction =
        Direction::from_str(&caps[1]).ok_or_else(|| format!("Invalid direction: {}", &caps[1]))?;

    let mut graph = MermaidGraph::new(direction);
    let mut subgraph_stack: Vec<MermaidSubgraph> = Vec::new();

    for line in lines.iter().skip(1) {
        let line = *line;

        // classDef
        if let Some(caps) = RE_CLASSDEF.captures(line) {
            let name = caps[1].to_string();
            let props = parse_style_props(&caps[2]);
            graph.class_defs.insert(name, props);
            continue;
        }

        // class assignment
        if let Some(caps) = RE_CLASS.captures(line) {
            let node_ids: Vec<&str> = caps[1].split(',').map(|s| s.trim()).collect();
            let class_name = caps[2].to_string();
            for id in node_ids {
                graph
                    .class_assignments
                    .insert(id.to_string(), class_name.clone());
            }
            continue;
        }

        // style statement
        if let Some(caps) = RE_STYLE.captures(line) {
            let node_ids: Vec<&str> = caps[1].split(',').map(|s| s.trim()).collect();
            let props = parse_style_props(&caps[2]);
            for id in node_ids {
                let entry = graph.node_styles.entry(id.to_string()).or_default();
                for (k, v) in &props {
                    entry.insert(k.clone(), v.clone());
                }
            }
            continue;
        }

        // direction override inside subgraph
        if let Some(caps) = RE_DIRECTION.captures(line) {
            if let Some(sg) = subgraph_stack.last_mut() {
                sg.direction = Direction::from_str(&caps[1]);
            }
            continue;
        }

        // subgraph start
        if let Some(caps) = RE_SUBGRAPH.captures(line) {
            let rest = caps[1].trim();
            let (id, label) = if let Some(bracket_caps) = RE_SUBGRAPH_BRACKET.captures(rest) {
                (bracket_caps[1].to_string(), bracket_caps[2].to_string())
            } else {
                let id = rest
                    .replace(' ', "_")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                (id, rest.to_string())
            };

            let sg = MermaidSubgraph {
                id,
                label,
                node_ids: Vec::new(),
                children: Vec::new(),
                direction: None,
            };
            subgraph_stack.push(sg);
            continue;
        }

        // subgraph end
        if line == "end" {
            if let Some(completed) = subgraph_stack.pop() {
                if let Some(parent) = subgraph_stack.last_mut() {
                    parent.children.push(completed);
                } else {
                    graph.subgraphs.push(completed);
                }
            }
            continue;
        }

        // Edge/node definitions
        parse_edge_line(line, &mut graph, &mut subgraph_stack);
    }

    Ok(graph)
}

/// Parse a state diagram
pub fn parse_state_diagram(lines: &[&str]) -> Result<MermaidGraph, String> {
    let mut graph = MermaidGraph::new(Direction::TD);
    let mut composite_stack: Vec<MermaidSubgraph> = Vec::new();
    let mut start_count = 0;
    let mut end_count = 0;

    for line in lines.iter().skip(1) {
        let line = *line;

        // direction override
        if let Some(caps) = RE_DIRECTION.captures(line) {
            let dir = Direction::from_str(&caps[1]);
            if let Some(sg) = composite_stack.last_mut() {
                sg.direction = dir;
            } else if let Some(d) = dir {
                graph.direction = d;
            }
            continue;
        }

        // composite state start
        if let Some(caps) = RE_STATE_BLOCK.captures(line) {
            let label = caps
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or(&caps[2])
                .to_string();
            let id = caps[2].to_string();
            let sg = MermaidSubgraph {
                id,
                label,
                node_ids: Vec::new(),
                children: Vec::new(),
                direction: None,
            };
            composite_stack.push(sg);
            continue;
        }

        // composite state end
        if line == "}" {
            if let Some(completed) = composite_stack.pop() {
                if let Some(parent) = composite_stack.last_mut() {
                    parent.children.push(completed);
                } else {
                    graph.subgraphs.push(completed);
                }
            }
            continue;
        }

        // state alias
        if let Some(caps) = RE_STATE_LABEL.captures(line) {
            let label = caps[1].to_string();
            let id = caps[2].to_string();
            register_state_node(
                &mut graph,
                &mut composite_stack,
                MermaidNode {
                    id,
                    label,
                    shape: NodeShape::Rounded,
                },
            );
            continue;
        }

        // transition
        if let Some(caps) = RE_STATE_TRANS.captures(line) {
            let mut source_id = caps[1].to_string();
            let mut target_id = caps[3].to_string();
            let edge_label = caps.get(4).map(|m| m.as_str().trim().to_string());

            if source_id == "[*]" {
                start_count += 1;
                source_id = if start_count > 1 {
                    format!("_start{}", start_count)
                } else {
                    "_start".to_string()
                };
                register_state_node(
                    &mut graph,
                    &mut composite_stack,
                    MermaidNode {
                        id: source_id.clone(),
                        label: String::new(),
                        shape: NodeShape::StateStart,
                    },
                );
            } else {
                ensure_state_node(&mut graph, &mut composite_stack, &source_id);
            }

            if target_id == "[*]" {
                end_count += 1;
                target_id = if end_count > 1 {
                    format!("_end{}", end_count)
                } else {
                    "_end".to_string()
                };
                register_state_node(
                    &mut graph,
                    &mut composite_stack,
                    MermaidNode {
                        id: target_id.clone(),
                        label: String::new(),
                        shape: NodeShape::StateEnd,
                    },
                );
            } else {
                ensure_state_node(&mut graph, &mut composite_stack, &target_id);
            }

            graph.edges.push(MermaidEdge {
                source: source_id,
                target: target_id,
                label: edge_label,
                style: EdgeStyle::Solid,
                has_arrow_start: false,
                has_arrow_end: true,
            });
            continue;
        }

        // state description
        if let Some(caps) = RE_NODE_LABEL.captures(line) {
            let id = caps[1].to_string();
            let label = caps[2].trim().to_string();
            register_state_node(
                &mut graph,
                &mut composite_stack,
                MermaidNode {
                    id,
                    label,
                    shape: NodeShape::Rounded,
                },
            );
            continue;
        }
    }

    Ok(graph)
}

fn register_state_node(
    graph: &mut MermaidGraph,
    composite_stack: &mut [MermaidSubgraph],
    node: MermaidNode,
) {
    let id = node.id.clone();
    if !graph.nodes.contains_key(&id) {
        graph.nodes.insert(id.clone(), node);
        graph.node_order.push(id.clone()); // Track insertion order
    }
    if let Some(current) = composite_stack.last_mut() {
        if !current.node_ids.contains(&id) {
            current.node_ids.push(id);
        }
    }
}

fn ensure_state_node(graph: &mut MermaidGraph, composite_stack: &mut [MermaidSubgraph], id: &str) {
    if !graph.nodes.contains_key(id) {
        register_state_node(
            graph,
            composite_stack,
            MermaidNode {
                id: id.to_string(),
                label: id.to_string(),
                shape: NodeShape::Rounded,
            },
        );
    } else if let Some(current) = composite_stack.last_mut() {
        if !current.node_ids.contains(&id.to_string()) {
            current.node_ids.push(id.to_string());
        }
    }
}

fn parse_style_props(props_str: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();
    for pair in props_str.split(',') {
        if let Some(colon_idx) = pair.find(':') {
            let key = pair[..colon_idx].trim();
            let val = pair[colon_idx + 1..].trim();
            if !key.is_empty() && !val.is_empty() {
                props.insert(key.to_string(), val.to_string());
            }
        }
    }
    props
}

/// Node shape patterns
struct NodePattern {
    regex: &'static Regex,
    shape: NodeShape,
}

fn get_node_patterns() -> Vec<NodePattern> {
    vec![
        // Triple delimiters (must be first)
        NodePattern {
            regex: &RE_NODE_DOUBLE_CIRCLE,
            shape: NodeShape::DoubleCircle,
        },
        // Double delimiters with mixed brackets
        NodePattern {
            regex: &RE_NODE_STADIUM,
            shape: NodeShape::Stadium,
        },
        NodePattern {
            regex: &RE_NODE_CIRCLE,
            shape: NodeShape::Circle,
        },
        NodePattern {
            regex: &RE_NODE_SUBROUTINE,
            shape: NodeShape::Subroutine,
        },
        NodePattern {
            regex: &RE_NODE_CYLINDER,
            shape: NodeShape::Cylinder,
        },
        // Trapezoid variants
        NodePattern {
            regex: &RE_NODE_TRAPEZOID,
            shape: NodeShape::Trapezoid,
        },
        NodePattern {
            regex: &RE_NODE_TRAPEZOID_ALT,
            shape: NodeShape::TrapezoidAlt,
        },
        // Asymmetric flag shape
        NodePattern {
            regex: &RE_NODE_ASYMMETRIC,
            shape: NodeShape::Asymmetric,
        },
        // Double curly braces (hexagon)
        NodePattern {
            regex: &RE_NODE_HEXAGON,
            shape: NodeShape::Hexagon,
        },
        // Single-char delimiters (last — most common, least specific)
        NodePattern {
            regex: &RE_NODE_RECTANGLE,
            shape: NodeShape::Rectangle,
        },
        NodePattern {
            regex: &RE_NODE_ROUNDED,
            shape: NodeShape::Rounded,
        },
        NodePattern {
            regex: &RE_NODE_DIAMOND,
            shape: NodeShape::Diamond,
        },
    ]
}

/// Parse a line that contains node definitions and edges
fn parse_edge_line(line: &str, graph: &mut MermaidGraph, subgraph_stack: &mut [MermaidSubgraph]) {
    let mut remaining = line.trim();

    // Parse the first node group
    let first_group = consume_node_group(remaining, graph, subgraph_stack);
    if first_group.is_none() {
        return;
    }
    let (first_ids, rest) = first_group.unwrap();
    if first_ids.is_empty() {
        return;
    }
    remaining = rest;

    let mut prev_ids = first_ids;

    // Parse chains of edges
    while !remaining.is_empty() {
        // Try to match an arrow
        if let Some(caps) = RE_ARROW.captures(remaining) {
            let has_arrow_start = caps.get(1).is_some();
            let arrow_op = &caps[2];
            let label = caps.get(3).map(|m| m.as_str().to_string());

            remaining = remaining[caps[0].len()..].trim_start();

            // Determine edge style and arrow end
            let (style, has_arrow_end) = match arrow_op {
                "-->" => (EdgeStyle::Solid, true),
                "---" => (EdgeStyle::Solid, false),
                "-.->" => (EdgeStyle::Dotted, true),
                "-.-" => (EdgeStyle::Dotted, false),
                "==>" => (EdgeStyle::Thick, true),
                "===" => (EdgeStyle::Thick, false),
                _ => (EdgeStyle::Solid, true),
            };

            // Parse target node group
            if let Some((target_ids, rest2)) = consume_node_group(remaining, graph, subgraph_stack)
            {
                remaining = rest2;

                // Create edges for all combinations
                for source in &prev_ids {
                    for target in &target_ids {
                        graph.edges.push(MermaidEdge {
                            source: source.clone(),
                            target: target.clone(),
                            label: label.clone(),
                            style,
                            has_arrow_start,
                            has_arrow_end,
                        });
                    }
                }

                prev_ids = target_ids;
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

/// Consume a node group (possibly with & separators)
fn consume_node_group<'a>(
    input: &'a str,
    graph: &mut MermaidGraph,
    subgraph_stack: &mut [MermaidSubgraph],
) -> Option<(Vec<String>, &'a str)> {
    let mut remaining = input.trim();
    let mut ids = Vec::new();

    loop {
        // Try to parse a node
        if let Some((id, rest)) = consume_single_node(remaining, graph, subgraph_stack) {
            ids.push(id);
            remaining = rest.trim_start();

            // Check for class shorthand :::className
            if remaining.starts_with(":::") {
                if let Some(caps) = RE_CLASS_SUFFIX.captures(remaining) {
                    let class_name = caps[1].to_string();
                    if let Some(last_id) = ids.last() {
                        graph.class_assignments.insert(last_id.clone(), class_name);
                    }
                    remaining = remaining[caps[0].len()..].trim_start();
                }
            }

            // Check for & separator
            if remaining.starts_with('&') {
                remaining = remaining[1..].trim_start();
                continue;
            }
        }
        break;
    }

    if ids.is_empty() {
        None
    } else {
        Some((ids, remaining))
    }
}

/// Consume a single node definition
fn consume_single_node<'a>(
    input: &'a str,
    graph: &mut MermaidGraph,
    subgraph_stack: &mut [MermaidSubgraph],
) -> Option<(String, &'a str)> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let patterns = get_node_patterns();

    // Try each pattern
    for pattern in &patterns {
        if let Some(caps) = pattern.regex.captures(input) {
            let id = caps[1].to_string();
            let label = caps[2].to_string();
            let matched_len = caps[0].len();

            // Register node if new
            if !graph.nodes.contains_key(&id) {
                graph.nodes.insert(
                    id.clone(),
                    MermaidNode {
                        id: id.clone(),
                        label,
                        shape: pattern.shape,
                    },
                );
                graph.node_order.push(id.clone()); // Track insertion order
            }

            // Track in subgraph
            if let Some(sg) = subgraph_stack.last_mut() {
                if !sg.node_ids.contains(&id) {
                    sg.node_ids.push(id.clone());
                }
            }

            return Some((id, &input[matched_len..]));
        }
    }

    // Try bare node (just an ID)
    if let Some(caps) = RE_BARE_ID.captures(input) {
        let id = caps[1].to_string();
        let matched_len = caps[0].len();

        // Register node if new (with default rectangle shape)
        if !graph.nodes.contains_key(&id) {
            graph.nodes.insert(
                id.clone(),
                MermaidNode {
                    id: id.clone(),
                    label: id.clone(),
                    shape: NodeShape::Rectangle,
                },
            );
            graph.node_order.push(id.clone()); // Track insertion order
        }

        // Track in subgraph
        if let Some(sg) = subgraph_stack.last_mut() {
            if !sg.node_ids.contains(&id) {
                sg.node_ids.push(id.clone());
            }
        }

        return Some((id, &input[matched_len..]));
    }

    None
}
