//! ER diagram ASCII rendering

use crate::types::{ErDiagram, Cardinality};
use super::types::AsciiConfig;
use super::canvas::{mk_canvas, canvas_to_string, set_char, draw_text};

/// Render an ER diagram to ASCII
pub fn render_er_ascii(diagram: &ErDiagram, config: &AsciiConfig) -> Result<String, String> {
    if diagram.entities.is_empty() && diagram.relationships.is_empty() {
        return Ok(String::new());
    }
    
    let use_ascii = config.use_ascii;
    
    // Box-drawing characters
    let (_h_line, _v_line, _tl, _tr, _bl, _br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    // Divider T-junctions
    let (_div_l, _div_r) = if use_ascii { ('+', '+') } else { ('├', '┤') };
    
    // For simple ER diagrams without attributes, render relationships inline
    let has_attributes = diagram.entities.iter().any(|e| !e.attributes.is_empty());
    if diagram.relationships.len() == 1 && diagram.entities.len() <= 2 && !has_attributes {
        return render_simple_er(diagram, config);
    }
    
    // For ER diagrams with attributes, render inline but with attribute rows
    if diagram.relationships.len() == 1 && diagram.entities.len() <= 2 && has_attributes {
        return render_er_with_attributes(diagram, config);
    }
    
    // Calculate dimensions for each entity
    let mut entity_boxes: Vec<EntityBox> = Vec::new();
    
    for entity in &diagram.entities {
        let header_width = entity.label.len() + 2;
        
        let attr_lines: Vec<String> = entity.attributes
            .iter()
            .map(|a| {
                let key_str = a.keys.iter()
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
        
        let attr_width = attr_lines.iter().map(|s| s.len()).max().unwrap_or(0) + 2;
        let box_width = header_width.max(attr_width) + 2;
        let box_height = 3 + attr_lines.len(); // header + attrs + borders
        
        entity_boxes.push(EntityBox {
            _id: entity.id.clone(),
            label: entity.label.clone(),
            _attr_lines: attr_lines,
            width: box_width,
            height: box_height.max(3),
            x: 0,
            y: 0,
        });
    }
    
    // Simple layout: stack entities horizontally
    let spacing = 6;
    let mut cur_x = 0;
    for eb in &mut entity_boxes {
        eb.x = cur_x as i32;
        eb.y = 0;
        cur_x += eb.width + spacing;
    }
    
    let total_w = entity_boxes.iter().map(|eb| eb.x as usize + eb.width).max().unwrap_or(1) + 2;
    let total_h = entity_boxes.iter().map(|eb| eb.y as usize + eb.height).max().unwrap_or(1) + 2;
    
    let mut canvas = mk_canvas(total_w, total_h);
    
    // Draw entity boxes
    for eb in &entity_boxes {
        draw_entity_box(&mut canvas, eb, use_ascii);
    }
    
    Ok(canvas_to_string(&canvas))
}

/// Render a simple ER diagram with one relationship inline
fn render_simple_er(diagram: &ErDiagram, config: &AsciiConfig) -> Result<String, String> {
    let use_ascii = config.use_ascii;
    let (_h_line, _v_line, _tl, _tr, _bl, _br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    
    let rel = &diagram.relationships[0];
    
    // Find entities
    let e1_label = diagram.entities.iter()
        .find(|e| e.id == rel.entity1)
        .map(|e| e.label.as_str())
        .unwrap_or(&rel.entity1);
    let e2_label = diagram.entities.iter()
        .find(|e| e.id == rel.entity2)
        .map(|e| e.label.as_str())
        .unwrap_or(&rel.entity2);
    
    // Box dimensions
    let e1_width = e1_label.len() + 4;
    let e2_width = e2_label.len() + 4;
    let box_height = 3;
    
    // Cardinality symbols
    let card1 = cardinality_to_str(rel.cardinality1, use_ascii);
    let card2 = cardinality_to_str(rel.cardinality2, use_ascii);
    let line_style = if rel.identifying { if use_ascii { "--" } else { "───" } } else { ".." };
    
    // Total width - the label and rel_str share the same space
    let rel_str = format!("{}{}{}",card1, line_style, card2);
    let middle_width = rel.label.chars().count().max(rel_str.chars().count());
    let total_w = e1_width + middle_width + e2_width + 1;
    let total_h = box_height;
    
    let mut canvas = mk_canvas(total_w, total_h);
    
    // Draw first entity box
    let e1_x = 0i32;
    draw_simple_box(&mut canvas, e1_x, 0, e1_width as i32, box_height as i32, e1_label, use_ascii);
    
    // Draw relationship label on top line
    let rel_x = e1_x + e1_width as i32;
    draw_text(&mut canvas, rel_x, 0, &rel.label);
    
    // Draw cardinality and line on middle line
    draw_text(&mut canvas, rel_x, 1, &rel_str);
    
    // Draw second entity box - right after the middle section
    let e2_x = rel_x + middle_width as i32;
    draw_simple_box(&mut canvas, e2_x, 0, e2_width as i32, box_height as i32, e2_label, use_ascii);
    
    Ok(canvas_to_string(&canvas))
}

/// Render an ER diagram with attributes - relationship inline with attribute rows below
fn render_er_with_attributes(diagram: &ErDiagram, config: &AsciiConfig) -> Result<String, String> {
    let use_ascii = config.use_ascii;
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    let (div_l, div_r) = if use_ascii { ('+', '+') } else { ('├', '┤') };
    
    let rel = &diagram.relationships[0];
    
    // Find entities and their attributes
    let e1 = diagram.entities.iter()
        .find(|e| e.id == rel.entity1);
    let e2 = diagram.entities.iter()
        .find(|e| e.id == rel.entity2);
    
    let e1_label = e1.map(|e| e.label.as_str()).unwrap_or(&rel.entity1);
    let e2_label = e2.map(|e| e.label.as_str()).unwrap_or(&rel.entity2);
    
    // Format attribute lines with keys - keys come BEFORE type for display
    let e1_attrs: Vec<String> = e1.map(|e| {
        e.attributes.iter().map(|a| {
            let key_prefix = a.keys.iter()
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
        }).collect()
    }).unwrap_or_default();
    
    let e2_attrs: Vec<String> = e2.map(|e| {
        e.attributes.iter().map(|a| {
            let key_prefix = a.keys.iter()
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
        }).collect()
    }).unwrap_or_default();
    
    // Cardinality symbols
    let card1 = cardinality_to_str(rel.cardinality1, use_ascii);
    let card2 = cardinality_to_str(rel.cardinality2, use_ascii);
    let line_style = if rel.identifying { if use_ascii { "--" } else { "───" } } else { ".." };
    let rel_str = format!("{}{}{}", card1, line_style, card2);
    
    // Middle section: max of rel_str length and truncated label
    let gap_width = rel_str.chars().count();  // The gap is exactly the relationship string length
    let label_display: String = rel.label.chars().take(gap_width).collect();
    
    // Decide row placement based on attribute count
    // If e1 has 2+ attrs, put label on divider (row 2) and rel_str on first attr (row 3)
    // Otherwise put label on name row (row 1) and rel_str on divider (row 2)
    let label_on_divider = e1_attrs.len() >= 2;
    
    // Calculate entity box widths
    let e1_attr_max = e1_attrs.iter().map(|s| s.len()).max().unwrap_or(0);
    let e2_attr_max = e2_attrs.iter().map(|s| s.len()).max().unwrap_or(0);
    let e1_inner = (e1_label.len()).max(e1_attr_max);
    let e2_inner = (e2_label.len()).max(e2_attr_max);
    let e1_width = e1_inner + 4; // +2 padding +2 borders
    let e2_width = e2_inner + 4;
    
    // Position entities
    let e1_x = 0i32;
    let e2_x = e1_width as i32 + gap_width as i32;
    
    // Calculate height
    let e1_total_rows = 3 + e1_attrs.len(); // top + name + divider + attrs + bottom
    let e2_total_rows = 3 + e2_attrs.len();
    let total_h = e1_total_rows.max(e2_total_rows) + 1;
    let total_w = e2_x as usize + e2_width + 4;
    
    let mut canvas = mk_canvas(total_w, total_h);
    
    // Row 0: Top borders  
    set_char(&mut canvas, e1_x, 0, tl);
    for i in 1..(e1_width as i32 - 1) {
        set_char(&mut canvas, e1_x + i, 0, h_line);
    }
    set_char(&mut canvas, e1_x + e1_width as i32 - 1, 0, tr);
    
    set_char(&mut canvas, e2_x, 0, tl);
    for i in 1..(e2_width as i32 - 1) {
        set_char(&mut canvas, e2_x + i, 0, h_line);
    }
    set_char(&mut canvas, e2_x + e2_width as i32 - 1, 0, tr);
    
    // Row 1: Entity names - label only if !label_on_divider
    set_char(&mut canvas, e1_x, 1, v_line);
    draw_text(&mut canvas, e1_x + 2, 1, e1_label);
    set_char(&mut canvas, e1_x + e1_width as i32 - 1, 1, v_line);
    
    if !label_on_divider {
        draw_text(&mut canvas, e1_x + e1_width as i32, 1, &label_display);
    }
    
    set_char(&mut canvas, e2_x, 1, v_line);
    draw_text(&mut canvas, e2_x + 2, 1, e2_label);
    set_char(&mut canvas, e2_x + e2_width as i32 - 1, 1, v_line);
    
    // Row 2: Divider - label if label_on_divider, rel_str if !label_on_divider
    set_char(&mut canvas, e1_x, 2, div_l);  // ├
    for i in 1..(e1_width as i32 - 1) {
        set_char(&mut canvas, e1_x + i, 2, h_line);
    }
    set_char(&mut canvas, e1_x + e1_width as i32 - 1, 2, div_r);  // ┤
    
    if label_on_divider {
        draw_text(&mut canvas, e1_x + e1_width as i32, 2, &label_display);
    } else {
        draw_text(&mut canvas, e1_x + e1_width as i32, 2, &rel_str);
    }
    
    set_char(&mut canvas, e2_x, 2, div_l);  // ├
    for i in 1..(e2_width as i32 - 1) {
        set_char(&mut canvas, e2_x + i, 2, h_line);
    }
    set_char(&mut canvas, e2_x + e2_width as i32 - 1, 2, div_r);  // ┤
    
    // Attribute rows for e1 - also draw rel_str on first attr row if label_on_divider
    for (i, attr) in e1_attrs.iter().enumerate() {
        let y = 3 + i as i32;
        set_char(&mut canvas, e1_x, y, v_line);
        draw_text(&mut canvas, e1_x + 2, y, attr);
        set_char(&mut canvas, e1_x + e1_width as i32 - 1, y, v_line);
        
        // Draw rel_str on first attribute row when label is on divider
        if i == 0 && label_on_divider {
            draw_text(&mut canvas, e1_x + e1_width as i32, y, &rel_str);
        }
    }
    
    // Attribute rows for e2
    for (i, attr) in e2_attrs.iter().enumerate() {
        let y = 3 + i as i32;
        set_char(&mut canvas, e2_x, y, v_line);
        draw_text(&mut canvas, e2_x + 2, y, attr);
        set_char(&mut canvas, e2_x + e2_width as i32 - 1, y, v_line);
    }
    
    // Bottom border for e1
    let _e1_bottom_y = 3 + e1_attrs.len().max(1) as i32 - 1;
    if e1_attrs.is_empty() {
        // No attrs - bottom comes right after divider
        set_char(&mut canvas, e1_x, 3, bl);
        for i in 1..(e1_width as i32 - 1) {
            set_char(&mut canvas, e1_x + i, 3, h_line);
        }
        set_char(&mut canvas, e1_x + e1_width as i32 - 1, 3, br);
    } else {
        let y = 3 + e1_attrs.len() as i32;
        set_char(&mut canvas, e1_x, y, bl);
        for i in 1..(e1_width as i32 - 1) {
            set_char(&mut canvas, e1_x + i, y, h_line);
        }
        set_char(&mut canvas, e1_x + e1_width as i32 - 1, y, br);
    }
    
    // Bottom border for e2
    if e2_attrs.is_empty() {
        set_char(&mut canvas, e2_x, 3, bl);
        for i in 1..(e2_width as i32 - 1) {
            set_char(&mut canvas, e2_x + i, 3, h_line);
        }
        set_char(&mut canvas, e2_x + e2_width as i32 - 1, 3, br);
    } else {
        let y = 3 + e2_attrs.len() as i32;
        set_char(&mut canvas, e2_x, y, bl);
        for i in 1..(e2_width as i32 - 1) {
            set_char(&mut canvas, e2_x + i, y, h_line);
        }
        set_char(&mut canvas, e2_x + e2_width as i32 - 1, y, br);
    }
    
    Ok(canvas_to_string(&canvas))
}

fn cardinality_to_str(card: Cardinality, use_ascii: bool) -> &'static str {
    if use_ascii {
        match card {
            Cardinality::One => "||",
            Cardinality::ZeroOne => "o|",
            Cardinality::Many => "}|",
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

struct EntityBox {
    _id: String,
    label: String,
    _attr_lines: Vec<String>,
    width: usize,
    height: usize,
    x: i32,
    y: i32,
}

fn draw_entity_box(canvas: &mut super::types::Canvas, eb: &EntityBox, use_ascii: bool) {
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    
    let x = eb.x;
    let y = eb.y;
    let w = eb.width as i32;
    let h = eb.height as i32;
    
    // Top border
    set_char(canvas, x, y, tl);
    for i in 1..(w - 1) {
        set_char(canvas, x + i, y, h_line);
    }
    set_char(canvas, x + w - 1, y, tr);
    
    // Label row
    set_char(canvas, x, y + 1, v_line);
    let label_x = x + 2;
    draw_text(canvas, label_x, y + 1, &eb.label);
    set_char(canvas, x + w - 1, y + 1, v_line);
    
    // Bottom border
    set_char(canvas, x, y + h - 1, bl);
    for i in 1..(w - 1) {
        set_char(canvas, x + i, y + h - 1, h_line);
    }
    set_char(canvas, x + w - 1, y + h - 1, br);
}

fn draw_simple_box(canvas: &mut super::types::Canvas, x: i32, y: i32, w: i32, h: i32, label: &str, use_ascii: bool) {
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
