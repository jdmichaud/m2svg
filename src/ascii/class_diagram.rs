//! Class diagram ASCII rendering

use crate::types::{ClassDiagram, ClassMember, RelationshipType, Visibility};
use super::types::AsciiConfig;
use super::canvas::{mk_canvas, canvas_to_string, set_char, draw_text};

/// Render a class diagram to ASCII
pub fn render_class_ascii(diagram: &ClassDiagram, config: &AsciiConfig) -> Result<String, String> {
    if diagram.classes.is_empty() {
        return Ok(String::new());
    }
    
    let use_ascii = config.use_ascii;
    let padding = 1;
    
    // Calculate dimensions for each class
    let mut class_boxes: Vec<ClassBox> = Vec::new();
    
    for cls in &diagram.classes {
        // Calculate width based on widest line
        let header_width = cls.label.len() + 2 * padding;
        
        let attr_lines: Vec<String> = cls.attributes
            .iter()
            .map(|m| format_member(m))
            .collect();
        let method_lines: Vec<String> = cls.methods
            .iter()
            .map(|m| format_member(m))
            .collect();
        
        let attr_width = attr_lines.iter().map(|s| s.len()).max().unwrap_or(0) + 2 * padding;
        let method_width = method_lines.iter().map(|s| s.len()).max().unwrap_or(0) + 2 * padding;
        
        let inner_width = header_width.max(attr_width).max(method_width);
        let box_width = inner_width + 2; // +2 for borders
        
        // Calculate height based on what sections are present (like TypeScript)
        // If no attrs and no methods: just header (2 rows: top border + header + bottom border = 3)
        // If no methods: header + attrs (top + header + divider + attrs + bottom)
        // Full: header + attrs + methods
        let box_height = if attr_lines.is_empty() && method_lines.is_empty() {
            3 // top + header + bottom
        } else if method_lines.is_empty() {
            // top + header + divider + attrs + bottom
            1 + 1 + 1 + attr_lines.len() + 1
        } else {
            // top + header + divider + attrs + divider + methods + bottom
            1 + 1 + 1 + attr_lines.len().max(1) + 1 + method_lines.len() + 1
        };
        
        class_boxes.push(ClassBox {
            id: cls.id.clone(),
            label: cls.label.clone(),
            _annotation: cls.annotation.clone(),
            attr_lines,
            method_lines,
            width: box_width,
            height: box_height,
            x: 0,
            y: 0,
        });
    }
    
    // Check for inheritance relationships to determine vertical layout
    let has_inheritance = diagram.relationships.iter().any(|r| 
        matches!(r.rel_type, RelationshipType::Inheritance | RelationshipType::Realization)
    );
    
    if has_inheritance && !diagram.relationships.is_empty() {
        // Find parent classes (sources of inheritance)
        let mut parent_ids = std::collections::HashSet::new();
        let mut child_ids = std::collections::HashSet::new();
        
        for rel in &diagram.relationships {
            if matches!(rel.rel_type, RelationshipType::Inheritance | RelationshipType::Realization) {
                // In class diagrams, inheritance is typically shown as Child --|> Parent
                // so 'from' is the child, 'to' is the parent
                parent_ids.insert(rel.to.clone());
                child_ids.insert(rel.from.clone());
            }
        }
        
        // Layout: parents at top, children below with connecting lines
        let spacing_v = 3; // Vertical spacing for relationship line
        let spacing_h = 6; // Horizontal spacing
        
        // Separate into parents and children
        let (mut parents, mut children): (Vec<_>, Vec<_>) = class_boxes.iter_mut()
            .partition(|cb| parent_ids.contains(&cb.id) && !child_ids.contains(&cb.id));
        
        // If a class is both parent and child (in a chain), treat as child for simplicity
        
        // Position parents at top
        let mut cur_x = 0;
        for cb in &mut parents {
            cb.x = cur_x as i32;
            cb.y = 0;
            cur_x += cb.width + spacing_h;
        }
        
        // Find max height of parents
        let parent_height = parents.iter().map(|cb| cb.height).max().unwrap_or(0);
        let child_y = (parent_height + spacing_v) as i32;
        
        // Position children below
        cur_x = 0;
        for cb in &mut children {
            cb.x = cur_x as i32;
            cb.y = child_y;
            cur_x += cb.width + spacing_h;
        }
        
        // Calculate total canvas size
        let total_w = class_boxes.iter().map(|cb| cb.x as usize + cb.width).max().unwrap_or(1);
        let total_h = class_boxes.iter().map(|cb| cb.y as usize + cb.height).max().unwrap_or(1) + 2;
        
        let mut canvas = mk_canvas(total_w, total_h);
        
        // Draw each class box
        for cb in &class_boxes {
            draw_class_box(&mut canvas, cb, use_ascii);
        }
        
        // Draw inheritance arrows between parent and child
        for rel in &diagram.relationships {
            if matches!(rel.rel_type, RelationshipType::Inheritance | RelationshipType::Realization) {
                // Find parent and child boxes
                let parent_box = class_boxes.iter().find(|cb| cb.id == rel.to);
                let child_box = class_boxes.iter().find(|cb| cb.id == rel.from);
                
                if let (Some(parent), Some(child)) = (parent_box, child_box) {
                    // Draw vertical line from child top to parent bottom
                    let parent_center_x = parent.x + (parent.width as i32 / 2);
                    let parent_bottom_y = parent.y + parent.height as i32 - 1;
                    let child_top_y = child.y;
                    
                    // Draw the arrow: ^ at parent bottom, -- below it, | connecting to child
                    // Arrow head is centered, -- is one char left and one at center
                    // The vertical connector aligns with the left dash
                    let arrow_y = parent_bottom_y + 1;
                    set_char(&mut canvas, parent_center_x, arrow_y, '^');
                    set_char(&mut canvas, parent_center_x - 1, arrow_y + 1, '-');
                    set_char(&mut canvas, parent_center_x, arrow_y + 1, '-');
                    
                    // Draw vertical line at parent_center_x - 1 (aligned with left -)
                    for y in (arrow_y + 2)..child_top_y {
                        set_char(&mut canvas, parent_center_x - 1, y, '|');
                    }
                }
            }
        }
        
        Ok(canvas_to_string(&canvas))
    } else {
        // Simple horizontal layout
        let spacing = 4;
        let mut cur_x = 0;
        for cb in &mut class_boxes {
            cb.x = cur_x as i32;
            cb.y = 0;
            cur_x += cb.width + spacing;
        }
        
        let total_w = class_boxes.iter().map(|cb| cb.x as usize + cb.width).max().unwrap_or(1);
        let total_h = class_boxes.iter().map(|cb| cb.y as usize + cb.height).max().unwrap_or(1) + 2;
        
        let mut canvas = mk_canvas(total_w, total_h);
        
        for cb in &class_boxes {
            draw_class_box(&mut canvas, cb, use_ascii);
        }
        
        Ok(canvas_to_string(&canvas))
    }
}

