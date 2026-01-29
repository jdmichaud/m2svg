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
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    
    // For simple ER diagrams, render relationships inline
    if diagram.relationships.len() == 1 && diagram.entities.len() <= 2 {
        return render_simple_er(diagram, config);
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
            id: entity.id.clone(),
            label: entity.label.clone(),
            attr_lines,
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
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
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
    let card1 = cardinality_to_str(rel.cardinality1);
    let card2 = cardinality_to_str(rel.cardinality2);
    let line_style = if rel.identifying { "--" } else { ".." };
    
    // Total width - the label and rel_str share the same space
    let rel_str = format!("{}{}{}",card1, line_style, card2);
    let middle_width = rel.label.len().max(rel_str.len());
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

fn cardinality_to_str(card: Cardinality) -> &'static str {
    match card {
        Cardinality::One => "||",
        Cardinality::ZeroOne => "o|",
        Cardinality::Many => "}|",
        Cardinality::ZeroMany => "o{",
    }
}

struct EntityBox {
    id: String,
    label: String,
    attr_lines: Vec<String>,
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
