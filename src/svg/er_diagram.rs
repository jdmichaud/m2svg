//! ER diagram SVG rendering

use super::renderer::escape_xml;
use super::theme::{build_style_block, svg_open_tag, DiagramColors};
use crate::types::{Cardinality, ErDiagram};

const BOX_PADDING: f64 = 16.0;
const LINE_HEIGHT: f64 = 22.0;
const H_GAP: f64 = 100.0;

struct EntityBox {
    id: String,
    label: String,
    attr_lines: Vec<String>,
    width: f64,
    height: f64,
    x: f64,
    y: f64,
}

/// Render an ER diagram to SVG
pub fn render_er_svg(
    diagram: &ErDiagram,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    if diagram.entities.is_empty() && diagram.relationships.is_empty() {
        return String::new();
    }

    // Build entity boxes
    let mut entity_boxes: Vec<EntityBox> = Vec::new();

    for entity in &diagram.entities {
        let attr_lines: Vec<String> = entity
            .attributes
            .iter()
            .map(|a| {
                let key_str = a
                    .keys
                    .iter()
                    .map(|k| match k {
                        crate::types::ErKey::PK => "PK",
                        crate::types::ErKey::FK => "FK",
                        crate::types::ErKey::UK => "UK",
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if key_str.is_empty() {
                    format!("{} {}", a.attr_type, a.name)
                } else {
                    format!("{} {} {}", a.attr_type, a.name, key_str)
                }
            })
            .collect();

        let header_width = entity.label.len();
        let attr_width = attr_lines.iter().map(|s| s.len()).max().unwrap_or(0);
        let max_chars = header_width.max(attr_width);
        let box_width = (max_chars as f64 * 8.0).max(80.0) + BOX_PADDING * 2.0;

        let num_lines = 1 + attr_lines.len().max(1); // header + attrs (at least 1 row)
        let box_height = num_lines as f64 * LINE_HEIGHT + BOX_PADDING * 2.0;

        entity_boxes.push(EntityBox {
            id: entity.id.clone(),
            label: entity.label.clone(),
            attr_lines,
            width: box_width,
            height: box_height,
            x: 0.0,
            y: 0.0,
        });
    }

    // Simple horizontal layout
    let mut cur_x = 20.0;
    for eb in &mut entity_boxes {
        eb.x = cur_x;
        eb.y = 50.0;
        cur_x += eb.width + H_GAP;
    }

    // Calculate canvas size
    let total_width = entity_boxes
        .iter()
        .map(|b| b.x + b.width)
        .fold(0.0f64, |a, b| a.max(b))
        + 40.0;
    let total_height = entity_boxes
        .iter()
        .map(|b| b.y + b.height)
        .fold(0.0f64, |a, b| a.max(b))
        + 60.0;

    let mut svg = String::new();
    svg.push_str(&svg_open_tag(
        total_width,
        total_height,
        colors,
        transparent,
    ));
    svg.push_str(&build_style_block(font));

    // Add ER-specific styles
    svg.push_str(
        r#"<style>
.er-line { stroke: var(--line); stroke-width: 1.5; }
.cardinality { font-size: 12px; fill: var(--fg); }
</style>"#,
    );

    // Draw relationships first
    for rel in &diagram.relationships {
        let from_box = entity_boxes.iter().find(|b| b.id == rel.entity1);
        let to_box = entity_boxes.iter().find(|b| b.id == rel.entity2);

        if let (Some(fb), Some(tb)) = (from_box, to_box) {
            svg.push_str(&draw_er_relationship(
                fb,
                tb,
                &rel.cardinality1,
                &rel.cardinality2,
                &rel.label,
            ));
        }
    }

    // Draw entity boxes
    for eb in &entity_boxes {
        svg.push_str(&draw_entity_box(eb));
    }

    svg.push_str("</svg>");
    svg
}

fn draw_entity_box(eb: &EntityBox) -> String {
    let mut s = String::new();

    // Main box
    s.push_str(&format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" class="node"/>"#,
        eb.x, eb.y, eb.width, eb.height
    ));
    s.push('\n');

    let cx = eb.x + eb.width / 2.0;
    let mut cur_y = eb.y + BOX_PADDING + LINE_HEIGHT * 0.7;

    // Entity name (header)
    s.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" class="class-name" text-anchor="middle">{}</text>"#,
        cx,
        cur_y,
        escape_xml(&eb.label)
    ));
    cur_y += LINE_HEIGHT;

    // Divider
    if !eb.attr_lines.is_empty() {
        let div_y = cur_y - LINE_HEIGHT * 0.3;
        s.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="divider"/>"#,
            eb.x,
            div_y,
            eb.x + eb.width,
            div_y
        ));
    }

    // Attributes
    for attr in &eb.attr_lines {
        s.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" class="member">{}</text>"#,
            eb.x + BOX_PADDING,
            cur_y,
            escape_xml(attr)
        ));
        cur_y += LINE_HEIGHT;
    }

    s
}

