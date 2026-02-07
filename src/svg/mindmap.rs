//! SVG renderer for Mindmap diagrams
//!
//! Renders mindmaps with root in center and children radiating outward.

use super::DiagramColors;
use crate::parser::mindmap::{Mindmap, MindmapNode, NodeShape};

/// Layout constants
const NODE_PADDING: f64 = 10.0;
const LEVEL_SPACING: f64 = 100.0;
const VERTICAL_SPACING: f64 = 50.0;
const FONT_SIZE: f64 = 14.0;
const CHAR_WIDTH: f64 = 8.0;
const MARGIN: f64 = 40.0;

/// Colors for different depth levels (matching mermaid's color scheme)
const DEPTH_COLORS: &[&str] = &[
    "#6666FF", // Root - blue/purple
    "#FFFF66", // Level 0 - yellow
    "#99FF99", // Level 1 - light green
    "#CC99FF", // Level 2 - light purple
    "#FF99CC", // Level 3 - pink
    "#99FFFF", // Level 4 - cyan
    "#FFCC99", // Level 5 - peach
];

fn get_depth_color(depth: usize) -> &'static str {
    if depth == 0 {
        DEPTH_COLORS[0] // Root gets special color
    } else {
        DEPTH_COLORS[((depth - 1) % (DEPTH_COLORS.len() - 1)) + 1]
    }
}

fn get_text_color(depth: usize) -> &'static str {
    if depth == 0 {
        "#FFFFFF" // White text on dark root
    } else {
        "#000000" // Black text on light colors
    }
}

/// Positioned node for rendering
struct PositionedNode {
    cx: f64,     // center x
    cy: f64,     // center y
    radius: f64, // for circles
    width: f64,  // for rectangles
    height: f64,
    label: String,
    shape: NodeShape,
    depth: usize,
    children: Vec<PositionedNode>,
}

/// Render a Mindmap to SVG
pub fn render_mindmap_svg(
    mindmap: &Mindmap,
    _colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    let Some(root) = &mindmap.root else {
        return empty_svg(transparent);
    };

    // Calculate root size
    let root_radius = calculate_node_radius(&root.label);

    // Split children: alternate between right (even index) and left (odd index)
    let right_children: Vec<_> = root
        .children
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, c)| c)
        .collect();
    let left_children: Vec<_> = root
        .children
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, c)| c)
        .collect();

    let right_height = calculate_side_height(&right_children);
    let left_height = calculate_side_height(&left_children);
    let max_side_height = right_height.max(left_height).max(root_radius * 2.0);

    // Root position - center of the diagram
    let left_width = calculate_max_child_width(&left_children);
    let root_cx = MARGIN + left_width + root_radius;
    let root_cy = MARGIN + max_side_height / 2.0;

    // Position all children
    let mut positioned_children: Vec<PositionedNode> = Vec::new();

    // Position right-side children
    let mut y = root_cy - right_height / 2.0;
    for child in &right_children {
        let child_pos = position_subtree(child, root_cx + root_radius + LEVEL_SPACING, y, 1, true);
        y += subtree_height(&child_pos) + VERTICAL_SPACING;
        positioned_children.push(child_pos);
    }

    // Position left-side children
    let mut y = root_cy - left_height / 2.0;
    for child in &left_children {
        let child_pos = position_subtree(child, root_cx - root_radius - LEVEL_SPACING, y, 1, false);
        y += subtree_height(&child_pos) + VERTICAL_SPACING;
        positioned_children.push(child_pos);
    }

    let root_positioned = PositionedNode {
        cx: root_cx,
        cy: root_cy,
        radius: root_radius,
        width: root_radius * 2.0,
        height: root_radius * 2.0,
        label: root.label.clone(),
        shape: root.shape.clone(),
        depth: 0,
        children: positioned_children,
    };

    // Calculate bounds (min_x, max_x, min_y, max_y)
    let (min_x, max_x, min_y, max_y) = calculate_bounds(&root_positioned);

    // Calculate offsets to ensure everything is visible with margin
    let offset_x = MARGIN - min_x;
    let offset_y = MARGIN - min_y;
    let width = max_x - min_x + MARGIN * 2.0;
    let height = max_y - min_y + MARGIN * 2.0;

    let mut svg = String::new();

    // SVG header
    let bg_color = if transparent { "none" } else { "#FFFFFF" };
    svg.push_str(&format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100%" viewBox="0 0 {:.0} {:.0}" preserveAspectRatio="xMidYMid meet">
<style>
  .node-text {{ font-family: '{}', sans-serif; font-size: {}px; dominant-baseline: middle; text-anchor: middle; }}
</style>
<rect width="100%" height="100%" fill="{}"/>
<g transform="translate({:.0}, {:.0})">
"##,
        width, height,
        font, FONT_SIZE,
        bg_color,
        offset_x, offset_y
    ));

    // Draw connectors first (behind nodes)
    draw_connectors(&root_positioned, &mut svg);

    // Draw nodes
    draw_node(&root_positioned, &mut svg);

    svg.push_str("</g>\n</svg>\n");
    svg
}

