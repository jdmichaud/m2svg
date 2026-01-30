//! Class diagram ASCII rendering

use crate::types::{ClassDiagram, ClassMember, RelationshipType, Visibility};
use super::types::AsciiConfig;
use super::canvas::{mk_canvas, canvas_to_string, set_char, draw_text};
use std::collections::{HashMap, HashSet};

/// Render a class diagram to ASCII
pub fn render_class_ascii(diagram: &ClassDiagram, config: &AsciiConfig) -> Result<String, String> {
    if diagram.classes.is_empty() {
        return Ok(String::new());
    }
    
    let use_ascii = config.use_ascii;
    let padding = 1;
    let h_gap = 4;  // horizontal gap between class boxes
    let v_gap_normal = 3;  // vertical gap for single child inheritance
    let v_gap_fanout = 4;  // vertical gap when parent has multiple children (for centered layout)
    
    // Build box dimensions for each class
    let mut class_boxes: HashMap<String, ClassBox> = HashMap::new();
    
    for cls in &diagram.classes {
        let has_annotation = cls.annotation.is_some();
        let annotation_str = cls.annotation.as_ref().map(|a| format!("<<{}>>", a));
        
        // Calculate width based on widest line
        let annotation_width = annotation_str.as_ref().map(|s| s.len()).unwrap_or(0) + 2 * padding;
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
        
        let inner_width = header_width.max(attr_width).max(method_width).max(annotation_width);
        let box_width = inner_width + 2; // +2 for borders
        
        // Calculate height based on what sections are present
        // Header section includes annotation (if any) + class name
        let header_height = if has_annotation { 2 } else { 1 };
        
        let box_height = if attr_lines.is_empty() && method_lines.is_empty() {
            1 + header_height + 1 // top + header(s) + bottom
        } else if method_lines.is_empty() {
            1 + header_height + 1 + attr_lines.len() + 1 // top + header + divider + attrs + bottom
        } else {
            1 + header_height + 1 + attr_lines.len().max(1) + 1 + method_lines.len() + 1
        };
        
        class_boxes.insert(cls.id.clone(), ClassBox {
            _id: cls.id.clone(),
            label: cls.label.clone(),
            annotation: cls.annotation.clone(),
            attr_lines,
            method_lines,
            width: box_width,
            height: box_height,
            x: 0,
            y: 0,
        });
    }
    
    // Assign levels using topological sort - all relationships cause level separation
    // "from" nodes are placed above "to" nodes in general
    // For inheritance/realization with marker_at_from, parent is 'from', child is 'to'
    let mut parents: HashMap<String, HashSet<String>> = HashMap::new();
    let mut children: HashMap<String, HashSet<String>> = HashMap::new();
    
    for rel in &diagram.relationships {
        // Determine parent (at top) and child (at bottom)
        // For inheritance with marker_at_from (A <|-- B): A is parent, B is child (from=A, to=B)
        // For inheritance with marker_at_to (A --|> B): B is parent, A is child (from=A, to=B, need to swap)
        let is_hierarchical = matches!(rel.rel_type, RelationshipType::Inheritance | RelationshipType::Realization);
        
        let (parent_id, child_id) = if is_hierarchical && !rel.marker_at_from {
            // marker at 'to' side means 'to' is the parent
            (rel.to.clone(), rel.from.clone())
        } else {
            // Default: 'from' is parent/source, 'to' is child/target
            (rel.from.clone(), rel.to.clone())
        };
        
        parents.entry(child_id.clone()).or_default().insert(parent_id.clone());
        children.entry(parent_id.clone()).or_default().insert(child_id);
    }
    
    // BFS from roots to assign levels
    let mut level: HashMap<String, usize> = HashMap::new();
    let roots: Vec<_> = diagram.classes.iter()
        .filter(|c| parents.get(&c.id).map(|s| s.is_empty()).unwrap_or(true))
        .map(|c| c.id.clone())
        .collect();
    
    let mut queue: Vec<String> = roots.clone();
    for id in &roots {
        level.insert(id.clone(), 0);
    }
    
    let level_cap = diagram.classes.len();
    let mut qi = 0;
    while qi < queue.len() {
        let id = queue[qi].clone();
        qi += 1;
        if let Some(child_set) = children.get(&id) {
            for child_id in child_set {
                let new_level = level.get(&id).copied().unwrap_or(0) + 1;
                if new_level > level_cap { continue; }
                if !level.contains_key(child_id) || level.get(child_id).copied().unwrap_or(0) < new_level {
                    level.insert(child_id.clone(), new_level);
                    queue.push(child_id.clone());
                }
            }
        }
    }
    
    // Assign unconnected classes to level 0
    for cls in &diagram.classes {
        level.entry(cls.id.clone()).or_insert(0);
    }
    
    // Group classes by level
    let max_level = level.values().copied().max().unwrap_or(0);
    let mut level_groups: Vec<Vec<String>> = vec![Vec::new(); max_level + 1];
    for cls in &diagram.classes {
        let lv = level.get(&cls.id).copied().unwrap_or(0);
        level_groups[lv].push(cls.id.clone());
    }
    
    // Determine which levels need extra gap (because parent has multiple children = fan-out)
    let mut level_has_fanout: Vec<bool> = vec![false; max_level + 1];
    for (parent_id, child_set) in &children {
        if child_set.len() > 1 {
            // This parent has a fan-out, mark its level as needing extra gap
            if let Some(&lv) = level.get(parent_id) {
                level_has_fanout[lv] = true;
            }
        }
    }
    
    // Position classes bottom-up to center parents over children
    // First, compute Y positions for each level (top-down for Y)
    let mut level_y: Vec<usize> = Vec::new();
    let mut current_y: usize = 0;
    
    for lv in 0..=max_level {
        let group = &level_groups[lv];
        level_y.push(current_y);
        if !group.is_empty() {
            let max_h = group.iter()
                .filter_map(|id| class_boxes.get(id))
                .map(|cb| cb.height)
                .max()
                .unwrap_or(0);
            // Use larger gap if this level has fan-outs
            let v_gap = if level_has_fanout[lv] { v_gap_fanout } else { v_gap_normal };
            current_y += max_h + v_gap;
        }
    }
    
    // Assign Y positions to all boxes
    for cls in &diagram.classes {
        let lv = level.get(&cls.id).copied().unwrap_or(0);
        if let Some(cb) = class_boxes.get_mut(&cls.id) {
            cb.y = level_y[lv] as i32;
        }
    }
    
    // Position X coordinates bottom-up: start with deepest level, center parents above children
    // First, position the bottom level left-to-right
    {
        let group = &level_groups[max_level];
        let mut current_x: usize = 0;
        for id in group {
            if let Some(cb) = class_boxes.get_mut(id) {
                cb.x = current_x as i32;
                current_x += cb.width + h_gap;
            }
        }
    }
    
    // Work upward from bottom, centering parents over their children
    for lv in (0..max_level).rev() {
        let group = &level_groups[lv];
        
        // Track which nodes have been positioned (centered over children)
        let mut positioned: HashSet<String> = HashSet::new();
        
        // For each node in this level, if it has children, center over them
        for id in group {
            if let Some(child_set) = children.get(id) {
                if !child_set.is_empty() {
                    // Calculate bounding box of all children
                    let mut min_x = i32::MAX;
                    let mut max_x = i32::MIN;
                    for child_id in child_set {
                        if let Some(cb) = class_boxes.get(child_id) {
                            min_x = min_x.min(cb.x);
                            max_x = max_x.max(cb.x + cb.width as i32);
                        }
                    }
                    
                    if min_x != i32::MAX {
                        // Center this parent over children
                        let children_center = (min_x + max_x) / 2;
                        if let Some(cb) = class_boxes.get_mut(id) {
                            cb.x = children_center - cb.width as i32 / 2;
                            positioned.insert(id.clone());
                        }
                    }
                }
            }
        }
        
        // Position remaining nodes (those without children) in gaps
        let mut used_ranges: Vec<(i32, i32)> = Vec::new();
        for id in group {
            if positioned.contains(id) {
                if let Some(cb) = class_boxes.get(id) {
                    used_ranges.push((cb.x, cb.x + cb.width as i32));
                }
            }
        }
        used_ranges.sort_by_key(|(start, _)| *start);
        
        let mut current_x: i32 = 0;
        for id in group {
            if !positioned.contains(id) {
                if let Some(cb) = class_boxes.get_mut(id) {
                    // Find a spot that doesn't overlap
                    let width = cb.width as i32;
                    let mut x = current_x;
                    loop {
                        let end = x + width;
                        let overlaps = used_ranges.iter().any(|(s, e)| {
                            x < *e + h_gap as i32 && end > *s - h_gap as i32
                        });
                        if !overlaps {
                            break;
                        }
                        x += 1;
                    }
                    cb.x = x;
                    used_ranges.push((x, x + width));
                    used_ranges.sort_by_key(|(start, _)| *start);
                    current_x = x + width + h_gap as i32;
                }
            }
        }
    }
    
    // Ensure no negative X coordinates - shift everything right if needed
    let min_x = class_boxes.values().map(|cb| cb.x).min().unwrap_or(0);
    if min_x < 0 {
        for cb in class_boxes.values_mut() {
            cb.x -= min_x;
        }
    }
    
    // Calculate canvas size
    let mut total_w: usize = 0;
    let mut total_h: usize = 0;
    for cb in class_boxes.values() {
        total_w = total_w.max(cb.x as usize + cb.width);
        total_h = total_h.max(cb.y as usize + cb.height);
    }
    total_w += 4;
    total_h += 2;
    
    let mut canvas = mk_canvas(total_w, total_h);
    
    // Draw class boxes
    for cb in class_boxes.values() {
        draw_class_box(&mut canvas, cb, use_ascii);
    }
    
    // Group inheritance relationships by parent for fan-out rendering
    // Each entry stores (child_id, label, is_dashed)
    let mut inheritance_by_parent: HashMap<String, Vec<(String, Option<String>, bool)>> = HashMap::new();
    let mut non_hierarchical_rels: Vec<_> = Vec::new();
    
    for rel in &diagram.relationships {
        let is_hierarchical = matches!(rel.rel_type, RelationshipType::Inheritance | RelationshipType::Realization);
        if is_hierarchical {
            // Determine parent based on marker position
            let (parent_id, child_id) = if rel.marker_at_from {
                (rel.from.clone(), rel.to.clone())
            } else {
                (rel.to.clone(), rel.from.clone())
            };
            let is_dashed = matches!(rel.rel_type, RelationshipType::Realization);
            inheritance_by_parent.entry(parent_id).or_default().push((child_id, rel.label.clone(), is_dashed));
        } else {
            non_hierarchical_rels.push(rel);
        }
    }
    
    // Draw inheritance fan-outs
    let (solid_v, dashed_v) = if use_ascii { ('|', ':') } else { ('│', '┊') };
    let (solid_h, _dashed_h) = if use_ascii { ('-', '.') } else { ('─', '┄') };
    let marker_char = if use_ascii { '^' } else { '△' };
    // In ASCII mode, use dashes for corners; in Unicode mode, use box-drawing chars
    let corner_tl = if use_ascii { '-' } else { '┌' };
    let corner_tr = if use_ascii { '-' } else { '┐' };
    let _t_down = if use_ascii { '-' } else { '┬' };
    let _t_up = if use_ascii { '-' } else { '┴' };
    
    for (parent_id, children_info) in &inheritance_by_parent {
        let parent_box = match class_boxes.get(parent_id) {
            Some(b) => b,
            None => continue,
        };
        
        // Get all child boxes with their info
        let mut child_data: Vec<(i32, &ClassBox, Option<&String>, bool)> = children_info.iter()
            .filter_map(|(cid, label, is_dashed)| {
                class_boxes.get(cid).map(|cb| (cb.x + cb.width as i32 / 2, cb, label.as_ref(), *is_dashed))
            })
            .collect();
        child_data.sort_by_key(|(x, _, _, _)| *x);
        
        if child_data.is_empty() { continue; }
        
        let parent_center_x = parent_box.x + parent_box.width as i32 / 2;
        let parent_bottom_y = parent_box.y + parent_box.height as i32 - 1;
        
        // Draw inheritance marker below parent
        let marker_y = parent_bottom_y + 1;
        set_char(&mut canvas, parent_center_x, marker_y, marker_char);
        
        if child_data.len() == 1 {
            // Single child: draw vertical line with optional label
            let (child_cx, child_box, label_opt, is_dashed) = child_data[0];
            let child_top_y = child_box.y;
            let line_v = if is_dashed { dashed_v } else { solid_v };
            
            // Calculate midpoint for label (same as non-hierarchical relationships)
            let mid_y = (parent_bottom_y + 1 + child_top_y) / 2;
            
            // Draw label if present (with space padding for readability)
            if let Some(lbl) = label_opt {
                let padded = format!(" {} ", lbl);  // Add space padding on both sides
                let label_start = parent_center_x - (padded.len() as i32 / 2);
                for (i, ch) in padded.chars().enumerate() {
                    let x = label_start + i as i32;
                    if x >= 0 {
                        set_char(&mut canvas, x, mid_y, ch);
                    }
                }
                // Draw vertical lines above and below label
                for y in (marker_y + 1)..mid_y {
                    set_char(&mut canvas, parent_center_x, y, line_v);
                }
                for y in (mid_y + 1)..child_top_y {
                    set_char(&mut canvas, child_cx, y, line_v);
                }
                // If centers don't align, draw horizontal connector at label level
                if parent_center_x != child_cx {
                    let (left_x, right_x) = if parent_center_x < child_cx {
                        (parent_center_x, child_cx)
                    } else {
                        (child_cx, parent_center_x)
                    };
                    for x in left_x..=right_x {
                        // Don't overwrite label chars
                        let label_start = parent_center_x - (lbl.len() as i32 / 2);
                        let label_end = label_start + lbl.len() as i32 - 1;
                        if x < label_start || x > label_end {
                            set_char(&mut canvas, x, mid_y, solid_h);
                        }
                    }
                }
            } else if child_cx == parent_center_x {
                // No label, aligned: simple vertical line
                for y in (marker_y + 1)..child_top_y {
                    set_char(&mut canvas, parent_center_x, y, line_v);
                }
            } else {
                // No label, not aligned: draw elbow
                let line_y = marker_y + 1;
                
                // Horizontal connector
                let (left_x, right_x) = if parent_center_x < child_cx {
                    (parent_center_x, child_cx)
                } else {
                    (child_cx, parent_center_x)
                };
                
                set_char(&mut canvas, left_x, line_y, corner_tl);
                set_char(&mut canvas, right_x, line_y, corner_tr);
                for x in (left_x + 1)..right_x {
                    set_char(&mut canvas, x, line_y, solid_h);
                }
                
                // Vertical from child_cx down to child
                for y in (line_y + 1)..child_top_y {
                    set_char(&mut canvas, child_cx, y, line_v);
                }
            }
        } else {
            // Multiple children: draw fan-out with parent centered above
            let leftmost_x = child_data.first().unwrap().0;
            let rightmost_x = child_data.last().unwrap().0;
            
            // Check if parent is centered (within the span of children)
            let parent_is_centered = parent_center_x >= leftmost_x && parent_center_x <= rightmost_x;
            
            // Horizontal bar position - leave room for vertical line from parent if centered
            let bar_y = if parent_is_centered { marker_y + 2 } else { marker_y + 1 };
            
            // If centered, draw vertical line from marker to bar
            if parent_is_centered {
                for y in (marker_y + 1)..bar_y {
                    set_char(&mut canvas, parent_center_x, y, solid_v);
                }
            }
            
            // Draw horizontal bar spanning all children
            for x in leftmost_x..=rightmost_x {
                set_char(&mut canvas, x, bar_y, solid_h);
            }
            
            // Draw corners at the ends of the bar (in Unicode mode only)
            if !use_ascii {
                set_char(&mut canvas, leftmost_x, bar_y, corner_tl);
                set_char(&mut canvas, rightmost_x, bar_y, corner_tr);
            }
            
            // Draw junction where parent meets bar (if centered)
            // In ASCII mode, just keep the dash; in Unicode mode, use cross
            if parent_is_centered && !use_ascii {
                let cross = '┼';
                set_char(&mut canvas, parent_center_x, bar_y, cross);
            } else if !parent_is_centered {
                if parent_center_x < leftmost_x {
                    // Parent is to the left - draw corner and extend bar
                    set_char(&mut canvas, leftmost_x, bar_y, corner_tl);
                    for x in parent_center_x..leftmost_x {
                        set_char(&mut canvas, x, bar_y, solid_h);
                    }
                } else {
                    // Parent is to the right - extend bar
                    set_char(&mut canvas, rightmost_x, bar_y, corner_tr);
                    for x in (rightmost_x + 1)..=parent_center_x {
                        set_char(&mut canvas, x, bar_y, solid_h);
                    }
                }
            }
            
            // Draw vertical lines from bar down to each child
            for (child_cx, child_box, _label_opt, is_dashed) in &child_data {
                let child_top_y = child_box.y;
                let line_v = if *is_dashed { dashed_v } else { solid_v };
                
                // Vertical line down to child
                for y in (bar_y + 1)..child_top_y {
                    set_char(&mut canvas, *child_cx, y, line_v);
                }
            }
        }
    }
    
    // Draw non-hierarchical relationship lines
    for rel in &non_hierarchical_rels {
        let from_box = class_boxes.get(&rel.from);
        let to_box = class_boxes.get(&rel.to);
        if from_box.is_none() || to_box.is_none() { continue; }
        
        let from_box = from_box.unwrap();
        let to_box = to_box.unwrap();
        
        let is_dashed = matches!(rel.rel_type, RelationshipType::Dependency | RelationshipType::Realization);
        let line_v = if is_dashed { dashed_v } else { solid_v };
        
        let (top_box, bottom_box) = if from_box.y < to_box.y {
            (from_box, to_box)
        } else {
            (to_box, from_box)
        };
        
        let top_center_x = top_box.x + (top_box.width as i32 / 2);
        let top_bottom_y = top_box.y + top_box.height as i32 - 1;
        let bottom_center_x = bottom_box.x + (bottom_box.width as i32 / 2);
        let bottom_top_y = bottom_box.y;
        let mid_y = (top_bottom_y + 1 + bottom_top_y) / 2;
        
        let marker_char = get_marker_shape(&rel.rel_type, false, use_ascii);
        let marker_at_source = matches!(rel.rel_type, RelationshipType::Composition | RelationshipType::Aggregation);
        
        if marker_at_source {
            // Marker at source (top)
            let marker_y = top_bottom_y + 1;
            set_char(&mut canvas, top_center_x, marker_y, marker_char);
            
            // Draw label if present (with space padding)
            if let Some(ref lbl) = rel.label {
                let padded = format!(" {} ", lbl);
                let label_start = top_center_x - (padded.len() as i32 / 2);
                for (i, ch) in padded.chars().enumerate() {
                    let x = label_start + i as i32;
                    if x >= 0 {
                        set_char(&mut canvas, x, mid_y, ch);
                    }
                }
            }
            
            // Vertical line from after marker/label to target
            for y in (mid_y + 1)..bottom_top_y {
                set_char(&mut canvas, top_center_x, y, line_v);
            }
        } else {
            // Arrow at target (bottom)
            // Vertical line from source
            for y in (top_bottom_y + 1)..mid_y {
                set_char(&mut canvas, top_center_x, y, line_v);
            }
            
            // Draw label if present (with space padding)
            if let Some(ref lbl) = rel.label {
                let padded = format!(" {} ", lbl);
                let label_start = top_center_x - (padded.len() as i32 / 2);
                for (i, ch) in padded.chars().enumerate() {
                    let x = label_start + i as i32;
                    if x >= 0 {
                        set_char(&mut canvas, x, mid_y, ch);
                    }
                }
            }
            
            // Vertical line to arrow
            for y in (mid_y + 1)..(bottom_top_y - 1) {
                set_char(&mut canvas, bottom_center_x, y, line_v);
            }
            
            // Arrow head pointing down
            set_char(&mut canvas, bottom_center_x, bottom_top_y - 1, marker_char);
        }
    }
    
    // Second pass: Draw all relationship labels in INPUT order
    // This ensures later labels overwrite earlier ones correctly (like TypeScript does)
    for rel in &diagram.relationships {
        if rel.label.is_none() { continue; }
        let label = rel.label.as_ref().unwrap();
        
        let from_box = class_boxes.get(&rel.from);
        let to_box = class_boxes.get(&rel.to);
        if from_box.is_none() || to_box.is_none() { continue; }
        
        let from_box = from_box.unwrap();
        let to_box = to_box.unwrap();
        
        let (top_box, _bottom_box) = if from_box.y < to_box.y {
            (from_box, to_box)
        } else {
            (to_box, from_box)
        };
        
        // Calculate midpoint for label
        let (parent_bottom_y, child_top_y) = if from_box.y < to_box.y {
            (from_box.y + from_box.height as i32 - 1, to_box.y)
        } else {
            (to_box.y + to_box.height as i32 - 1, from_box.y)
        };
        let mid_y = (parent_bottom_y + 1 + child_top_y) / 2;
        
        let center_x = top_box.x + (top_box.width as i32 / 2);
        
        // Draw padded label
        let padded = format!(" {} ", label);
        let label_start = center_x - (padded.len() as i32 / 2);
        for (i, ch) in padded.chars().enumerate() {
            let x = label_start + i as i32;
            if x >= 0 {
                set_char(&mut canvas, x, mid_y, ch);
            }
        }
    }
    
    Ok(canvas_to_string(&canvas))
}

