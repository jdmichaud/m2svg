//! SVG renderer - converts a PositionedGraph into an SVG string.
//!
//! Pure string building, no DOM manipulation.
//! Renders back-to-front: groups → edges → edge labels → nodes → node labels.

use super::styles::{
    estimate_text_width, ArrowHead, FontSizes, FontWeights, StrokeWidths, TEXT_BASELINE_SHIFT,
};
use super::theme::{build_style_block, svg_open_tag, DiagramColors};
use super::types::{EdgeStyle, NodeShape, Point, PositionedEdge, PositionedGraph, PositionedGroup, PositionedNode};

/// Render a positioned graph as an SVG string.
pub fn render_svg(
    graph: &PositionedGraph,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    // SVG root with CSS variables + style block + defs
    parts.push(svg_open_tag(graph.width, graph.height, colors, transparent));
    parts.push(build_style_block(font));
    parts.push("<defs>".to_string());
    parts.push(arrow_marker_defs());
    parts.push("</defs>".to_string());

    // 1. Group backgrounds (subgraph rectangles with header bands)
    for group in &graph.groups {
        let rendered = render_group(group);
        if !rendered.is_empty() {
            parts.push(rendered);
        }
    }

    // 2. Edges (polylines — rendered behind nodes)
    for edge in &graph.edges {
        parts.push(render_edge(edge));
    }

    // 3. Edge labels (positioned at midpoint of edge)
    for edge in &graph.edges {
        if edge.label.is_some() {
            parts.push(render_edge_label(edge));
        }
    }

    // 4. Node shapes
    for node in &graph.nodes {
        parts.push(render_node_shape(node));
    }

    // 5. Node labels
    for node in &graph.nodes {
        parts.push(render_node_label(node));
    }

    parts.push("</svg>".to_string());

    parts.join("\n")
}

// ============================================================================
// Arrow marker definitions
// ============================================================================

fn arrow_marker_defs() -> String {
    let w = ArrowHead::WIDTH;
    let h = ArrowHead::HEIGHT;
    format!(
        r#"  <marker id="arrowhead" markerWidth="{w}" markerHeight="{h}" refX="{w}" refY="{half_h}" orient="auto">
    <polygon points="0 0, {w} {half_h}, 0 {h}" fill="var(--_arrow)" />
  </marker>
  <marker id="arrowhead-start" markerWidth="{w}" markerHeight="{h}" refX="0" refY="{half_h}" orient="auto-start-reverse">
    <polygon points="{w} 0, 0 {half_h}, {w} {h}" fill="var(--_arrow)" />
  </marker>"#,
        w = w,
        h = h,
        half_h = h / 2.0
    )
}

// ============================================================================
// Group rendering (subgraph backgrounds)
// ============================================================================

fn render_group(group: &PositionedGroup) -> String {
    // Skip groups without position (e.g., empty subgraphs)
    let (x, y, width, height) = match (group.x, group.y, group.width, group.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x, y, w, h),
        _ => return String::new(),
    };
    
    let header_height = FontSizes::GROUP_HEADER + 16.0;
    let mut parts: Vec<String> = Vec::new();

    // Outer rectangle
    parts.push(format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="0" ry="0" fill="var(--_group-fill)" stroke="var(--_node-stroke)" stroke-width="{}" />"#,
        fmt_num(x), fmt_num(y), fmt_num(width), fmt_num(height), StrokeWidths::OUTER_BOX
    ));

    // Header band
    parts.push(format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="0" ry="0" fill="var(--_group-hdr)" stroke="var(--_node-stroke)" stroke-width="{}" />"#,
        fmt_num(x), fmt_num(y), fmt_num(width), header_height, StrokeWidths::OUTER_BOX
    ));

    // Header label
    parts.push(format!(
        r#"<text x="{}" y="{}" dy="{}" font-size="{}" font-weight="{}" fill="var(--_text-sec)">{}</text>"#,
        fmt_num(x + 12.0),
        fmt_num(y + header_height / 2.0),
        TEXT_BASELINE_SHIFT,
        FontSizes::GROUP_HEADER,
        FontWeights::GROUP_HEADER,
        escape_xml(&group.label)
    ));

    // Render nested groups recursively
    for child in &group.children {
        parts.push(render_group(child));
    }

    parts.join("\n")
}