fn empty_svg(transparent: bool) -> String {
    let bg_color = if transparent { "none" } else { "#FFFFFF" };
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">
<rect width="100%" height="100%" fill="{}"/>
</svg>
"##,
        bg_color
    )
}

fn calculate_node_radius(label: &str) -> f64 {
    let text_width = label.chars().count() as f64 * CHAR_WIDTH;
    (text_width / 2.0 + NODE_PADDING).max(25.0)
}

fn calculate_node_size(node: &MindmapNode) -> (f64, f64) {
    let text_width = node.label.chars().count() as f64 * CHAR_WIDTH;
    let width = text_width + NODE_PADDING * 2.0;
    let height = FONT_SIZE + NODE_PADDING * 2.0;
    (width.max(60.0), height.max(30.0))
}

fn calculate_side_height(children: &[&MindmapNode]) -> f64 {
    if children.is_empty() {
        return 0.0;
    }
    let total: f64 = children.iter().map(|c| estimate_subtree_height(c)).sum();
    total + (children.len() - 1) as f64 * VERTICAL_SPACING
}

fn calculate_max_child_width(children: &[&MindmapNode]) -> f64 {
    children
        .iter()
        .map(|c| {
            let (w, _) = calculate_node_size(c);
            w + estimate_subtree_width(c)
        })
        .fold(0.0, f64::max)
}

fn estimate_subtree_height(node: &MindmapNode) -> f64 {
    if node.children.is_empty() {
        let (_, h) = calculate_node_size(node);
        return h;
    }
    let children_height: f64 = node
        .children
        .iter()
        .map(|c| estimate_subtree_height(c))
        .sum();
    children_height + (node.children.len() - 1) as f64 * VERTICAL_SPACING
}

fn estimate_subtree_width(node: &MindmapNode) -> f64 {
    if node.children.is_empty() {
        return 0.0;
    }
    let max_child_width = node
        .children
        .iter()
        .map(|c| {
            let (w, _) = calculate_node_size(c);
            w + estimate_subtree_width(c)
        })
        .fold(0.0, f64::max);
    LEVEL_SPACING + max_child_width
}

fn position_subtree(
    node: &MindmapNode,
    x: f64,
    start_y: f64,
    depth: usize,
    right_side: bool,
) -> PositionedNode {
    let (width, height) = calculate_node_size(node);
    let radius = if matches!(node.shape, NodeShape::Circle) {
        calculate_node_radius(&node.label)
    } else {
        width / 2.0
    };

    if node.children.is_empty() {
        return PositionedNode {
            cx: x,
            cy: start_y + height / 2.0,
            radius,
            width,
            height,
            label: node.label.clone(),
            shape: node.shape.clone(),
            depth,
            children: vec![],
        };
    }

    // Position children
    let children_height = estimate_subtree_height(node) - height;
    let node_cy = start_y + estimate_subtree_height(node) / 2.0;

    let child_x = if right_side {
        x + radius + LEVEL_SPACING
    } else {
        x - radius - LEVEL_SPACING
    };

    let mut children = Vec::new();
    let mut y = node_cy - children_height / 2.0 - VERTICAL_SPACING / 2.0;

    for child in &node.children {
        let child_pos = position_subtree(child, child_x, y, depth + 1, right_side);
        y += subtree_height(&child_pos) + VERTICAL_SPACING;
        children.push(child_pos);
    }

    PositionedNode {
        cx: x,
        cy: node_cy,
        radius,
        width,
        height,
        label: node.label.clone(),
        shape: node.shape.clone(),
        depth,
        children,
    }
}

fn subtree_height(node: &PositionedNode) -> f64 {
    if node.children.is_empty() {
        return node.height;
    }
    let children_height: f64 = node.children.iter().map(|c| subtree_height(c)).sum();
    children_height + (node.children.len() - 1) as f64 * VERTICAL_SPACING
}

fn calculate_bounds(node: &PositionedNode) -> (f64, f64, f64, f64) {
    let node_left = node.cx - node.radius;
    let node_right = node.cx + node.radius;
    let node_top = node.cy - node.radius;
    let node_bottom = node.cy + node.radius;

    let mut min_x = node_left;
    let mut max_x = node_right;
    let mut min_y = node_top;
    let mut max_y = node_bottom;

    for child in &node.children {
        let (child_min_x, child_max_x, child_min_y, child_max_y) = calculate_bounds(child);
        min_x = min_x.min(child_min_x);
        max_x = max_x.max(child_max_x);
        min_y = min_y.min(child_min_y);
        max_y = max_y.max(child_max_y);
    }

    (min_x, max_x, min_y, max_y)
}