struct ClassBox {
    id: String,
    label: String,
    _annotation: Option<String>,
    attr_lines: Vec<String>,
    method_lines: Vec<String>,
    width: usize,
    height: usize,
    x: i32,
    y: i32,
}

fn format_member(member: &ClassMember) -> String {
    let vis = match member.visibility {
        Visibility::Public => "+",
        Visibility::Private => "-",
        Visibility::Protected => "#",
        Visibility::Package => "~",
        Visibility::None => "",
    };
    
    if let Some(ref t) = member.member_type {
        format!("{}{}: {}", vis, member.name, t)
    } else {
        format!("{}{}", vis, member.name)
    }
}

fn draw_class_box(canvas: &mut super::types::Canvas, cb: &ClassBox, use_ascii: bool) {
    let (h_line, v_line, tl, tr, bl, br, div_l, div_r) = if use_ascii {
        ('-', '|', '+', '+', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘', '├', '┤')
    };
    
    let x = cb.x;
    let y = cb.y;
    let w = cb.width as i32;
    
    // Top border
    set_char(canvas, x, y, tl);
    for i in 1..(w - 1) {
        set_char(canvas, x + i, y, h_line);
    }
    set_char(canvas, x + w - 1, y, tr);
    
    // Header row - label is left-aligned with 1 char padding (space after |)
    let header_y = y + 1;
    set_char(canvas, x, header_y, v_line);
    set_char(canvas, x + 1, header_y, ' '); // Padding space
    let label_x = x + 2; // Left-aligned with 1 space padding after border
    draw_text(canvas, label_x, header_y, &cb.label);
    set_char(canvas, x + w - 1, header_y, v_line);
    
    let mut cur_y = header_y + 1;
    
    // Handle based on what sections exist
    let has_attrs = !cb.attr_lines.is_empty();
    let has_methods = !cb.method_lines.is_empty();
    
    if has_attrs || has_methods {
        // Divider after header
        set_char(canvas, x, cur_y, div_l);
        for i in 1..(w - 1) {
            set_char(canvas, x + i, cur_y, h_line);
        }
        set_char(canvas, x + w - 1, cur_y, div_r);
        cur_y += 1;
        
        // Attributes section
        if has_attrs {
            for line in &cb.attr_lines {
                set_char(canvas, x, cur_y, v_line);
                draw_text(canvas, x + 2, cur_y, line);
                set_char(canvas, x + w - 1, cur_y, v_line);
                cur_y += 1;
            }
        } else if has_methods {
            // Empty attrs row if we have methods but no attrs
            set_char(canvas, x, cur_y, v_line);
            set_char(canvas, x + w - 1, cur_y, v_line);
            cur_y += 1;
        }
        
        // Methods section (only if we have methods)
        if has_methods {
            // Divider before methods
            set_char(canvas, x, cur_y, div_l);
            for i in 1..(w - 1) {
                set_char(canvas, x + i, cur_y, h_line);
            }
            set_char(canvas, x + w - 1, cur_y, div_r);
            cur_y += 1;
            
            for line in &cb.method_lines {
                set_char(canvas, x, cur_y, v_line);
                draw_text(canvas, x + 2, cur_y, line);
                set_char(canvas, x + w - 1, cur_y, v_line);
                cur_y += 1;
            }
        }
    }
    
    // Bottom border
    set_char(canvas, x, cur_y, bl);
    for i in 1..(w - 1) {
        set_char(canvas, x + i, cur_y, h_line);
    }
    set_char(canvas, x + w - 1, cur_y, br);
}