fn get_marker_shape(rel_type: &RelationshipType, _is_hierarchical: bool, use_ascii: bool) -> char {
    match rel_type {
        RelationshipType::Inheritance | RelationshipType::Realization => {
            if use_ascii { '^' } else { '△' }
        },
        RelationshipType::Composition => {
            if use_ascii { '*' } else { '◆' }
        },
        RelationshipType::Aggregation => {
            if use_ascii { 'o' } else { '◇' }
        },
        RelationshipType::Association | RelationshipType::Dependency => {
            if use_ascii { 'v' } else { '▼' }
        },
    }
}

struct ClassBox {
    _id: String,
    label: String,
    annotation: Option<String>,
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
    
    let mut cur_y = y + 1;
    
    // Header section: optional annotation + class name
    if let Some(ref annot) = cb.annotation {
        let annot_str = format!("<<{}>>", annot);
        set_char(canvas, x, cur_y, v_line);
        set_char(canvas, x + 1, cur_y, ' ');
        draw_text(canvas, x + 2, cur_y, &annot_str);
        set_char(canvas, x + w - 1, cur_y, v_line);
        cur_y += 1;
    }
    
    // Class name row
    set_char(canvas, x, cur_y, v_line);
    set_char(canvas, x + 1, cur_y, ' ');
    draw_text(canvas, x + 2, cur_y, &cb.label);
    set_char(canvas, x + w - 1, cur_y, v_line);
    cur_y += 1;
    
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
