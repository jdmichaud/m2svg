//! Class diagram SVG rendering

use super::renderer::escape_xml;
use super::theme::{build_style_block, svg_open_tag, DiagramColors};
use crate::types::{ClassDiagram, ClassMember, RelationshipType, Visibility};
use std::collections::{HashMap, HashSet};

const BOX_PADDING: f64 = 12.0;
const LINE_HEIGHT: f64 = 20.0;
const H_GAP: f64 = 60.0;
const V_GAP: f64 = 50.0;

#[allow(dead_code)]
struct ClassBox {
    id: String,
    label: String,
    annotation: Option<String>,
    attr_lines: Vec<String>,
    method_lines: Vec<String>,
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    is_lollipop: bool,
}

/// Render a class diagram to SVG
pub fn render_class_svg(
    diagram: &ClassDiagram,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    if diagram.classes.is_empty() {
        return String::new();
    }

    // Build box dimensions for each class
    let mut class_boxes: HashMap<String, ClassBox> = HashMap::new();

    for cls in &diagram.classes {
        // Lollipop interface nodes are rendered as plain text (no box)
        if cls.is_lollipop {
            let text_width = (cls.label.len() as f64 * 8.0).max(40.0);
            class_boxes.insert(
                cls.id.clone(),
                ClassBox {
                    id: cls.id.clone(),
                    label: cls.label.clone(),
                    annotation: None,
                    attr_lines: Vec::new(),
                    method_lines: Vec::new(),
                    width: text_width,
                    height: LINE_HEIGHT,
                    x: 0.0,
                    y: 0.0,
                    is_lollipop: true,
                },
            );
            continue;
        }

        let annotation_str = cls.annotation.as_ref().map(|a| format!("<<{}>>", a));

        let attr_lines: Vec<String> = cls.attributes.iter().map(format_member).collect();
        let method_lines: Vec<String> = cls.methods.iter().map(format_member).collect();

        // Calculate width based on widest line
        let annotation_width = annotation_str.as_ref().map(|s| s.len()).unwrap_or(0);
        let header_width = cls.label.len();
        let attr_width = attr_lines.iter().map(|s| s.len()).max().unwrap_or(0);
        let method_width = method_lines.iter().map(|s| s.len()).max().unwrap_or(0);

        let max_chars = header_width
            .max(attr_width)
            .max(method_width)
            .max(annotation_width);
        let box_width = (max_chars as f64 * 8.0).max(80.0) + BOX_PADDING * 2.0;

        // Calculate height
        let has_annotation = cls.annotation.is_some();
        let header_lines = if has_annotation { 2 } else { 1 };
        let mut total_lines = header_lines;

        if !attr_lines.is_empty() || !method_lines.is_empty() {
            total_lines += 1; // divider
            total_lines += attr_lines.len().max(1);
        }
        if !method_lines.is_empty() {
            total_lines += 1; // divider
            total_lines += method_lines.len();
        }

        let box_height = total_lines as f64 * LINE_HEIGHT + BOX_PADDING * 2.0;

        class_boxes.insert(
            cls.id.clone(),
            ClassBox {
                id: cls.id.clone(),
                label: cls.label.clone(),
                annotation: cls.annotation.clone(),
                attr_lines,
                method_lines,
                width: box_width,
                height: box_height,
                x: 0.0,
                y: 0.0,
                is_lollipop: false,
            },
        );
    }

    // Assign levels using relationship hierarchy
    let mut parents: HashMap<String, HashSet<String>> = HashMap::new();
    let mut children: HashMap<String, HashSet<String>> = HashMap::new();

    for rel in &diagram.relationships {
        let is_hierarchical = matches!(
            rel.rel_type,
            RelationshipType::Inheritance | RelationshipType::Realization
        );
        let (parent, child) = if is_hierarchical && rel.marker_at_from {
            (rel.from.clone(), rel.to.clone())
        } else if is_hierarchical {
            (rel.to.clone(), rel.from.clone())
        } else {
            (rel.from.clone(), rel.to.clone())
        };

        parents
            .entry(child.clone())
            .or_default()
            .insert(parent.clone());
        children.entry(parent).or_default().insert(child);
    }

    // Compute levels (BFS from roots)
    let mut levels: HashMap<String, usize> = HashMap::new();
    let allids: HashSet<_> = class_boxes.keys().cloned().collect();
    let roots: Vec<_> = allids
        .iter()
        .filter(|id| parents.get(*id).map(|p| p.is_empty()).unwrap_or(true))
        .cloned()
        .collect();

    for root in &roots {
        if !levels.contains_key(root) {
            levels.insert(root.clone(), 0);
        }
    }

    let mut queue: Vec<String> = roots.clone();
    while let Some(id) = queue.pop() {
        let level = *levels.get(&id).unwrap_or(&0);
        if let Some(kids) = children.get(&id) {
            for kid in kids {
                let new_level = level + 1;
                if !levels.contains_key(kid) || levels[kid] < new_level {
                    levels.insert(kid.clone(), new_level);
                    queue.push(kid.clone());
                }
            }
        }
    }

    // Assign unconnected nodes to level 0
    for id in &allids {
        levels.entry(id.clone()).or_insert(0);
    }

    // Group by level and position - sort by id for deterministic output
    let max_level = levels.values().copied().max().unwrap_or(0);
    let mut level_nodes: Vec<Vec<String>> = vec![Vec::new(); max_level + 1];
    let mut sorted_ids: Vec<_> = levels.iter().collect();
    sorted_ids.sort_by_key(|(id, _)| *id);
    for (id, level) in sorted_ids {
        level_nodes[*level].push(id.clone());
    }

    // Position boxes
    for (level, nodes) in level_nodes.iter().enumerate() {
        let mut cur_x = 20.0;
        let level_y = level as f64 * (150.0 + V_GAP) + 20.0;

        for id in nodes {
            if let Some(b) = class_boxes.get_mut(id) {
                b.x = cur_x;
                b.y = level_y;
                cur_x += b.width + H_GAP;
            }
        }
    }

    // Calculate canvas size
    let total_width = class_boxes
        .values()
        .map(|b| b.x + b.width)
        .fold(0.0f64, |a, b| a.max(b))
        + 40.0;
    let total_height = class_boxes
        .values()
        .map(|b| b.y + b.height)
        .fold(0.0f64, |a, b| a.max(b))
        + 40.0;

    let mut svg = String::new();
    svg.push_str(&svg_open_tag(
        total_width,
        total_height,
        colors,
        transparent,
    ));
    svg.push_str(&build_style_block(font));

    // Draw relationships first (behind boxes)
    for rel in &diagram.relationships {
        let from_box = class_boxes.get(&rel.from);
        let to_box = class_boxes.get(&rel.to);
        if let (Some(fb), Some(tb)) = (from_box, to_box) {
            svg.push_str(&draw_relationship(
                fb,
                tb,
                &rel.rel_type,
                rel.marker_at_from,
            ));
        }
    }

    // Draw class boxes - sort by id for deterministic output
    let mut sorted_boxes: Vec<_> = class_boxes.values().collect();
    sorted_boxes.sort_by_key(|b| &b.id);
    for b in sorted_boxes {
        if b.is_lollipop {
            svg.push_str(&draw_lollipop_label(b));
        } else {
            svg.push_str(&draw_class_box(b));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn format_member(m: &ClassMember) -> String {
    let vis = match m.visibility {
        Visibility::Public => "+",
        Visibility::Private => "-",
        Visibility::Protected => "#",
        Visibility::Package => "~",
        Visibility::None => "",
    };
    if m.is_method {
        let params = m.params.as_deref().unwrap_or("");
        let has_params = !params.is_empty();
        let ret = match &m.member_type {
            Some(t) if !t.eq_ignore_ascii_case("void") && !has_params => {
                format!(" : {}", t)
            }
            _ => String::new(),
        };
        format!("{}{}({}){}", vis, m.name, params, ret)
    } else if let Some(ref member_type) = m.member_type {
        format!("{}{} : {}", vis, m.name, member_type)
    } else {
        format!("{}{}", vis, m.name)
    }
}

fn draw_class_box(b: &ClassBox) -> String {
    let mut s = String::new();

    // Main box
    s.push_str(&format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" class="node"/>"#,
        b.x, b.y, b.width, b.height
    ));
    s.push('\n');

    let mut cur_y = b.y + BOX_PADDING + LINE_HEIGHT * 0.7;
    let cx = b.x + b.width / 2.0;

    // Annotation (if any)
    if let Some(ref ann) = b.annotation {
        s.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" class="annotation" text-anchor="middle">&lt;&lt;{}&gt;&gt;</text>"#,
            cx, cur_y, escape_xml(ann)
        ));
        cur_y += LINE_HEIGHT;
    }

    // Class name (bold)
    s.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" class="class-name" text-anchor="middle">{}</text>"#,
        cx,
        cur_y,
        escape_xml(&b.label)
    ));
    cur_y += LINE_HEIGHT;

    // Divider after header
    if !b.attr_lines.is_empty() || !b.method_lines.is_empty() {
        let div_y = cur_y - LINE_HEIGHT * 0.3 + BOX_PADDING / 2.0;
        s.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="divider"/>"#,
            b.x,
            div_y,
            b.x + b.width,
            div_y
        ));
        cur_y += BOX_PADDING / 2.0;
    }

    // Attributes
    for attr in &b.attr_lines {
        s.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" class="member">{}</text>"#,
            b.x + BOX_PADDING,
            cur_y,
            escape_xml(attr)
        ));
        cur_y += LINE_HEIGHT;
    }

    // Divider before methods
    if !b.method_lines.is_empty() {
        let div_y = cur_y - LINE_HEIGHT * 0.3 + BOX_PADDING / 2.0;
        s.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="divider"/>"#,
            b.x,
            div_y,
            b.x + b.width,
            div_y
        ));
        cur_y += BOX_PADDING / 2.0;
    }

    // Methods
    for method in &b.method_lines {
        s.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" class="member">{}</text>"#,
            b.x + BOX_PADDING,
            cur_y,
            escape_xml(method)
        ));
        cur_y += LINE_HEIGHT;
    }

    s
}

