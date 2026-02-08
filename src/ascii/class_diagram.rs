//! Class diagram ASCII rendering

use super::canvas::{canvas_to_string, draw_text, mk_canvas, set_char};
use super::types::AsciiConfig;
use crate::types::{ClassDiagram, ClassMember, RelationshipType, Visibility};
use std::collections::{HashMap, HashSet};

/// Render a class diagram to ASCII
pub fn render_class_ascii(diagram: &ClassDiagram, config: &AsciiConfig) -> Result<String, String> {
    if diagram.classes.is_empty() {
        return Ok(String::new());
    }

    let use_ascii = config.use_ascii;
    let padding = 1;
    let h_gap = 4; // horizontal gap between class boxes
    let v_gap_normal = 3; // vertical gap for single child inheritance
    let v_gap_fanout = 4; // vertical gap when parent has multiple children (for centered layout)
    let is_horizontal = diagram.direction == "LR" || diagram.direction == "RL";
    let is_rl = diagram.direction == "RL";

    // Build box dimensions for each class
    let mut class_boxes: HashMap<String, ClassBox> = HashMap::new();

    for cls in &diagram.classes {
        // Lollipop interface nodes are rendered as plain text labels (no box)
        if cls.is_lollipop {
            class_boxes.insert(
                cls.id.clone(),
                ClassBox {
                    _id: cls.id.clone(),
                    label: cls.label.clone(),
                    annotation: None,
                    attr_lines: Vec::new(),
                    method_lines: Vec::new(),
                    width: cls.label.len(),
                    height: 1,
                    x: 0,
                    y: 0,
                    is_lollipop: true,
                },
            );
            continue;
        }

        let has_annotation = cls.annotation.is_some();
        let annotation_str = cls.annotation.as_ref().map(|a| format!("<<{}>>", a));

        // Calculate width based on widest line
        let annotation_width = annotation_str.as_ref().map(|s| s.len()).unwrap_or(0) + 2 * padding;
        let header_width = cls.label.len() + 2 * padding;

        let attr_lines: Vec<String> = cls.attributes.iter().map(format_member).collect();
        let method_lines: Vec<String> = cls.methods.iter().map(format_member).collect();

        let attr_width = attr_lines.iter().map(|s| s.len()).max().unwrap_or(0) + 2 * padding;
        let method_width = method_lines.iter().map(|s| s.len()).max().unwrap_or(0) + 2 * padding;

        let inner_width = header_width
            .max(attr_width)
            .max(method_width)
            .max(annotation_width);
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

        class_boxes.insert(
            cls.id.clone(),
            ClassBox {
                _id: cls.id.clone(),
                label: cls.label.clone(),
                annotation: cls.annotation.clone(),
                attr_lines,
                method_lines,
                width: box_width,
                height: box_height,
                x: 0,
                y: 0,
                is_lollipop: false,
            },
        );
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
        let is_hierarchical = matches!(
            rel.rel_type,
            RelationshipType::Inheritance | RelationshipType::Realization
        );

        let (parent_id, child_id) = if is_hierarchical && !rel.marker_at_from {
            // marker at 'to' side means 'to' is the parent
            (rel.to.clone(), rel.from.clone())
        } else {
            // Default: 'from' is parent/source, 'to' is child/target
            (rel.from.clone(), rel.to.clone())
        };

        parents
            .entry(child_id.clone())
            .or_default()
            .insert(parent_id.clone());
        children
            .entry(parent_id.clone())
            .or_default()
            .insert(child_id);
    }

    // BFS from roots to assign levels
    let mut level: HashMap<String, usize> = HashMap::new();
    let roots: Vec<_> = diagram
        .classes
        .iter()
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
                if new_level > level_cap {
                    continue;
                }
                if !level.contains_key(child_id)
                    || level.get(child_id).copied().unwrap_or(0) < new_level
                {
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

    // ========================================================================
    // Horizontal layout (LR / RL)
    // ========================================================================
    if is_horizontal {
        return render_horizontal_class_diagram(
            diagram,
            &mut class_boxes,
            &level,
            &level_groups,
            max_level,
            &children,
            h_gap,
            is_rl,
            use_ascii,
        );
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
            let max_h = group
                .iter()
                .filter_map(|id| class_boxes.get(id))
                .map(|cb| cb.height)
                .max()
                .unwrap_or(0);
            // Use larger gap if this level has fan-outs
            let v_gap = if level_has_fanout[lv] {
                v_gap_fanout
            } else {
                v_gap_normal
            };
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
                    // Calculate center based on child center points
                    let mut min_cx = i32::MAX;
                    let mut max_cx = i32::MIN;
                    for child_id in child_set {
                        if let Some(cb) = class_boxes.get(child_id) {
                            let cx = cb.x + cb.width as i32 / 2;
                            min_cx = min_cx.min(cx);
                            max_cx = max_cx.max(cx);
                        }
                    }

                    if min_cx != i32::MAX {
                        // Center this parent over the midpoint of child centers
                        let children_center = (min_cx + max_cx) / 2;
                        if let Some(cb) = class_boxes.get_mut(id) {
                            cb.x = children_center - cb.width as i32 / 2;
                            positioned.insert(id.clone());
                        }
                    }
                }
            }
        }

        // Resolve overlaps among positioned nodes at this level:
        // collect them in group order, then shift any that overlap a predecessor
        let positioned_ids: Vec<&String> =
            group.iter().filter(|id| positioned.contains(*id)).collect();
        if positioned_ids.len() > 1 {
            // Sort by subtree depth (deepest first → center/left) then by current X.
            // This ensures nodes with deep subtrees occupy interior positions and
            // nodes with shallow connections sit on the outside where their edges
            // can drop straight down without crossing through intermediate boxes.
            fn subtree_depth(
                id: &str,
                children: &HashMap<String, HashSet<String>>,
                memo: &mut HashMap<String, usize>,
            ) -> usize {
                if let Some(&d) = memo.get(id) {
                    return d;
                }
                let d = match children.get(id) {
                    Some(cs) if !cs.is_empty() => {
                        1 + cs
                            .iter()
                            .map(|c| subtree_depth(c, children, memo))
                            .max()
                            .unwrap_or(0)
                    }
                    _ => 0,
                };
                memo.insert(id.to_string(), d);
                d
            }
            let mut depth_memo: HashMap<String, usize> = HashMap::new();
            let mut sorted: Vec<String> = positioned_ids.into_iter().cloned().collect();
            sorted.sort_by(|a, b| {
                let da = subtree_depth(a, &children, &mut depth_memo);
                let db = subtree_depth(b, &children, &mut depth_memo);
                // Deeper subtrees first (larger depth → smaller sort key), then by X
                db.cmp(&da).then_with(|| {
                    let xa = class_boxes.get(a).map(|cb| cb.x).unwrap_or(0);
                    let xb = class_boxes.get(b).map(|cb| cb.x).unwrap_or(0);
                    xa.cmp(&xb)
                })
            });
            // Re-position: deepest-first gets its centered position; others shift right
            for i in 1..sorted.len() {
                let prev_end = class_boxes
                    .get(&sorted[i - 1])
                    .map(|cb| cb.x + cb.width as i32)
                    .unwrap_or(0);
                if let Some(cb) = class_boxes.get_mut(&sorted[i]) {
                    let min_x = prev_end + h_gap as i32;
                    if cb.x < min_x {
                        cb.x = min_x;
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
                        let overlaps = used_ranges
                            .iter()
                            .any(|(s, e)| x < *e + h_gap as i32 && end > *s - h_gap as i32);
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
    // Also account for relationship labels that extend left of their parent box
    let mut min_x = class_boxes.values().map(|cb| cb.x).min().unwrap_or(0);

    // Check if any relationship label would extend past the left edge
    for rel in &diagram.relationships {
        if let Some(ref lbl) = rel.label {
            let padded_len = lbl.len() as i32 + 2; // " label "
                                                   // Find the box whose center the label will be drawn around
            let center_x = if let Some(from_box) = class_boxes.get(&rel.from) {
                if let Some(to_box) = class_boxes.get(&rel.to) {
                    let (top_box, _) = if from_box.y <= to_box.y {
                        (from_box, to_box)
                    } else {
                        (to_box, from_box)
                    };
                    top_box.x + top_box.width as i32 / 2
                } else {
                    continue;
                }
            } else {
                continue;
            };
            let label_start = center_x - padded_len / 2;
            min_x = min_x.min(label_start);
        }
    }

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

    // Draw class boxes (in definition order for deterministic overlap)
    for cls in &diagram.classes {
        if let Some(cb) = class_boxes.get(&cls.id) {
            if cb.is_lollipop {
                // Draw as plain text label (no box)
                draw_text(&mut canvas, cb.x, cb.y, &cb.label);
            } else {
                draw_class_box(&mut canvas, cb, use_ascii);
            }
        }
    }

    // Group inheritance relationships by parent for fan-out rendering
    // Each entry stores (child_id, label, is_dashed)
    let mut inheritance_by_parent: HashMap<String, Vec<(String, Option<String>, bool)>> =
        HashMap::new();
    let mut non_hierarchical_rels: Vec<_> = Vec::new();

    for rel in &diagram.relationships {
        let is_hierarchical = matches!(
            rel.rel_type,
            RelationshipType::Inheritance | RelationshipType::Realization
        );
        if is_hierarchical {
            // Determine parent based on marker position
            let (parent_id, child_id) = if rel.marker_at_from {
                (rel.from.clone(), rel.to.clone())
            } else {
                (rel.to.clone(), rel.from.clone())
            };
            let is_dashed = matches!(rel.rel_type, RelationshipType::Realization);
            inheritance_by_parent.entry(parent_id).or_default().push((
                child_id,
                rel.label.clone(),
                is_dashed,
            ));
        } else {
            non_hierarchical_rels.push(rel);
        }
    }

    // Draw inheritance fan-outs
    let (solid_v, dashed_v) = if use_ascii {
        ('|', ':')
    } else {
        ('│', '┊')
    };
    let (solid_h, _dashed_h) = if use_ascii {
        ('-', '.')
    } else {
        ('─', '┄')
    };
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
        let mut child_data: Vec<(i32, &ClassBox, Option<&String>, bool)> = children_info
            .iter()
            .filter_map(|(cid, label, is_dashed)| {
                class_boxes
                    .get(cid)
                    .map(|cb| (cb.x + cb.width as i32 / 2, cb, label.as_ref(), *is_dashed))
            })
            .collect();
        child_data.sort_by_key(|(x, _, _, _)| *x);

        if child_data.is_empty() {
            continue;
        }

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
                let padded = format!(" {} ", lbl); // Add space padding on both sides
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
            let parent_is_centered =
                parent_center_x >= leftmost_x && parent_center_x <= rightmost_x;

            // Horizontal bar position - leave room for vertical line from parent if centered
            let bar_y = if parent_is_centered {
                marker_y + 2
            } else {
                marker_y + 1
            };

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

                // For middle children, snap the vertical bar to the parent center
                // if it falls within the child's box, for better visual alignment
                let drop_x = if *child_cx != leftmost_x
                    && *child_cx != rightmost_x
                    && parent_center_x >= child_box.x
                    && parent_center_x < child_box.x + child_box.width as i32
                {
                    parent_center_x
                } else {
                    *child_cx
                };

                // Vertical line down to child
                for y in (bar_y + 1)..child_top_y {
                    set_char(&mut canvas, drop_x, y, line_v);
                }
            }
        }
    }

    // Collect all box bounding rectangles for collision detection
    let all_boxes: Vec<(i32, i32, i32, i32)> = class_boxes
        .values()
        .map(|cb| {
            (
                cb.x,
                cb.y,
                cb.x + cb.width as i32 - 1,
                cb.y + cb.height as i32 - 1,
            )
        })
        .collect();

    // Draw non-hierarchical relationship lines
    for rel in &non_hierarchical_rels {
        let from_box = class_boxes.get(&rel.from);
        let to_box = class_boxes.get(&rel.to);
        if from_box.is_none() || to_box.is_none() {
            continue;
        }

        let from_box = from_box.unwrap();
        let to_box = to_box.unwrap();

        let is_dashed = matches!(
            rel.rel_type,
            RelationshipType::Dependency | RelationshipType::Realization
        );
        let line_v = if is_dashed { dashed_v } else { solid_v };
        let line_h = if is_dashed { _dashed_h } else { solid_h };

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
        let marker_at_source = matches!(
            rel.rel_type,
            RelationshipType::Composition | RelationshipType::Aggregation
        );

        // Determine cardinality placement: from_card near source, to_card near target
        // "from" is top when from_box.y < to_box.y, otherwise bottom
        let (top_card, bottom_card) = if from_box.y <= to_box.y {
            (&rel.from_cardinality, &rel.to_cardinality)
        } else {
            (&rel.to_cardinality, &rel.from_cardinality)
        };

        // Check if the straight vertical path from top_center_x passes through any
        // intermediate box (not the source or target box themselves)
        let mut blocked_by: Option<(i32, i32, i32, i32)> = None;
        for &(bx1, by1, bx2, by2) in &all_boxes {
            // Skip the source and target boxes themselves
            if by1 == top_box.y && bx1 == top_box.x {
                continue;
            }
            if by1 == bottom_box.y && bx1 == bottom_box.x {
                continue;
            }
            // Check if the vertical line at top_center_x would pass through this box
            if top_center_x >= bx1
                && top_center_x <= bx2
                && by1 > top_bottom_y
                && by2 < bottom_top_y
            {
                blocked_by = Some((bx1, by1, bx2, by2));
                break;
            }
        }

        if let Some((blocker_x1, _blocker_y1, blocker_x2, blocker_y2)) = blocked_by {
            // Route around the blocking box: go to one side, down past it, then to target
            // Choose the side that minimizes distance: prefer routing toward the target,
            // but ensure route_x is outside the blocker
            let dist_left = (top_center_x - (blocker_x1 - 2)).abs();
            let dist_right = ((blocker_x2 + 2) - top_center_x).abs();
            let route_x = if dist_left <= dist_right && blocker_x1 - 2 >= 0 {
                blocker_x1 - 2 // two columns left of blocker
            } else {
                blocker_x2 + 2 // two columns right of blocker
            };

            // Vertical line from top box down to first bend
            let bend1_y = top_bottom_y + 1;

            // Horizontal line from top_center_x to route_x
            let (hx1, hx2) = if route_x < top_center_x {
                (route_x, top_center_x)
            } else {
                (top_center_x, route_x)
            };
            for x in hx1..=hx2 {
                set_char(&mut canvas, x, bend1_y, line_h);
            }

            // Vertical line down past the blocker
            let bend2_y = blocker_y2 + 1;
            for y in (bend1_y + 1)..=bend2_y {
                set_char(&mut canvas, route_x, y, line_v);
            }

            // Horizontal line from route_x to bottom_center_x
            let (hx1, hx2) = if route_x < bottom_center_x {
                (route_x, bottom_center_x)
            } else {
                (bottom_center_x, route_x)
            };
            for x in hx1..=hx2 {
                set_char(&mut canvas, x, bend2_y, line_h);
            }

            // Vertical line from bend2 down to arrow
            for y in (bend2_y + 1)..(bottom_top_y - 1) {
                set_char(&mut canvas, bottom_center_x, y, line_v);
            }

            // Arrow head pointing down
            set_char(&mut canvas, bottom_center_x, bottom_top_y - 1, marker_char);
        } else if marker_at_source {
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
            if let Some(lbl) = rel.label.as_ref() {
                // Vertical line from source to mid_y (label row)
                for y in (top_bottom_y + 1)..mid_y {
                    set_char(&mut canvas, top_center_x, y, line_v);
                }

                // Draw label (with space padding)
                let padded = format!(" {} ", lbl);
                let label_start = top_center_x - (padded.len() as i32 / 2);
                for (i, ch) in padded.chars().enumerate() {
                    let x = label_start + i as i32;
                    if x >= 0 {
                        set_char(&mut canvas, x, mid_y, ch);
                    }
                }

                // Vertical line from below label to arrow
                for y in (mid_y + 1)..(bottom_top_y - 1) {
                    set_char(&mut canvas, bottom_center_x, y, line_v);
                }
            } else if top_center_x == bottom_center_x {
                // No label, aligned: simple vertical line
                for y in (top_bottom_y + 1)..(bottom_top_y - 1) {
                    set_char(&mut canvas, top_center_x, y, line_v);
                }
            } else if (top_center_x - bottom_center_x).abs() <= 2 {
                // No label, nearly aligned: draw straight vertical at bottom center
                for y in (top_bottom_y + 1)..(bottom_top_y - 1) {
                    set_char(&mut canvas, bottom_center_x, y, line_v);
                }
            } else {
                // No label, not aligned: draw elbow via midpoint
                // Check if the horizontal segment at mid_y would cross any box
                let (hx_min, hx_max) = if top_center_x < bottom_center_x {
                    (top_center_x, bottom_center_x)
                } else {
                    (bottom_center_x, top_center_x)
                };
                let mut elbow_y = mid_y;
                for &(bx1, by1, bx2, by2) in &all_boxes {
                    // Skip source and target
                    if by1 == top_box.y && bx1 == top_box.x {
                        continue;
                    }
                    if by1 == bottom_box.y && bx1 == bottom_box.x {
                        continue;
                    }
                    // If the box overlaps the horizontal segment range at elbow_y
                    if elbow_y >= by1 && elbow_y <= by2 && bx2 >= hx_min && bx1 <= hx_max {
                        // Move elbow below this box
                        elbow_y = by2 + 1;
                    }
                }
                for y in (top_bottom_y + 1)..elbow_y {
                    set_char(&mut canvas, top_center_x, y, line_v);
                }
                // Horizontal connector at elbow_y
                for x in hx_min..=hx_max {
                    set_char(&mut canvas, x, elbow_y, line_h);
                }
                // Vertical from connector down to arrow
                for y in (elbow_y + 1)..(bottom_top_y - 1) {
                    set_char(&mut canvas, bottom_center_x, y, line_v);
                }
            }

            // Arrow head pointing down
            set_char(&mut canvas, bottom_center_x, bottom_top_y - 1, marker_char);
        }

        // Draw cardinality labels
        // Top cardinality: to the left of the vertical line, on the first line below the source box
        if let Some(ref card) = top_card {
            let card_y = top_bottom_y + 1;
            let card_x = top_center_x - card.len() as i32;
            draw_text(&mut canvas, card_x, card_y, card);
        }
        // Bottom cardinality: right after the arrow marker
        if let Some(ref card) = bottom_card {
            let card_y = bottom_top_y - 1;
            let card_x = bottom_center_x + 1;
            if card.len() > 1 {
                let padded = format!(" {}", card);
                draw_text(&mut canvas, card_x, card_y, &padded);
            } else {
                draw_text(&mut canvas, card_x, card_y, card);
            }
        }
    }

    // Second pass: Draw all relationship labels in INPUT order
    // This ensures later labels overwrite earlier ones correctly (like TypeScript does)
    for rel in &diagram.relationships {
        if rel.label.is_none() {
            continue;
        }
        let label = rel.label.as_ref().unwrap();

        let from_box = class_boxes.get(&rel.from);
        let to_box = class_boxes.get(&rel.to);
        if from_box.is_none() || to_box.is_none() {
            continue;
        }

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

/// Render a class diagram with horizontal (LR/RL) layout.
/// Levels become columns; nodes within a column stack vertically.
/// For RL, level 0 is rightmost; for LR, level 0 is leftmost.
#[allow(clippy::too_many_arguments)]
fn render_horizontal_class_diagram(
    diagram: &ClassDiagram,
    class_boxes: &mut HashMap<String, ClassBox>,
    level: &HashMap<String, usize>,
    level_groups: &[Vec<String>],
    max_level: usize,
    children: &HashMap<String, HashSet<String>>,
    _h_gap: usize,
    is_rl: bool,
    use_ascii: bool,
) -> Result<String, String> {
    let v_gap = 1; // vertical gap between boxes in the same column

    // Compute X positions for each level (columns left-to-right)
    // For RL: level 0 goes at the rightmost column, so we reverse
    let mut level_x: Vec<usize> = Vec::new();
    let mut current_x: usize = 0;

    // Edge label gap: space for the label + marker + padding between columns
    // Calculate based on longest relationship label in the diagram
    let max_label_len = diagram
        .relationships
        .iter()
        .filter_map(|r| r.label.as_ref())
        .map(|l| l.len())
        .max()
        .unwrap_or(0);
    // Need room for: gap(1) + marker(1) + cardinality(~2) + space(1) + label + space(1) + cardinality(~2) + marker(1) + gap(1)
    let edge_gap = (max_label_len + 10).max(12);

    for group in level_groups.iter() {
        level_x.push(current_x);
        if !group.is_empty() {
            let max_w = group
                .iter()
                .filter_map(|id| class_boxes.get(id))
                .map(|cb| cb.width)
                .max()
                .unwrap_or(0);
            current_x += max_w + edge_gap;
        }
    }

    // For RL: reverse so level 0 is rightmost
    if is_rl {
        let total_width = current_x;
        // We need to mirror: x' = total_width - x - column_width
        // But simpler: just reverse the level_x mapping
        let mut reversed_x: Vec<usize> = Vec::new();
        for lv in 0..=max_level {
            let group = &level_groups[lv];
            let max_w = group
                .iter()
                .filter_map(|id| class_boxes.get(id))
                .map(|cb| cb.width)
                .max()
                .unwrap_or(0);
            // Mirror: place this level at (total_width - original_x - max_w)
            let mirrored = total_width.saturating_sub(level_x[lv] + max_w);
            reversed_x.push(mirrored);
        }
        level_x = reversed_x;
    }

    // Assign X positions to all boxes based on their level
    for cls in &diagram.classes {
        let lv = level.get(&cls.id).copied().unwrap_or(0);
        if let Some(cb) = class_boxes.get_mut(&cls.id) {
            cb.x = level_x[lv] as i32;
        }
    }

    // Assign Y positions: within each level column, stack boxes vertically
    // Center parents vertically over their children (work rightward for RL, leftward for LR)
    // First pass: position the leaf level (deepest) top-to-bottom
    {
        let leaf_level = if is_rl { 0 } else { max_level };
        let group = &level_groups[leaf_level];
        let mut current_y: usize = 0;
        for id in group {
            if let Some(cb) = class_boxes.get_mut(id) {
                cb.y = current_y as i32;
                current_y += cb.height + v_gap;
            }
        }
    }

    // Work from leaves toward roots, centering parents over their children
    let levels_to_process: Vec<usize> = if is_rl {
        (1..=max_level).collect()
    } else {
        (0..max_level).rev().collect()
    };

    for lv in levels_to_process {
        let group = &level_groups[lv];
        let mut positioned: HashSet<String> = HashSet::new();

        for id in group {
            if let Some(child_set) = children.get(id) {
                if !child_set.is_empty() {
                    let mut min_y = i32::MAX;
                    let mut max_y_end = i32::MIN;
                    for child_id in child_set {
                        if let Some(cb) = class_boxes.get(child_id) {
                            min_y = min_y.min(cb.y);
                            max_y_end = max_y_end.max(cb.y + cb.height as i32);
                        }
                    }
                    if min_y != i32::MAX {
                        let children_center_y = (min_y + max_y_end) / 2;
                        if let Some(cb) = class_boxes.get_mut(id) {
                            cb.y = children_center_y - cb.height as i32 / 2;
                            positioned.insert(id.clone());
                        }
                    }
                }
            }
        }

        // Position remaining nodes (without children) stacked below positioned ones
        let mut current_y: i32 = 0;
        for id in group {
            if positioned.contains(id) {
                if let Some(cb) = class_boxes.get(id) {
                    let end = cb.y + cb.height as i32 + v_gap as i32;
                    if end > current_y {
                        current_y = end;
                    }
                }
            }
        }
        for id in group {
            if !positioned.contains(id) {
                if let Some(cb) = class_boxes.get_mut(id) {
                    cb.y = current_y;
                    current_y += cb.height as i32 + v_gap as i32;
                }
            }
        }
    }

    // Ensure no negative Y coordinates
    let min_y = class_boxes.values().map(|cb| cb.y).min().unwrap_or(0);
    if min_y < 0 {
        for cb in class_boxes.values_mut() {
            cb.y -= min_y;
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
    for cls in &diagram.classes {
        if let Some(cb) = class_boxes.get(&cls.id) {
            if cb.is_lollipop {
                draw_text(&mut canvas, cb.x, cb.y, &cb.label);
            } else {
                draw_class_box(&mut canvas, cb, use_ascii);
            }
        }
    }

    // Character sets
    let (solid_h, dashed_h) = if use_ascii {
        ('-', '.')
    } else {
        ('─', '┄')
    };
    let (solid_v, _dashed_v) = if use_ascii {
        ('|', ':')
    } else {
        ('│', '┊')
    };

    // Draw horizontal relationship edges
    // Each edge connects from the right side of the left box to the left side of the right box.
    // The line is drawn at the target box's vertical center.
    // If the source is at a different Y, a vertical connector is drawn on the source side.
    for rel in &diagram.relationships {
        let from_box = match class_boxes.get(&rel.from) {
            Some(b) => b,
            None => continue,
        };
        let to_box = match class_boxes.get(&rel.to) {
            Some(b) => b,
            None => continue,
        };

        // Determine which box is left and which is right
        let (left_box, right_box, from_is_left) = if from_box.x < to_box.x {
            (from_box, to_box, true)
        } else {
            (to_box, from_box, false)
        };

        // Connection X coordinates: just outside each box edge, with 1 char gap
        let left_conn_x = left_box.x + left_box.width as i32 + 1;
        let right_conn_x = right_box.x - 2;

        // Y centers
        let left_center_y = left_box.y + left_box.height as i32 / 2;
        let right_center_y = right_box.y + right_box.height as i32 / 2;

        // Determine marker side (needed for vertical connector X position)
        let is_hierarchical = matches!(
            rel.rel_type,
            RelationshipType::Inheritance | RelationshipType::Realization
        );
        let marker_at_source = matches!(
            rel.rel_type,
            RelationshipType::Composition | RelationshipType::Aggregation
        );
        let marker_on_from = if is_hierarchical || marker_at_source {
            rel.marker_at_from
        } else {
            false
        };
        let marker_on_left = if marker_on_from {
            from_is_left
        } else {
            !from_is_left
        };

        // Draw the edge at the target box's center Y, with elbow if needed
        let target_is_left = !from_is_left;
        let line_y = if target_is_left {
            left_center_y
        } else {
            right_center_y
        };
        let source_y = if target_is_left {
            right_center_y
        } else {
            left_center_y
        };

        // Phase 1: Draw vertical connectors first
        if source_y != line_y {
            let vert_x = if target_is_left {
                if marker_on_left {
                    right_conn_x
                } else {
                    right_conn_x - 1
                }
            } else if marker_on_left {
                left_conn_x + 1
            } else {
                left_conn_x
            };
            let (y_min, y_max) = if source_y < line_y {
                (source_y, line_y)
            } else {
                (line_y, source_y)
            };
            for y in y_min..=y_max {
                set_char(&mut canvas, vert_x, y, solid_v);
            }
        }
    }

    // Phase 2: Draw horizontal lines and markers after all vertical connectors,
    // so horizontal lines overwrite vertical connectors at junction points
    for rel in &diagram.relationships {
        let from_box = match class_boxes.get(&rel.from) {
            Some(b) => b,
            None => continue,
        };
        let to_box = match class_boxes.get(&rel.to) {
            Some(b) => b,
            None => continue,
        };

        let is_dashed = matches!(
            rel.rel_type,
            RelationshipType::Dependency | RelationshipType::Realization
        );
        let line_h = if is_dashed { dashed_h } else { solid_h };

        let (left_box, right_box, from_is_left) = if from_box.x < to_box.x {
            (from_box, to_box, true)
        } else {
            (to_box, from_box, false)
        };

        let left_conn_x = left_box.x + left_box.width as i32 + 1;
        let right_conn_x = right_box.x - 2;
        let left_center_y = left_box.y + left_box.height as i32 / 2;
        let right_center_y = right_box.y + right_box.height as i32 / 2;

        let is_hierarchical = matches!(
            rel.rel_type,
            RelationshipType::Inheritance | RelationshipType::Realization
        );
        let marker_at_source = matches!(
            rel.rel_type,
            RelationshipType::Composition | RelationshipType::Aggregation
        );
        let marker_char = if is_hierarchical {
            if use_ascii {
                '<'
            } else {
                '◁'
            }
        } else if marker_at_source {
            get_marker_shape(&rel.rel_type, false, use_ascii)
        } else if from_is_left {
            if use_ascii {
                '>'
            } else {
                '▶'
            }
        } else if use_ascii {
            '<'
        } else {
            '◀'
        };
        let marker_on_from = if is_hierarchical || marker_at_source {
            rel.marker_at_from
        } else {
            false
        };
        let marker_on_left = if marker_on_from {
            from_is_left
        } else {
            !from_is_left
        };

        let target_is_left = !from_is_left;
        let line_y = if target_is_left {
            left_center_y
        } else {
            right_center_y
        };
        let source_y = if target_is_left {
            right_center_y
        } else {
            left_center_y
        };

        let (marker_x, line_start, line_end) = if marker_on_left {
            (left_conn_x, left_conn_x + 1, right_conn_x)
        } else {
            (right_conn_x, left_conn_x, right_conn_x - 1)
        };

        for x in line_start..=line_end {
            set_char(&mut canvas, x, line_y, line_h);
        }
        set_char(&mut canvas, marker_x, line_y, marker_char);

        // In Unicode mode, draw corner piece where vertical connector meets horizontal line
        // Only for significant vertical offsets (> 1 row) to avoid corners on short elbows
        if !use_ascii && (source_y - line_y).abs() > 1 {
            let vert_x = if target_is_left {
                if marker_on_left {
                    right_conn_x
                } else {
                    right_conn_x - 1
                }
            } else if marker_on_left {
                left_conn_x + 1
            } else {
                left_conn_x
            };
            // Determine corner type based on direction of vertical and horizontal
            let corner = if source_y < line_y {
                // Vertical comes from above
                if vert_x > marker_x {
                    '┘' // horizontal goes left, vertical comes from above
                } else {
                    '└' // horizontal goes right, vertical comes from above
                }
            } else {
                // Vertical comes from below
                if vert_x > marker_x {
                    '┐' // horizontal goes left, vertical comes from below
                } else {
                    '┌' // horizontal goes right, vertical comes from below
                }
            };
            set_char(&mut canvas, vert_x, line_y, corner);
        }
    }

    // Phase 3: draw cardinalities and labels after all edges
    // Skip from_cardinality on source side if vertical connector runs through card_y
    for rel in &diagram.relationships {
        let from_box = match class_boxes.get(&rel.from) {
            Some(b) => b,
            None => continue,
        };
        let to_box = match class_boxes.get(&rel.to) {
            Some(b) => b,
            None => continue,
        };

        let (left_box, right_box, from_is_left) = if from_box.x < to_box.x {
            (from_box, to_box, true)
        } else {
            (to_box, from_box, false)
        };

        let left_conn_x = left_box.x + left_box.width as i32 + 1;
        let right_conn_x = right_box.x - 2;
        let left_center_y = left_box.y + left_box.height as i32 / 2;
        let right_center_y = right_box.y + right_box.height as i32 / 2;

        let target_is_left = !from_is_left;
        let line_y = if target_is_left {
            left_center_y
        } else {
            right_center_y
        };
        let source_y = if target_is_left {
            right_center_y
        } else {
            left_center_y
        };

        // Re-derive marker_on_left so we can offset cardinalities past the marker
        let is_hierarchical = matches!(
            rel.rel_type,
            RelationshipType::Inheritance | RelationshipType::Realization
        );
        let marker_at_source = matches!(
            rel.rel_type,
            RelationshipType::Composition | RelationshipType::Aggregation
        );
        let marker_on_from = if is_hierarchical || marker_at_source {
            rel.marker_at_from
        } else {
            false
        };
        let marker_on_left = if marker_on_from {
            from_is_left
        } else {
            !from_is_left
        };

        // Cardinality: both on the same line as the label (line above the edge)
        // Offset by 1 from the marker position so cardinality doesn't overlap the marker
        let card_y = line_y - 1;
        let left_card_x = if marker_on_left {
            left_conn_x + 1
        } else {
            left_conn_x
        };
        let right_card_x_fn = |card_len: usize| -> i32 {
            if marker_on_left {
                right_conn_x - card_len as i32 + 1
            } else {
                right_conn_x - card_len as i32
            }
        };

        // from_cardinality is near the source box; only draw if source_y == card_y
        // (otherwise a vertical connector runs through that position)
        if let Some(ref card) = rel.from_cardinality {
            if source_y == card_y {
                let card_x = if from_is_left {
                    left_card_x
                } else {
                    right_card_x_fn(card.len())
                };
                draw_text(&mut canvas, card_x, card_y, card);
            }
        }
        // to_cardinality is near the target box (no vertical connector there)
        if let Some(ref card) = rel.to_cardinality {
            let card_x = if from_is_left {
                right_card_x_fn(card.len())
            } else {
                left_card_x
            };
            draw_text(&mut canvas, card_x, card_y, card);
        }

        // Label centered above the horizontal line
        if let Some(ref lbl) = rel.label {
            let mid_x = (left_conn_x + right_conn_x) / 2;
            let label_start = mid_x - lbl.len() as i32 / 2;
            draw_text(&mut canvas, label_start, card_y, lbl);
        }
    }

    Ok(canvas_to_string(&canvas))
}

fn get_marker_shape(rel_type: &RelationshipType, _is_hierarchical: bool, use_ascii: bool) -> char {
    match rel_type {
        RelationshipType::Inheritance | RelationshipType::Realization => {
            if use_ascii {
                '^'
            } else {
                '△'
            }
        }
        RelationshipType::Composition => {
            if use_ascii {
                '*'
            } else {
                '◆'
            }
        }
        RelationshipType::Aggregation => {
            if use_ascii {
                'o'
            } else {
                '◇'
            }
        }
        RelationshipType::Association | RelationshipType::Dependency => {
            if use_ascii {
                'v'
            } else {
                '▼'
            }
        }
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
    is_lollipop: bool,
}

fn format_member(member: &ClassMember) -> String {
    let vis = match member.visibility {
        Visibility::Public => "+",
        Visibility::Private => "-",
        Visibility::Protected => "#",
        Visibility::Package => "~",
        Visibility::None => "",
    };

    if member.is_method {
        let params = member.params.as_deref().unwrap_or("");
        let has_params = !params.is_empty();
        let ret = match &member.member_type {
            Some(t) if !t.eq_ignore_ascii_case("void") && !has_params => {
                format!(": {}", t)
            }
            _ => String::new(),
        };
        format!("{}{}({}){}", vis, member.name, params, ret)
    } else if let Some(ref t) = member.member_type {
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
        // Center annotation within the box (inner width = w - 2)
        let inner_w = (w - 2) as usize;
        let annot_offset = if annot_str.len() < inner_w {
            (inner_w - annot_str.len()) / 2
        } else {
            1
        };
        draw_text(canvas, x + 1 + annot_offset as i32, cur_y, &annot_str);
        set_char(canvas, x + w - 1, cur_y, v_line);
        cur_y += 1;
    }

    // Class name row (centered)
    set_char(canvas, x, cur_y, v_line);
    let inner_w = (w - 2) as usize;
    let name_offset = if cb.label.len() < inner_w {
        (inner_w - cb.label.len()) / 2
    } else {
        1
    };
    draw_text(canvas, x + 1 + name_offset as i32, cur_y, &cb.label);
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