/// Draw connectors from a node to its children
fn draw_connectors(node: &PositionedNode, svg: &mut String) {
    for child in &node.children {
        let is_right = child.cx > node.cx;

        let start_x = if is_right {
            node.cx + node.radius
        } else {
            node.cx - node.radius
        };
        let start_y = node.cy;
        let end_x = if is_right {
            child.cx - child.radius
        } else {
            child.cx + child.radius
        };
        let end_y = child.cy;

        // Use quadratic bezier for smoother curves
        let ctrl_x = (start_x + end_x) / 2.0;

        let color = get_depth_color(child.depth);
        let stroke_width = (5 - child.depth).max(2);

        svg.push_str(&format!(
            r##"<path d="M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}" stroke="{}" stroke-width="{}" fill="none"/>"##,
            start_x, start_y,
            ctrl_x, end_y,
            end_x, end_y,
            color, stroke_width
        ));
        svg.push('\n');

        // Recurse
        draw_connectors(child, svg);
    }
}

/// Draw a node and its children
fn draw_node(node: &PositionedNode, svg: &mut String) {
    let fill = get_depth_color(node.depth);
    let text_fill = get_text_color(node.depth);

    // Draw shape based on node type
    match &node.shape {
        NodeShape::Circle => {
            // True circle
            svg.push_str(&format!(
                r##"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" stroke="#333" stroke-width="1.5"/>"##,
                node.cx, node.cy, node.radius, fill
            ));
        }
        NodeShape::Rounded => {
            let x = node.cx - node.width / 2.0;
            let y = node.cy - node.height / 2.0;
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" fill="{}" stroke="#333" stroke-width="1.5"/>"##,
                x, y, node.width, node.height, node.height / 2.0, fill
            ));
        }
        NodeShape::Square => {
            let x = node.cx - node.width / 2.0;
            let y = node.cy - node.height / 2.0;
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" stroke="#333" stroke-width="1.5"/>"##,
                x, y, node.width, node.height, fill
            ));
        }
        NodeShape::Hexagon => {
            let x = node.cx - node.width / 2.0;
            let y = node.cy - node.height / 2.0;
            let inset = 15.0;
            let points = format!(
                "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                x + inset,
                y,
                x + node.width - inset,
                y,
                x + node.width,
                node.cy,
                x + node.width - inset,
                y + node.height,
                x + inset,
                y + node.height,
                x,
                node.cy
            );
            svg.push_str(&format!(
                r##"<polygon points="{}" fill="{}" stroke="#333" stroke-width="1.5"/>"##,
                points, fill
            ));
        }
        NodeShape::Bang => {
            // Explosion/starburst shape - use a jagged polygon
            let r = node.radius;
            let mut points = String::new();
            for i in 0..12 {
                let angle = (i as f64) * std::f64::consts::PI / 6.0 - std::f64::consts::PI / 2.0;
                let radius = if i % 2 == 0 { r } else { r * 0.6 };
                let px = node.cx + radius * angle.cos();
                let py = node.cy + radius * angle.sin();
                if i > 0 {
                    points.push(' ');
                }
                points.push_str(&format!("{:.1},{:.1}", px, py));
            }
            svg.push_str(&format!(
                r##"<polygon points="{}" fill="{}" stroke="#333" stroke-width="1.5"/>"##,
                points, fill
            ));
        }
        NodeShape::Cloud => {
            // Cloud shape - simplified as a rounded blob
            svg.push_str(&format!(
                r##"<ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="{}" stroke="#333" stroke-width="1.5"/>"##,
                node.cx, node.cy, node.radius, node.radius * 0.7, fill
            ));
        }
        NodeShape::Default => {
            // Default - rounded rectangle
            let x = node.cx - node.width / 2.0;
            let y = node.cy - node.height / 2.0;
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4" fill="{}" stroke="#333" stroke-width="1.5"/>"##,
                x, y, node.width, node.height, fill
            ));
        }
    }
    svg.push('\n');

    // Draw text
    svg.push_str(&format!(
        r##"<text class="node-text" x="{:.1}" y="{:.1}" fill="{}">{}</text>"##,
        node.cx,
        node.cy,
        text_fill,
        escape_xml(&node.label)
    ));
    svg.push('\n');

    // Draw children
    for child in &node.children {
        draw_node(child, svg);
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