// ============================================================================
// Edge rendering
// ============================================================================

fn render_edge(edge: &PositionedEdge) -> String {
    if edge.points.len() < 2 {
        return String::new();
    }

    let path_data = points_to_polyline_path(&edge.points);
    let dash_array = if edge.style == EdgeStyle::Dotted {
        " stroke-dasharray=\"4 4\""
    } else {
        ""
    };
    let stroke_width = if edge.style == EdgeStyle::Thick {
        StrokeWidths::CONNECTOR * 2.0
    } else {
        StrokeWidths::CONNECTOR
    };

    // Build marker attributes based on arrow direction flags
    let mut markers = String::new();
    if edge.has_arrow_end {
        markers.push_str(" marker-end=\"url(#arrowhead)\"");
    }
    if edge.has_arrow_start {
        markers.push_str(" marker-start=\"url(#arrowhead-start)\"");
    }

    format!(
        r#"<polyline points="{}" fill="none" stroke="var(--_line)" stroke-width="{}"{}{} />"#,
        path_data, stroke_width, dash_array, markers
    )
}

/// Convert points to SVG polyline points attribute: "x1,y1 x2,y2 ..."
fn points_to_polyline_path(points: &[Point]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_edge_label(edge: &PositionedEdge) -> String {
    let label = match &edge.label {
        Some(l) => l,
        None => return String::new(),
    };

    // Use layout-computed label position when available.
    // Fall back to geometric midpoint of the edge polyline.
    let mid = edge
        .label_position
        .unwrap_or_else(|| edge_midpoint(&edge.points));

    let text_width = estimate_text_width(label, FontSizes::EDGE_LABEL, FontWeights::EDGE_LABEL);
    let padding = 8.0;

    // Background pill behind text for readability
    let bg_width = text_width + padding * 2.0;
    let bg_height = FontSizes::EDGE_LABEL + padding * 2.0;

    format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="4" ry="4" fill="var(--bg)" stroke="var(--_inner-stroke)" stroke-width="0.5" />
<text x="{}" y="{}" text-anchor="middle" dy="{}" font-size="{}" font-weight="{}" fill="var(--_text-muted)">{}</text>"#,
        mid.x - bg_width / 2.0,
        mid.y - bg_height / 2.0,
        bg_width,
        bg_height,
        mid.x,
        mid.y,
        TEXT_BASELINE_SHIFT,
        FontSizes::EDGE_LABEL,
        FontWeights::EDGE_LABEL,
        escape_xml(label)
    )
}

/// Get the midpoint of a polyline (by walking segments)
fn edge_midpoint(points: &[Point]) -> Point {
    if points.is_empty() {
        return Point { x: 0.0, y: 0.0 };
    }
    if points.len() == 1 {
        return points[0];
    }

    // Calculate total length
    let mut total_length = 0.0;
    for i in 1..points.len() {
        total_length += dist(&points[i - 1], &points[i]);
    }

    // Walk to the halfway point
    let mut remaining = total_length / 2.0;
    for i in 1..points.len() {
        let seg_len = dist(&points[i - 1], &points[i]);
        if remaining <= seg_len {
            let t = remaining / seg_len;
            return Point {
                x: points[i - 1].x + t * (points[i].x - points[i - 1].x),
                y: points[i - 1].y + t * (points[i].y - points[i - 1].y),
            };
        }
        remaining -= seg_len;
    }

    points[points.len() - 1]
}

fn dist(a: &Point, b: &Point) -> f64 {
    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
}

// ============================================================================
// Node rendering
// ============================================================================