fn draw_er_relationship(
    from: &EntityBox,
    to: &EntityBox,
    from_card: &Cardinality,
    to_card: &Cardinality,
    label: &str,
) -> String {
    let mut s = String::new();

    // Calculate connection points (horizontal line between boxes)
    let (from_x, from_y, to_x, to_y) = if from.x < to.x {
        (
            from.x + from.width,
            from.y + from.height / 2.0,
            to.x,
            to.y + to.height / 2.0,
        )
    } else {
        (
            from.x,
            from.y + from.height / 2.0,
            to.x + to.width,
            to.y + to.height / 2.0,
        )
    };

    // Main line
    s.push_str(&format!(
        r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
        from_x, from_y, to_x, to_y
    ));
    s.push('\n');

    // From side marker
    s.push_str(&draw_cardinality_marker(
        from_x,
        from_y,
        if from.x < to.x { 1.0 } else { -1.0 },
        from_card,
    ));

    // To side marker
    s.push_str(&draw_cardinality_marker(
        to_x,
        to_y,
        if from.x < to.x { -1.0 } else { 1.0 },
        to_card,
    ));

    // Label in the middle
    let mid_x = (from_x + to_x) / 2.0;
    let mid_y = (from_y + to_y) / 2.0 - 10.0;
    s.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" class="edge-label" text-anchor="middle">{}</text>"#,
        mid_x,
        mid_y,
        escape_xml(label)
    ));
    s.push('\n');

    s
}

fn draw_cardinality_marker(x: f64, y: f64, dir: f64, card: &Cardinality) -> String {
    let mut s = String::new();
    let offset = 15.0;

    match card {
        Cardinality::One => {
            // Two vertical lines (||)
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x + dir * offset,
                y - 8.0,
                x + dir * offset,
                y + 8.0
            ));
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x + dir * (offset + 5.0),
                y - 8.0,
                x + dir * (offset + 5.0),
                y + 8.0
            ));
        }
        Cardinality::ZeroOne => {
            // Circle + vertical line (o|)
            s.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="5" class="marker-hollow"/>"#,
                x + dir * offset,
                y
            ));
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x + dir * (offset + 10.0),
                y - 8.0,
                x + dir * (offset + 10.0),
                y + 8.0
            ));
        }
        Cardinality::ZeroMany => {
            // Circle + crow's foot (o{)
            s.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="5" class="marker-hollow"/>"#,
                x + dir * (offset + 15.0),
                y
            ));
            // Crow's foot (three lines)
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x,
                y,
                x + dir * offset,
                y - 8.0
            ));
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x,
                y,
                x + dir * offset,
                y
            ));
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x,
                y,
                x + dir * offset,
                y + 8.0
            ));
        }
        Cardinality::Many => {
            // Vertical line + crow's foot (}|)
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x + dir * (offset + 10.0),
                y - 8.0,
                x + dir * (offset + 10.0),
                y + 8.0
            ));
            // Crow's foot
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x,
                y,
                x + dir * offset,
                y - 8.0
            ));
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x,
                y,
                x + dir * offset,
                y
            ));
            s.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="er-line"/>"#,
                x,
                y,
                x + dir * offset,
                y + 8.0
            ));
        }
    }
    s.push('\n');
    s
}