fn draw_lollipop_label(b: &ClassBox) -> String {
    let cx = b.x + b.width / 2.0;
    let cy = b.y + LINE_HEIGHT * 0.7;
    format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" class=\"class-name\" text-anchor=\"middle\">{}</text>\n",
        cx,
        cy,
        escape_xml(&b.label)
    )
}

fn draw_relationship(
    from: &ClassBox,
    to: &ClassBox,
    rel_type: &RelationshipType,
    marker_at_from: bool,
) -> String {
    let mut s = String::new();

    // Calculate connection points
    let (from_x, from_y, to_x, to_y) = if from.y < to.y {
        // from is above to
        (
            from.x + from.width / 2.0,
            from.y + from.height,
            to.x + to.width / 2.0,
            to.y,
        )
    } else if from.y > to.y {
        // from is below to
        (
            from.x + from.width / 2.0,
            from.y,
            to.x + to.width / 2.0,
            to.y + to.height,
        )
    } else if from.x < to.x {
        // same level, from is left
        (
            from.x + from.width,
            from.y + from.height / 2.0,
            to.x,
            to.y + to.height / 2.0,
        )
    } else {
        // same level, from is right
        (
            from.x,
            from.y + from.height / 2.0,
            to.x + to.width,
            to.y + to.height / 2.0,
        )
    };

    let is_dashed = matches!(
        rel_type,
        RelationshipType::Dependency | RelationshipType::Realization
    );
    let line_class = if is_dashed { "rel-dashed" } else { "rel-line" };

    s.push_str(&format!(
        r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="{}"/>"#,
        from_x, from_y, to_x, to_y, line_class
    ));
    s.push('\n');

    // Draw marker at appropriate end
    let (marker_x, marker_y, dx, dy) = if marker_at_from {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        (from_x, from_y, dx, dy)
    } else {
        let dx = from_x - to_x;
        let dy = from_y - to_y;
        (to_x, to_y, dx, dy)
    };

    let len = (dx * dx + dy * dy).sqrt();
    if len > 0.0 {
        let (ndx, ndy) = (dx / len, dy / len);
        s.push_str(&draw_marker(marker_x, marker_y, ndx, ndy, rel_type));
    }

    s
}