fn render_node_shape(node: &PositionedNode) -> String {
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;

    // Resolve fill and stroke — inline styles override the CSS variable defaults
    let fill = node
        .inline_style
        .as_ref()
        .and_then(|s| s.get("fill"))
        .map(|s| s.as_str())
        .unwrap_or("var(--_node-fill)");
    let stroke = node
        .inline_style
        .as_ref()
        .and_then(|s| s.get("stroke"))
        .map(|s| s.as_str())
        .unwrap_or("var(--_node-stroke)");
    let default_sw = format!("{}", StrokeWidths::INNER_BOX);
    let sw = node
        .inline_style
        .as_ref()
        .and_then(|s| s.get("stroke-width"))
        .map(|s| s.as_str())
        .unwrap_or(&default_sw);

    match node.shape {
        NodeShape::Diamond => render_diamond(x, y, w, h, fill, stroke, sw),
        NodeShape::Rounded => render_rounded_rect(x, y, w, h, fill, stroke, sw),
        NodeShape::Stadium => render_stadium(x, y, w, h, fill, stroke, sw),
        NodeShape::Circle => render_circle(x, y, w, h, fill, stroke, sw),
        NodeShape::Subroutine => render_subroutine(x, y, w, h, fill, stroke, sw),
        NodeShape::Doublecircle => render_double_circle(x, y, w, h, fill, stroke, sw),
        NodeShape::Hexagon => render_hexagon(x, y, w, h, fill, stroke, sw),
        NodeShape::Cylinder => render_cylinder(x, y, w, h, fill, stroke, sw),
        NodeShape::Asymmetric => render_asymmetric(x, y, w, h, fill, stroke, sw),
        NodeShape::Trapezoid => render_trapezoid(x, y, w, h, fill, stroke, sw),
        NodeShape::TrapezoidAlt => render_trapezoid_alt(x, y, w, h, fill, stroke, sw),
        NodeShape::StateStart => render_state_start(x, y, w, h),
        NodeShape::StateEnd => render_state_end(x, y, w, h),
        NodeShape::Rectangle => render_rect(x, y, w, h, fill, stroke, sw),
    }
}

// --- Basic shapes ---

fn render_rect(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="0" ry="0" fill="{}" stroke="{}" stroke-width="{}" />"#,
        x, y, w, h, fill, stroke, sw
    )
}

fn render_rounded_rect(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="6" ry="6" fill="{}" stroke="{}" stroke-width="{}" />"#,
        x, y, w, h, fill, stroke, sw
    )
}

fn render_stadium(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let r = h / 2.0;
    format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        x, y, w, h, r, r, fill, stroke, sw
    )
}

fn render_circle(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let r = w.min(h) / 2.0;
    format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        cx, cy, r, fill, stroke, sw
    )
}

fn render_diamond(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let hw = w / 2.0;
    let hh = h / 2.0;
    let points = format!(
        "{},{} {},{} {},{} {},{}",
        cx, cy - hh,      // top
        cx + hw, cy,      // right
        cx, cy + hh,      // bottom
        cx - hw, cy       // left
    );
    format!(
        r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        points, fill, stroke, sw
    )
}

// --- Batch 1 shapes ---

fn render_subroutine(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let inset = 8.0;
    format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="0" ry="0" fill="{}" stroke="{}" stroke-width="{}" />
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        x, y, w, h, fill, stroke, sw,
        x + inset, y, x + inset, y + h, stroke, sw,
        x + w - inset, y, x + w - inset, y + h, stroke, sw
    )
}

fn render_double_circle(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let outer_r = w.min(h) / 2.0;
    let inner_r = outer_r - 5.0;
    format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}" />
<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        cx, cy, outer_r, fill, stroke, sw,
        cx, cy, inner_r, fill, stroke, sw
    )
}

fn render_hexagon(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let inset = h / 4.0;
    let points = format!(
        "{},{} {},{} {},{} {},{} {},{} {},{}",
        x + inset, y,           // top-left
        x + w - inset, y,       // top-right
        x + w, y + h / 2.0,     // mid-right
        x + w - inset, y + h,   // bottom-right
        x + inset, y + h,       // bottom-left
        x, y + h / 2.0          // mid-left
    );
    format!(
        r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        points, fill, stroke, sw
    )
}

// --- Batch 2 shapes ---

