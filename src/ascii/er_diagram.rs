//! ER diagram ASCII rendering

use super::canvas::{canvas_to_string, draw_text, mk_canvas, set_char};
use super::types::AsciiConfig;
use crate::types::{Cardinality, ErDiagram};

/// Render an ER diagram to ASCII
pub fn render_er_ascii(diagram: &ErDiagram, config: &AsciiConfig) -> Result<String, String> {
    if diagram.entities.is_empty() && diagram.relationships.is_empty() {
        return Ok(String::new());
    }

    render_general_er(diagram, config)
}

/// Format an entity's attributes as display strings (e.g. "PK string name")
fn format_entity_attrs(entity: &crate::types::ErEntity) -> Vec<String> {
    entity
        .attributes
        .iter()
        .map(|a| {
            let key_prefix = a
                .keys
                .iter()
                .map(|k| match k {
                    crate::types::ErKey::PK => "PK",
                    crate::types::ErKey::FK => "FK",
                    crate::types::ErKey::UK => "UK",
                })
                .collect::<Vec<_>>()
                .join(" ");
            if key_prefix.is_empty() {
                format!("   {} {}", a.attr_type, a.name)
            } else {
                format!("{} {} {}", key_prefix, a.attr_type, a.name)
            }
        })
        .collect()
}

/// General case: render entities chained by relationships inline.
///
/// Entities are ordered by following the relationship chain. Each relationship
/// is drawn as a label + cardinality connector in the gap between adjacent boxes.
/// Entity boxes include attribute rows when attributes are defined.
fn render_general_er(diagram: &ErDiagram, config: &AsciiConfig) -> Result<String, String> {
    let use_ascii = config.use_ascii;

    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    let (div_l, div_r) = if use_ascii {
        ('+', '+')
    } else {
        ('├', '┤')
    };

    // Build an ordered sequence of entities by walking the relationship chain.
    let mut ordered_ids: Vec<String> = Vec::new();
    if !diagram.relationships.is_empty() {
        ordered_ids.push(diagram.relationships[0].entity1.clone());
    }
    for rel in &diagram.relationships {
        if !ordered_ids.contains(&rel.entity1) {
            ordered_ids.push(rel.entity1.clone());
        }
        if !ordered_ids.contains(&rel.entity2) {
            ordered_ids.push(rel.entity2.clone());
        }
    }
    // Add any entities not referenced by relationships
    for ent in &diagram.entities {
        if !ordered_ids.contains(&ent.id) {
            ordered_ids.push(ent.id.clone());
        }
    }

    // Look up entity by id
    let entity_for = |id: &str| -> Option<&crate::types::ErEntity> {
        diagram.entities.iter().find(|e| e.id == id)
    };

    // Look up label
    let label_for = |id: &str| -> String {
        entity_for(id)
            .map(|e| e.label.clone())
            .unwrap_or_else(|| id.to_string())
    };

    // Format attributes for each entity
    let attrs_for: Vec<Vec<String>> = ordered_ids
        .iter()
        .map(|id| {
            entity_for(id)
                .map(|e| format_entity_attrs(e))
                .unwrap_or_default()
        })
        .collect();

    // Find relationship between two adjacent entities (if any)
    let rel_between = |id1: &str, id2: &str| -> Option<&crate::types::ErRelationship> {
        diagram.relationships.iter().find(|r| {
            (r.entity1 == id1 && r.entity2 == id2) || (r.entity1 == id2 && r.entity2 == id1)
        })
    };

    // For each adjacent pair, compute the relationship connector string and label
    struct Gap {
        label: String,
        connector: String,
        width: usize,
    }

    let mut gaps: Vec<Gap> = Vec::new();
    for i in 0..ordered_ids.len().saturating_sub(1) {
        let id1 = &ordered_ids[i];
        let id2 = &ordered_ids[i + 1];
        if let Some(rel) = rel_between(id1, id2) {
            // Determine direction: if entity1 matches id1, draw card1--card2; otherwise reverse
            let (c1, c2) = if rel.entity1 == *id1 {
                (rel.cardinality1, rel.cardinality2)
            } else {
                (rel.cardinality2, rel.cardinality1)
            };
            let card1 = cardinality_to_str_left(c1, use_ascii);
            let card2 = cardinality_to_str_right(c2, use_ascii);
            let card1_len = card1.chars().count();
            let is_identifying = rel.identifying;
            let fill_char = if is_identifying {
                if use_ascii {
                    '-'
                } else {
                    '─'
                }
            } else {
                '.'
            };

            // The label (with padding) must fit over the line portion only
            let label_padded = format!(" {} ", rel.label);
            let label_padded_len = label_padded.chars().count();
            // Minimum 2 line chars (the base "--" or "..")
            let line_len = label_padded_len.max(2);

            // Build the connector: card1 + line_chars + card2
            let line_fill: String = std::iter::repeat(fill_char).take(line_len).collect();
            let connector = format!("{}{}{}", card1, line_fill, card2);
            let width = connector.chars().count();

            // Build the label string: centered over the line portion, offset by card1_len
            let label_total_pad = line_len.saturating_sub(label_padded_len);
            let label_left_pad = label_total_pad / 2;
            let label = format!(
                "{}{}",
                " ".repeat(card1_len + label_left_pad),
                label_padded.trim_end()
            );

            gaps.push(Gap {
                label,
                connector,
                width,
            });
        } else {
            // No relationship — just spacing
            gaps.push(Gap {
                label: String::new(),
                connector: String::new(),
                width: 6,
            });
        }
    }

    // Compute entity box widths (accounting for label and attributes)
    let entity_widths: Vec<usize> = ordered_ids
        .iter()
        .enumerate()
        .map(|(idx, id)| {
            let label_len = label_for(id).len();
            let attr_max = attrs_for[idx].iter().map(|s| s.len()).max().unwrap_or(0);
            label_len.max(attr_max) + 4
        })
        .collect();

    // Compute entity box heights
    // No attrs: 3 rows (top, name, bottom)
    // With attrs: 3 + num_attrs + 1 rows (top, name, divider, attrs..., bottom)
    let entity_heights: Vec<usize> = attrs_for
        .iter()
        .map(|attrs| if attrs.is_empty() { 3 } else { 4 + attrs.len() })
        .collect();

    // Compute positions — each entity box is placed after the previous box + gap
    let mut positions: Vec<usize> = Vec::new();
    let mut cur_x = 0usize;
    for (i, w) in entity_widths.iter().enumerate() {
        positions.push(cur_x);
        if i < gaps.len() {
            cur_x += w + gaps[i].width;
        }
    }

    let total_w = positions.last().unwrap_or(&0) + entity_widths.last().unwrap_or(&0) + 3;
    let max_height = *entity_heights.iter().max().unwrap_or(&3);
    let total_h = max_height + 1;

    let mut canvas = mk_canvas(total_w, total_h);

    // Draw entity boxes with attributes
    for (i, id) in ordered_ids.iter().enumerate() {
        let label = label_for(id);
        let x = positions[i] as i32;
        let w = entity_widths[i] as i32;
        let attrs = &attrs_for[i];

        if attrs.is_empty() {
            // Simple 3-row box: top, name, bottom
            draw_simple_box(&mut canvas, x, 0, w, 3, &label, use_ascii);
        } else {
            // Box with attributes: top, name, divider, attrs..., bottom
            // Top border
            set_char(&mut canvas, x, 0, tl);
            for j in 1..(w - 1) {
                set_char(&mut canvas, x + j, 0, h_line);
            }
            set_char(&mut canvas, x + w - 1, 0, tr);

            // Name row
            set_char(&mut canvas, x, 1, v_line);
            draw_text(&mut canvas, x + 2, 1, &label);
            set_char(&mut canvas, x + w - 1, 1, v_line);

            // Divider row
            set_char(&mut canvas, x, 2, div_l);
            for j in 1..(w - 1) {
                set_char(&mut canvas, x + j, 2, h_line);
            }
            set_char(&mut canvas, x + w - 1, 2, div_r);

            // Attribute rows
            for (ai, attr) in attrs.iter().enumerate() {
                let y = 3 + ai as i32;
                set_char(&mut canvas, x, y, v_line);
                draw_text(&mut canvas, x + 2, y, attr);
                set_char(&mut canvas, x + w - 1, y, v_line);
            }

            // Bottom border
            let bot_y = 3 + attrs.len() as i32;
            set_char(&mut canvas, x, bot_y, bl);
            for j in 1..(w - 1) {
                set_char(&mut canvas, x + j, bot_y, h_line);
            }
            set_char(&mut canvas, x + w - 1, bot_y, br);
        }

        // Draw the gap connector to the right of this box
        if i < gaps.len() {
            let gap = &gaps[i];
            let gap_x = x + w;
            // Row 0 (top border line): draw the label (pre-offset to center over line portion)
            draw_text(&mut canvas, gap_x, 0, &gap.label);
            // Row 1 (name row): draw the connector
            draw_text(&mut canvas, gap_x, 1, &gap.connector);
        }
    }

    Ok(canvas_to_string(&canvas))
}

