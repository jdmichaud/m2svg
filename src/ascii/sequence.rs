//! Sequence diagram ASCII rendering

use crate::types::SequenceDiagram;
use super::types::AsciiConfig;
use super::canvas::{mk_canvas, canvas_to_string, set_char, draw_text};

/// Render a sequence diagram to ASCII
pub fn render_sequence_ascii(diagram: &SequenceDiagram, config: &AsciiConfig) -> Result<String, String> {
    if diagram.actors.is_empty() {
        return Ok(String::new());
    }
    
    let use_ascii = config.use_ascii;
    
    // Box-drawing characters
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    
    // Layout: compute lifeline X positions
    let box_pad = 1;
    let actor_box_widths: Vec<usize> = diagram.actors
        .iter()
        .map(|a| a.label.len() + 2 * box_pad + 2)
        .collect();
    let half_box: Vec<usize> = actor_box_widths.iter().map(|w| (w + 1) / 2).collect();
    let actor_box_h = 3; // top border + label row + bottom border
    
    // Compute minimum gap between adjacent lifelines
    let mut adj_max_width: Vec<usize> = vec![0; diagram.actors.len().saturating_sub(1)];
    
    let actor_idx: std::collections::HashMap<&str, usize> = diagram.actors
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id.as_str(), i))
        .collect();
    
    for msg in &diagram.messages {
        let fi = actor_idx.get(msg.from.as_str()).copied().unwrap_or(0);
        let ti = actor_idx.get(msg.to.as_str()).copied().unwrap_or(0);
        if fi == ti {
            continue; // self-messages don't affect spacing
        }
        let lo = fi.min(ti);
        let hi = fi.max(ti);
        let needed = msg.label.len() + 4;
        let num_gaps = hi - lo;
        let per_gap = (needed + num_gaps - 1) / num_gaps;
        for g in lo..hi {
            adj_max_width[g] = adj_max_width[g].max(per_gap);
        }
    }
    
    // Compute lifeline x-positions
    let mut ll_x: Vec<usize> = vec![half_box[0]];
    for i in 1..diagram.actors.len() {
        let gap = (half_box[i - 1] + half_box[i] + 2)
            .max(adj_max_width[i - 1] + 2)
            .max(10);
        ll_x.push(ll_x[i - 1] + gap);
    }
    
    // Compute vertical positions
    let mut msg_arrow_y: Vec<usize> = Vec::new();
    let mut cur_y = actor_box_h;
    
    for m in 0..diagram.messages.len() {
        cur_y += 1; // blank row before message
        
        let msg = &diagram.messages[m];
        let is_self = msg.from == msg.to;
        
        if is_self {
            msg_arrow_y.push(cur_y);
            cur_y += 3;
        } else {
            msg_arrow_y.push(cur_y + 1);
            cur_y += 2;
        }
    }
    
    cur_y += 1; // gap before footer
    let footer_y = cur_y;
    let total_h = footer_y + actor_box_h;
    
    // Total canvas width
    let last_ll = ll_x.last().copied().unwrap_or(0);
    let last_half = half_box.last().copied().unwrap_or(0);
    let mut total_w = last_ll + last_half + 2;
    
    // Ensure canvas is wide enough for self-message labels
    for (m, msg) in diagram.messages.iter().enumerate() {
        if msg.from == msg.to {
            let fi = actor_idx.get(msg.from.as_str()).copied().unwrap_or(0);
            let self_right = ll_x[fi] + 6 + 2 + msg.label.len();
            total_w = total_w.max(self_right + 1);
        }
    }
    
    let mut canvas = mk_canvas(total_w, total_h);
    
    // Draw actor boxes (header and footer)
    for (i, actor) in diagram.actors.iter().enumerate() {
        let cx = ll_x[i] as i32;
        let w = actor_box_widths[i] as i32;
        let half_w = w / 2;
        
        // Header box (top)
        draw_actor_box(&mut canvas, cx, 0, w, &actor.label, use_ascii);
        
        // Footer box (bottom)
        draw_actor_box(&mut canvas, cx, footer_y as i32, w, &actor.label, use_ascii);
        
        // Draw lifeline between boxes
        for y in actor_box_h..footer_y {
            set_char(&mut canvas, cx, y as i32, v_line);
        }
    }
    
    // Draw messages
    for (m, msg) in diagram.messages.iter().enumerate() {
        let fi = actor_idx.get(msg.from.as_str()).copied().unwrap_or(0);
        let ti = actor_idx.get(msg.to.as_str()).copied().unwrap_or(0);
        let arrow_y = msg_arrow_y[m] as i32;
        let is_self = fi == ti;
        
        if is_self {
            // Self-message: goes right, loops down, comes back with arrow
            let x = ll_x[fi] as i32;
            let corner_tr = if use_ascii { '+' } else { '┐' };
            let corner_bl = if use_ascii { '+' } else { '┘' };
            let arrow_left = if use_ascii { '<' } else { '◄' };
            let junction = if use_ascii { '+' } else { '├' };
            
            // Top line: junction on lifeline, then go right
            set_char(&mut canvas, x, arrow_y, junction);
            set_char(&mut canvas, x + 1, arrow_y, h_line);
            set_char(&mut canvas, x + 2, arrow_y, h_line);
            set_char(&mut canvas, x + 3, arrow_y, h_line);
            set_char(&mut canvas, x + 4, arrow_y, corner_tr);
            
            // Vertical down
            set_char(&mut canvas, x + 4, arrow_y + 1, v_line);
            
            // Bottom line: arrow left back to lifeline
            set_char(&mut canvas, x, arrow_y + 2, arrow_left);
            set_char(&mut canvas, x + 1, arrow_y + 2, h_line);
            set_char(&mut canvas, x + 2, arrow_y + 2, h_line);
            set_char(&mut canvas, x + 3, arrow_y + 2, h_line);
            set_char(&mut canvas, x + 4, arrow_y + 2, corner_bl);
            
            // Label on the right of the vertical line
            draw_text(&mut canvas, x + 6, arrow_y + 1, &msg.label);
        } else {
            // Normal message
            let from_x = ll_x[fi] as i32;
            let to_x = ll_x[ti] as i32;
            let is_dashed = msg.line_style == crate::types::LineStyle::Dashed;
            
            let (arrow_char, line_char) = if ti > fi {
                // Left to right
                (if use_ascii { '>' } else { '►' }, if is_dashed { '.' } else { h_line })
            } else {
                // Right to left
                (if use_ascii { '<' } else { '◄' }, if is_dashed { '.' } else { h_line })
            };
            
            let (start_x, end_x) = if ti > fi {
                (from_x + 1, to_x - 1)
            } else {
                (to_x + 1, from_x - 1)
            };
            
            // Draw arrow line
            for x in start_x..=end_x {
                set_char(&mut canvas, x, arrow_y, line_char);
            }
            
            // Draw arrowhead
            let arrow_x = if ti > fi { to_x } else { to_x };
            set_char(&mut canvas, arrow_x, arrow_y, arrow_char);
            
            // Draw label above the line
            let label_x = (from_x + to_x) / 2 - (msg.label.len() as i32) / 2;
            draw_text(&mut canvas, label_x, arrow_y - 1, &msg.label);
        }
    }
    
    Ok(canvas_to_string(&canvas))
}

fn draw_actor_box(canvas: &mut super::types::Canvas, cx: i32, top_y: i32, width: i32, label: &str, use_ascii: bool) {
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    
    let half_w = width / 2;
    let left = cx - half_w;
    let right = cx + half_w;
    
    // Top border
    set_char(canvas, left, top_y, tl);
    for x in (left + 1)..right {
        set_char(canvas, x, top_y, h_line);
    }
    set_char(canvas, right, top_y, tr);
    
    // Middle row (with label)
    set_char(canvas, left, top_y + 1, v_line);
    let label_x = cx - (label.len() as i32) / 2;
    draw_text(canvas, label_x, top_y + 1, label);
    set_char(canvas, right, top_y + 1, v_line);
    
    // Bottom border
    set_char(canvas, left, top_y + 2, bl);
    for x in (left + 1)..right {
        set_char(canvas, x, top_y + 2, h_line);
    }
    set_char(canvas, right, top_y + 2, br);
}
