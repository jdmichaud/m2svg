//! Sequence diagram SVG rendering

use super::renderer::escape_xml;
use super::theme::{build_style_block, svg_open_tag, DiagramColors};
use crate::types::SequenceDiagram;
use std::collections::HashMap;

const ACTOR_BOX_HEIGHT: f64 = 40.0;
const ACTOR_PADDING: f64 = 16.0;
const LIFELINE_MIN_GAP: f64 = 120.0;
const MESSAGE_SPACING: f64 = 50.0;

/// Render a sequence diagram to SVG
pub fn render_sequence_svg(
    diagram: &SequenceDiagram,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    if diagram.actors.is_empty() {
        return String::new();
    }

    // Calculate actor box widths based on label lengths
    let actor_widths: Vec<f64> = diagram
        .actors
        .iter()
        .map(|a| (a.label.len() as f64 * 9.0).max(60.0) + ACTOR_PADDING * 2.0)
        .collect();

    // Build actor index for lookup
    let actor_idx: HashMap<&str, usize> = diagram
        .actors
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id.as_str(), i))
        .collect();

    // Calculate gaps between lifelines based on message label lengths
    let mut gaps: Vec<f64> = vec![LIFELINE_MIN_GAP; diagram.actors.len().saturating_sub(1)];
    for msg in &diagram.messages {
        let fi = actor_idx.get(msg.from.as_str()).copied().unwrap_or(0);
        let ti = actor_idx.get(msg.to.as_str()).copied().unwrap_or(0);
        if fi == ti {
            continue;
        }
        let lo = fi.min(ti);
        let hi = fi.max(ti);
        let needed = msg.label.len() as f64 * 8.0 + 40.0;
        let num_gaps = (hi - lo) as f64;
        let per_gap = needed / num_gaps;
        for g in lo..hi {
            gaps[g] = gaps[g].max(per_gap);
        }
    }

    // Calculate lifeline X positions
    let mut ll_x: Vec<f64> = vec![actor_widths[0] / 2.0 + 20.0];
    for i in 1..diagram.actors.len() {
        let gap = gaps[i - 1].max((actor_widths[i - 1] + actor_widths[i]) / 2.0 + 20.0);
        ll_x.push(ll_x[i - 1] + gap);
    }

    // Calculate vertical positions for messages
    let header_y = ACTOR_BOX_HEIGHT + 20.0;
    let mut msg_y: Vec<f64> = Vec::new();
    let mut cur_y = header_y;

    for msg in &diagram.messages {
        let is_self = msg.from == msg.to;
        cur_y += MESSAGE_SPACING;
        if is_self {
            msg_y.push(cur_y);
            cur_y += 30.0; // Extra space for self-loop
        } else {
            msg_y.push(cur_y);
        }
    }

    let footer_y = cur_y + MESSAGE_SPACING;
    let total_height = footer_y + ACTOR_BOX_HEIGHT + 20.0;
    let total_width = ll_x.last().copied().unwrap_or(0.0)
        + actor_widths.last().copied().unwrap_or(60.0) / 2.0
        + 40.0;

    let mut svg = String::new();
    svg.push_str(&svg_open_tag(
        total_width,
        total_height,
        colors,
        transparent,
    ));
    svg.push_str(&build_style_block(font));

    // Draw lifelines (dashed lines between actor boxes)
    for (i, &x) in ll_x.iter().enumerate() {
        let top = ACTOR_BOX_HEIGHT;
        let bottom = footer_y;
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="lifeline"/>"#,
            x, top, x, bottom
        ));
        svg.push_str("\n");

        // Draw actor boxes (header)
        let w = actor_widths[i];
        let label = &diagram.actors[i].label;
        svg.push_str(&draw_actor_box(x, 0.0, w, ACTOR_BOX_HEIGHT, label));

        // Draw actor boxes (footer)
        svg.push_str(&draw_actor_box(x, footer_y, w, ACTOR_BOX_HEIGHT, label));
    }

    // Draw messages
    for (m, msg) in diagram.messages.iter().enumerate() {
        let fi = actor_idx.get(msg.from.as_str()).copied().unwrap_or(0);
        let ti = actor_idx.get(msg.to.as_str()).copied().unwrap_or(0);
        let y = msg_y[m];
        let is_self = fi == ti;
        let is_dashed = msg.line_style == crate::types::LineStyle::Dashed;
        let line_class = if is_dashed {
            "message-dashed"
        } else {
            "message"
        };

        if is_self {
            // Self-message loop
            let x = ll_x[fi];
            let loop_width = 40.0;
            let loop_height = 25.0;
            svg.push_str(&format!(
                r#"<path d="M {:.1} {:.1} h {:.1} v {:.1} h -{:.1}" class="{}" fill="none"/>"#,
                x, y, loop_width, loop_height, loop_width, line_class
            ));
            // Arrowhead
            svg.push_str(&format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" class="arrow"/>"#,
                x,
                y + loop_height,
                x + 8.0,
                y + loop_height - 4.0,
                x + 8.0,
                y + loop_height + 4.0
            ));
            // Label
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" class="message-label">{}</text>"#,
                x + loop_width + 5.0,
                y + loop_height / 2.0 + 4.0,
                escape_xml(&msg.label)
            ));
        } else {
            let from_x = ll_x[fi];
            let to_x = ll_x[ti];
            let left_to_right = ti > fi;

            // Message line
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="{}"/>"#,
                from_x, y, to_x, y, line_class
            ));

            // Arrowhead
            let (ax, dir) = if left_to_right {
                (to_x, -1.0)
            } else {
                (to_x, 1.0)
            };
            svg.push_str(&format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" class="arrow"/>"#,
                ax,
                y,
                ax + dir * 10.0,
                y - 5.0,
                ax + dir * 10.0,
                y + 5.0
            ));

            // Label above line
            let label_x = (from_x + to_x) / 2.0;
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" class="message-label" text-anchor="middle">{}</text>"#,
                label_x,
                y - 8.0,
                escape_xml(&msg.label)
            ));
        }
        svg.push('\n');
    }

    svg.push_str("</svg>");
    svg
}

fn draw_actor_box(cx: f64, top_y: f64, width: f64, height: f64, label: &str) -> String {
    let x = cx - width / 2.0;
    let mut s = String::new();
    s.push_str(&format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" class="node"/>"#,
        x, top_y, width, height
    ));
    s.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" class="node-label" text-anchor="middle" dominant-baseline="middle">{}</text>"#,
        cx, top_y + height / 2.0, escape_xml(label)
    ));
    s.push('\n');
    s
}