/// Left-side cardinality symbol (entity is to the left of the connector)
fn cardinality_to_str_left(card: Cardinality, use_ascii: bool) -> &'static str {
    if use_ascii {
        match card {
            Cardinality::One => "||",
            Cardinality::ZeroOne => "o|",
            Cardinality::Many => "}|",
            Cardinality::ZeroMany => "}o",
        }
    } else {
        match card {
            Cardinality::One => "║",
            Cardinality::ZeroOne => "o│",
            Cardinality::Many => "╟",
            Cardinality::ZeroMany => "o╟",
        }
    }
}

/// Right-side cardinality symbol (entity is to the right of the connector)
fn cardinality_to_str_right(card: Cardinality, use_ascii: bool) -> &'static str {
    if use_ascii {
        match card {
            Cardinality::One => "||",
            Cardinality::ZeroOne => "|o",
            Cardinality::Many => "|{",
            Cardinality::ZeroMany => "o{",
        }
    } else {
        match card {
            Cardinality::One => "║",
            Cardinality::ZeroOne => "o│",
            Cardinality::Many => "╟",
            Cardinality::ZeroMany => "o╟",
        }
    }
}

fn draw_simple_box(
    canvas: &mut super::types::Canvas,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &str,
    use_ascii: bool,
) {
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };

    // Top border
    set_char(canvas, x, y, tl);
    for i in 1..(w - 1) {
        set_char(canvas, x + i, y, h_line);
    }
    set_char(canvas, x + w - 1, y, tr);

    // Middle row
    set_char(canvas, x, y + 1, v_line);
    let label_x = x + (w - label.len() as i32) / 2;
    draw_text(canvas, label_x, y + 1, label);
    set_char(canvas, x + w - 1, y + 1, v_line);

    // Bottom border
    set_char(canvas, x, y + h - 1, bl);
    for i in 1..(w - 1) {
        set_char(canvas, x + i, y + h - 1, h_line);
    }
    set_char(canvas, x + w - 1, y + h - 1, br);
}