fn render_cylinder(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let ry = 7.0;
    let cx = x + w / 2.0;
    let body_top = y + ry;
    let body_h = h - 2.0 * ry;

    format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="none" />
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />
<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />
<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        x, body_top, w, body_h, fill,
        x, body_top, x, body_top + body_h, stroke, sw,
        x + w, body_top, x + w, body_top + body_h, stroke, sw,
        cx, y + h - ry, w / 2.0, ry, fill, stroke, sw,
        cx, body_top, w / 2.0, ry, fill, stroke, sw
    )
}

fn render_asymmetric(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let indent = 12.0;
    let points = format!(
        "{},{} {},{} {},{} {},{} {},{}",
        x + indent, y,           // top-left (indented)
        x + w, y,                // top-right
        x + w, y + h,            // bottom-right
        x + indent, y + h,       // bottom-left (indented)
        x, y + h / 2.0           // left point
    );
    format!(
        r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        points, fill, stroke, sw
    )
}

fn render_trapezoid(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let inset = w * 0.15;
    let points = format!(
        "{},{} {},{} {},{} {},{}",
        x + inset, y,            // top-left (indented)
        x + w - inset, y,        // top-right (indented)
        x + w, y + h,            // bottom-right (full width)
        x, y + h                 // bottom-left (full width)
    );
    format!(
        r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        points, fill, stroke, sw
    )
}

fn render_trapezoid_alt(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str, sw: &str) -> String {
    let inset = w * 0.15;
    let points = format!(
        "{},{} {},{} {},{} {},{}",
        x, y,                        // top-left (full width)
        x + w, y,                    // top-right (full width)
        x + w - inset, y + h,        // bottom-right (indented)
        x + inset, y + h             // bottom-left (indented)
    );
    format!(
        r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        points, fill, stroke, sw
    )
}

// --- Batch 3: State diagram pseudostates ---

fn render_state_start(x: f64, y: f64, w: f64, h: f64) -> String {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let r = w.min(h) / 2.0 - 2.0;
    format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="var(--_text)" stroke="none" />"#,
        cx, cy, r
    )
}

fn render_state_end(x: f64, y: f64, w: f64, h: f64) -> String {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let outer_r = w.min(h) / 2.0 - 2.0;
    let inner_r = outer_r - 4.0;
    format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="var(--_text)" stroke-width="{}" />
<circle cx="{}" cy="{}" r="{}" fill="var(--_text)" stroke="none" />"#,
        cx, cy, outer_r, StrokeWidths::INNER_BOX * 2.0,
        cx, cy, inner_r
    )
}

// ============================================================================
// Node label rendering
// ============================================================================

fn render_node_label(node: &PositionedNode) -> String {
    // State pseudostates have no label
    if matches!(node.shape, NodeShape::StateStart | NodeShape::StateEnd) {
        if node.label.is_empty() {
            return String::new();
        }
    }

    let cx = node.x + node.width / 2.0;
    let cy = node.y + node.height / 2.0;

    // Resolve text color — inline styles can override the CSS variable default
    let text_color = node
        .inline_style
        .as_ref()
        .and_then(|s| s.get("color"))
        .map(|s| s.as_str())
        .unwrap_or("var(--_text)");

    format!(
        r#"<text x="{}" y="{}" text-anchor="middle" dy="{}" font-size="{}" font-weight="{}" fill="{}">{}</text>"#,
        cx, cy, TEXT_BASELINE_SHIFT, FontSizes::NODE_LABEL, FontWeights::NODE_LABEL, text_color, escape_xml(&node.label)
    )
}

// ============================================================================
// Utilities
// ============================================================================

/// Escape special XML characters in text content
pub fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Format a float to match JavaScript's number-to-string behavior.
/// JavaScript outputs full precision for floating point numbers.
fn fmt_num(n: f64) -> String {
    // Use JavaScript-compatible precision
    // Rust's default float display matches JS for most cases
    // Just need to handle integer values without decimal point
    let s = format!("{}", n);
    // If it already looks good (has decimal or is integer), return as-is
    if s.contains('.') || !s.contains('e') {
        s
    } else {
        s
    }
}