fn draw_marker(x: f64, y: f64, dx: f64, dy: f64, rel_type: &RelationshipType) -> String {
    let size = 12.0;

    // Perpendicular direction
    let (px, py) = (-dy, dx);

    match rel_type {
        RelationshipType::Inheritance | RelationshipType::Realization => {
            // Hollow triangle
            let tip_x = x;
            let tip_y = y;
            let base_x = x + dx * size;
            let base_y = y + dy * size;
            let left_x = base_x + px * size / 2.0;
            let left_y = base_y + py * size / 2.0;
            let right_x = base_x - px * size / 2.0;
            let right_y = base_y - py * size / 2.0;
            format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" class="marker-hollow"/>"#,
                tip_x, tip_y, left_x, left_y, right_x, right_y
            )
        }
        RelationshipType::Composition => {
            // Filled diamond
            let tip_x = x;
            let tip_y = y;
            let mid_x = x + dx * size / 2.0;
            let mid_y = y + dy * size / 2.0;
            let back_x = x + dx * size;
            let back_y = y + dy * size;
            let left_x = mid_x + px * size / 3.0;
            let left_y = mid_y + py * size / 3.0;
            let right_x = mid_x - px * size / 3.0;
            let right_y = mid_y - py * size / 3.0;
            format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" class="marker-filled"/>"#,
                tip_x, tip_y, left_x, left_y, back_x, back_y, right_x, right_y
            )
        }
        RelationshipType::Aggregation => {
            // Hollow diamond
            let tip_x = x;
            let tip_y = y;
            let mid_x = x + dx * size / 2.0;
            let mid_y = y + dy * size / 2.0;
            let back_x = x + dx * size;
            let back_y = y + dy * size;
            let left_x = mid_x + px * size / 3.0;
            let left_y = mid_y + py * size / 3.0;
            let right_x = mid_x - px * size / 3.0;
            let right_y = mid_y - py * size / 3.0;
            format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" class="marker-hollow"/>"#,
                tip_x, tip_y, left_x, left_y, back_x, back_y, right_x, right_y
            )
        }
        RelationshipType::Dependency => {
            // Open arrow
            let tip_x = x;
            let tip_y = y;
            let left_x = x + dx * size + px * size / 2.0;
            let left_y = y + dy * size + py * size / 2.0;
            let right_x = x + dx * size - px * size / 2.0;
            let right_y = y + dy * size - py * size / 2.0;
            format!(
                r#"<polyline points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" class="marker-open" fill="none"/>"#,
                left_x, left_y, tip_x, tip_y, right_x, right_y
            )
        }
        _ => {
            // Default arrow
            let tip_x = x;
            let tip_y = y;
            let left_x = x + dx * size + px * size / 2.0;
            let left_y = y + dy * size + py * size / 2.0;
            let right_x = x + dx * size - px * size / 2.0;
            let right_y = y + dy * size - py * size / 2.0;
            format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" class="marker-filled"/>"#,
                tip_x, tip_y, left_x, left_y, right_x, right_y
            )
        }
    }
}
