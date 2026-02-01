//! SVG renderer for GitGraph diagrams

use crate::types::{GitGraph, GitGraphDirection, CommitType};
use super::DiagramColors;
use std::collections::HashMap;

/// Render a GitGraph to SVG
pub fn render_gitgraph_svg(graph: &GitGraph, colors: &DiagramColors, font: &str, transparent: bool) -> String {
    match graph.direction {
        GitGraphDirection::LR => render_horizontal_svg(graph, colors, font, transparent),
        GitGraphDirection::TB => render_vertical_svg(graph, colors, font, transparent, false),
        GitGraphDirection::BT => render_vertical_svg(graph, colors, font, transparent, true),
    }
}

/// Branch colors (matching mermaid.js default theme)
const BRANCH_COLORS: &[&str] = &[
    "#0000ED",  // main - blue
    "#DEDC00",  // develop - yellow
    "#00DE00",  // feature - green
    "#0078D7",  // other - azure
    "#00DED4",  // cyan
    "#00DE76",  // mint
    "#DE00DE",  // magenta
    "#DE0000",  // red
];

fn get_branch_color(branch_index: usize) -> &'static str {
    BRANCH_COLORS[branch_index % BRANCH_COLORS.len()]
}

/// Render horizontal (LR) git graph to SVG
fn render_horizontal_svg(graph: &GitGraph, colors: &DiagramColors, font: &str, transparent: bool) -> String {
    let commit_radius = 10.0;
    let commit_spacing_x = 50.0;
    let branch_spacing_y = 50.0;
    let padding = 40.0;
    let label_offset = 20.0;
    
    // Assign branches to rows
    let mut branch_rows: HashMap<String, usize> = HashMap::new();
    
    // main first
    for branch in &graph.branches {
        if branch.name == "main" {
            branch_rows.insert(branch.name.clone(), 0);
        }
    }
    let mut row = 1;
    
    for branch in &graph.branches {
        if branch.name != "main" && !branch_rows.contains_key(&branch.name) {
            branch_rows.insert(branch.name.clone(), row);
            row += 1;
        }
    }
    
    let num_rows = row.max(1);
    
    // Calculate commit positions (skip cherry-picks in x advancement)
    let mut commit_positions: HashMap<String, (f64, f64)> = HashMap::new();
    let mut x = padding;
    
    for commit in &graph.commits {
        let y = padding + (branch_rows[&commit.branch] as f64) * branch_spacing_y;
        commit_positions.insert(commit.id.clone(), (x, y));
        x += commit_spacing_x;
    }
    
    let width = x + padding + 100.0; // Extra space for branch labels
    let height = padding * 2.0 + (num_rows as f64) * branch_spacing_y;
    
    let mut svg = String::new();
    
    // SVG header
    let bg_color = if transparent { "none" } else { &colors.bg };
    svg.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
<style>
  .commit {{ fill: {}; }}
  .commit-text {{ font-family: '{}', sans-serif; font-size: 12px; fill: {}; text-anchor: middle; }}
  .branch-text {{ font-family: '{}', sans-serif; font-size: 12px; fill: {}; }}
  .tag-text {{ font-family: '{}', sans-serif; font-size: 10px; fill: #333; }}
  .tag-bg {{ fill: #FFFFDE; stroke: #333; stroke-width: 1; }}
</style>
<rect width="100%" height="100%" fill="{}"/>
"#,
        width, height, width, height,
        colors.surface.as_deref().unwrap_or(&colors.bg),
        font, colors.fg,
        font, colors.fg,
        font,
        bg_color
    ));
    
    // Draw branch lines (sorted by row for deterministic output)
    let mut sorted_branches: Vec<_> = branch_rows.iter().collect();
    sorted_branches.sort_by_key(|(_, &row)| row);
    
    for (branch_name, branch_row) in &sorted_branches {
        let y = padding + (**branch_row as f64) * branch_spacing_y;
        let color = get_branch_color(**branch_row);
        
        // Find first and last commit on this branch
        let commits_on_branch: Vec<_> = graph.commits.iter()
            .filter(|c| &c.branch == *branch_name)
            .collect();
        
        if let (Some(first), Some(last)) = (commits_on_branch.first(), commits_on_branch.last()) {
            let (x1, _) = commit_positions[&first.id];
            let (x2, _) = commit_positions[&last.id];
            
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2"/>"#,
                x1, y, x2, y, color
            ));
            svg.push('\n');
        }
    }
    
    // Draw connections (branches and merges)
    for commit in &graph.commits {
        let (cx, cy) = commit_positions[&commit.id];
        
        for parent_id in &commit.parent_ids {
            if let Some(&(px, py)) = commit_positions.get(parent_id) {
                let parent_branch = graph.commits.iter()
                    .find(|c| &c.id == parent_id)
                    .map(|c| &c.branch);
                
                let color = if let Some(pb) = parent_branch {
                    get_branch_color(*branch_rows.get(pb).unwrap_or(&0))
                } else {
                    get_branch_color(*branch_rows.get(&commit.branch).unwrap_or(&0))
                };
                
                if (cy - py).abs() > 1.0 {
                    // Different branches - draw line-arc-line path like mermaid.js
                    // Goes along source branch first, then arcs to target branch
                    let arc_radius = 20.0;
                    
                    if cy > py {
                        // Going down (branching): vertical down first, arc, then horizontal
                        svg.push_str(&format!(
                            r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                            px, py,
                            px, cy - arc_radius,
                            arc_radius, arc_radius,
                            px + arc_radius, cy,
                            cx, cy,
                            color
                        ));
                    } else {
                        // Going up (merging): horizontal first, arc, then vertical up
                        svg.push_str(&format!(
                            r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                            px, py,
                            cx - arc_radius, py,
                            arc_radius, arc_radius,
                            cx, py - arc_radius,
                            cx, cy,
                            color
                        ));
                    }
                    svg.push('\n');
                }
            }
        }
    }
    
    // Draw cherry-pick connections (bent line from source commit to cherry-pick position)
    for commit in &graph.commits {
        if commit.is_cherry_pick {
            if let Some(ref source_id) = commit.cherry_pick_source {
                if let Some(&(sx, sy)) = commit_positions.get(source_id) {
                    let (cx, cy) = commit_positions[&commit.id];
                    let source_branch = graph.commits.iter()
                        .find(|c| c.id == *source_id)
                        .map(|c| &c.branch);
                    let color = if let Some(sb) = source_branch {
                        get_branch_color(*branch_rows.get(sb).unwrap_or(&0))
                    } else {
                        get_branch_color(*branch_rows.get(&commit.branch).unwrap_or(&0))
                    };

                    if (cy - sy).abs() > 1.0 {
                        let arc_radius = 20.0;
                        if cy > sy {
                            // Source is above, cherry-pick is below: vertical down, arc, horizontal
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                sx, sy,
                                sx, cy - arc_radius,
                                arc_radius, arc_radius,
                                sx + arc_radius, cy,
                                cx, cy,
                                color
                            ));
                        } else {
                            // Source is below, cherry-pick is above: horizontal, arc, vertical up
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                sx, sy,
                                cx - arc_radius, sy,
                                arc_radius, arc_radius,
                                cx, sy - arc_radius,
                                cx, cy,
                                color
                            ));
                        }
                        svg.push('\n');
                    }
                }
            }
        }
    }
    
    // Draw commits
    for commit in &graph.commits {
        let (cx, cy) = commit_positions[&commit.id];
        let branch_row = *branch_rows.get(&commit.branch).unwrap_or(&0);
        let color = get_branch_color(branch_row);
        
        if commit.is_cherry_pick {
            // Cherry-pick icon: circle with two small dots and V-lines (cherry stems)
            svg.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="0"/>"#,
                cx, cy, commit_radius, color, color
            ));
            // Two small white circles (cherries)
            svg.push_str(&format!(
                r##"<circle cx="{}" cy="{}" r="2.75" fill="#fff"/>"##,
                cx - 3.0, cy + 2.0
            ));
            svg.push_str(&format!(
                r##"<circle cx="{}" cy="{}" r="2.75" fill="#fff"/>"##,
                cx + 3.0, cy + 2.0
            ));
            // V-shaped stems
            svg.push_str(&format!(
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#fff"/>"##,
                cx + 3.0, cy + 1.0, cx, cy - 5.0
            ));
            svg.push_str(&format!(
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#fff"/>"##,
                cx - 3.0, cy + 1.0, cx, cy - 5.0
            ));
            svg.push('\n');
            
            // Cherry-pick tag label
            let source_id = commit.cherry_pick_source.as_deref().unwrap_or("");
            let parent_id = commit.cherry_pick_parent.as_deref().unwrap_or("");
            let tag_text = if !parent_id.is_empty() {
                format!("cherry-pick:{}|parent:{}", source_id, parent_id)
            } else {
                format!("cherry-pick:{}", source_id)
            };
            let tag_width = (tag_text.len() as f64) * 5.2 + 10.0;
            let tag_x = cx - tag_width / 2.0 + 8.0;
            let tag_y = cy - commit_radius - 15.0;
            // Tag polygon (arrow shape pointing left like mermaid)
            let poly_left = tag_x;
            let poly_top = tag_y - 8.5;
            let poly_bottom = tag_y + 8.5;
            let poly_right = tag_x + tag_width;
            let poly_arrow_x = poly_left + 8.0;
            let poly_arrow_y = tag_y;
            svg.push_str(&format!(
                r##"<polygon points="{},{} {},{} {},{} {},{} {},{} {},{}" fill="#FFFFDE" stroke="#333" stroke-width="1"/>"##,
                poly_left, poly_arrow_y,
                poly_arrow_x, poly_top,
                poly_right, poly_top,
                poly_right, poly_bottom,
                poly_arrow_x, poly_bottom,
                poly_left, poly_arrow_y
            ));
            // Tag hole (small circle)
            svg.push_str(&format!(
                r##"<circle cx="{}" cy="{}" r="1.5" fill="#333"/>"##,
                poly_left + 4.0, poly_arrow_y
            ));
            // Tag text
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="tag-text">{}</text>"#,
                poly_arrow_x + 4.0, tag_y + 3.5, tag_text
            ));
            svg.push('\n');
            
            continue;
        }
        
        // Draw commit circle
        let (fill, stroke, stroke_width): (&str, &str, f64) = match commit.commit_type {
            CommitType::Normal => (color, color, 0.0),
            CommitType::Reverse => (colors.bg.as_str(), color, 3.0),
            CommitType::Highlight => (color, "#FFD700", 3.0),
        };
        
        if commit.is_merge {
            // Merge commits get a diamond shape
            svg.push_str(&format!(
                r#"<polygon points="{},{} {},{} {},{} {},{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                cx, cy - commit_radius,
                cx + commit_radius, cy,
                cx, cy + commit_radius,
                cx - commit_radius, cy,
                fill, stroke, stroke_width.max(1.0)
            ));
        } else {
            svg.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                cx, cy, commit_radius, fill, stroke, stroke_width
            ));
        }
        svg.push('\n');
        
        // Draw commit ID
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" class="commit-text">{}</text>"#,
            cx, cy + commit_radius + label_offset, commit.id
        ));
        svg.push('\n');
        
        // Draw tag if present
        if let Some(ref tag) = commit.tag {
            let tag_y = cy - commit_radius - 15.0;
            let tag_width = (tag.len() as f64) * 7.0 + 10.0;
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="18" rx="3" class="tag-bg"/>"#,
                cx - tag_width / 2.0, tag_y - 12.0, tag_width
            ));
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="tag-text" text-anchor="middle">{}</text>"#,
                cx, tag_y, tag
            ));
            svg.push('\n');
        }
    }
    
    // Draw branch labels (sorted by row for deterministic output)
    for (branch_name, branch_row) in &sorted_branches {
        let y = padding + (**branch_row as f64) * branch_spacing_y;
        
        // Find last commit on this branch
        if let Some(last_commit) = graph.commits.iter()
            .filter(|c| &c.branch == *branch_name)
            .last() 
        {
            let (x, _) = commit_positions[&last_commit.id];
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="branch-text">({})</text>"#,
                x + commit_radius + 10.0, y + 4.0, branch_name
            ));
            svg.push('\n');
        }
    }
    
    svg.push_str("</svg>\n");
    svg
}

/// Render vertical (TB/BT) git graph to SVG
fn render_vertical_svg(graph: &GitGraph, colors: &DiagramColors, font: &str, transparent: bool, reverse: bool) -> String {
    let commit_radius = 10.0;
    let commit_spacing_y = 50.0;
    let branch_spacing_x = 50.0;
    let padding = 40.0;
    
    // Assign branches to columns
    let mut branch_cols: HashMap<String, usize> = HashMap::new();
    branch_cols.insert("main".to_string(), 0);
    let mut col = 1;
    
    for branch in &graph.branches {
        if branch.name != "main" && !branch_cols.contains_key(&branch.name) {
            branch_cols.insert(branch.name.clone(), col);
            col += 1;
        }
    }
    
    let num_cols = col.max(1);
    
    // Calculate commit positions
    let mut commit_positions: HashMap<String, (f64, f64)> = HashMap::new();
    let num_commits = graph.commits.len();
    
    for (i, commit) in graph.commits.iter().enumerate() {
        let x = padding + (branch_cols[&commit.branch] as f64) * branch_spacing_x;
        let row = if reverse { num_commits - 1 - i } else { i };
        let y = padding + (row as f64) * commit_spacing_y;
        commit_positions.insert(commit.id.clone(), (x, y));
    }
    
    let width = padding * 2.0 + (num_cols as f64) * branch_spacing_x + 100.0;
    let height = padding * 2.0 + (num_commits as f64) * commit_spacing_y;
    
    let mut svg = String::new();
    
    let bg_color = if transparent { "none" } else { &colors.bg };
    svg.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
<style>
  .commit {{ fill: {}; }}
  .commit-text {{ font-family: '{}', sans-serif; font-size: 12px; fill: {}; }}
  .branch-text {{ font-family: '{}', sans-serif; font-size: 12px; fill: {}; }}
</style>
<rect width="100%" height="100%" fill="{}"/>
"#,
        width, height, width, height,
        colors.surface.as_deref().unwrap_or(&colors.bg),
        font, colors.fg,
        font, colors.fg,
        bg_color
    ));
    
    // Draw branch lines (sorted by col for deterministic output)
    let mut sorted_branches: Vec<_> = branch_cols.iter().collect();
    sorted_branches.sort_by_key(|(_, &col)| col);
    
    for (branch_name, branch_col) in &sorted_branches {
        let x = padding + (**branch_col as f64) * branch_spacing_x;
        let color = get_branch_color(**branch_col);
        
        let commits_on_branch: Vec<_> = graph.commits.iter()
            .filter(|c| &c.branch == *branch_name)
            .collect();
        
        if let (Some(first), Some(last)) = (commits_on_branch.first(), commits_on_branch.last()) {
            let (_, y1) = commit_positions[&first.id];
            let (_, y2) = commit_positions[&last.id];
            
            let (y_start, y_end) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2"/>"#,
                x, y_start, x, y_end, color
            ));
            svg.push('\n');
        }
    }
    
    // Draw connections
    for commit in &graph.commits {
        let (cx, cy) = commit_positions[&commit.id];
        
        for parent_id in &commit.parent_ids {
            if let Some(&(px, py)) = commit_positions.get(parent_id) {
                if (cx - px).abs() > 1.0 {
                    // Different branches - draw line-arc-line path like mermaid.js
                    // Always go along source branch (vertical) first, then arc, then horizontal to target
                    let arc_radius = 20.0;
                    let color = get_branch_color(*branch_cols.get(&commit.branch).unwrap_or(&0));
                    
                    if reverse {
                        // Bottom-to-top: flow goes upward (py > cy, i.e., parent Y is greater)
                        if cx > px {
                            // Branching right: horizontal RIGHT from parent first, arc up (counter-clockwise to bulge bottom-right), then vertical UP to child
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                cx - arc_radius, py,
                                arc_radius, arc_radius,
                                cx, py - arc_radius,
                                cx, cy,
                                color
                            ));
                        } else {
                            // Merging left: vertical UP from parent first, arc left (counter-clockwise), then horizontal to child
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                px, cy + arc_radius,
                                arc_radius, arc_radius,
                                px - arc_radius, cy,
                                cx, cy,
                                color
                            ));
                        }
                    } else {
                        // Top-to-bottom: flow goes downward (cy > py, i.e., child Y is greater)
                        if cx > px {
                            // Branching right: horizontal RIGHT from parent first, arc down, then vertical DOWN to child (entering from top)
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 1 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                cx - arc_radius, py,
                                arc_radius, arc_radius,
                                cx, py + arc_radius,
                                cx, cy,
                                color
                            ));
                        } else {
                            // Merging left: vertical DOWN from parent first, arc left (clockwise to bulge bottom-left), then horizontal to child
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 1 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                px, cy - arc_radius,
                                arc_radius, arc_radius,
                                px - arc_radius, cy,
                                cx, cy,
                                color
                            ));
                        }
                    }
                    svg.push('\n');
                }
            }
        }
    }
    
    // Draw commits
    for commit in &graph.commits {
        let (cx, cy) = commit_positions[&commit.id];
        let branch_col = *branch_cols.get(&commit.branch).unwrap_or(&0);
        let color = get_branch_color(branch_col);
        
        let (fill, stroke, stroke_width): (&str, &str, f64) = match commit.commit_type {
            CommitType::Normal => (color, color, 0.0),
            CommitType::Reverse => (colors.bg.as_str(), color, 3.0),
            CommitType::Highlight => (color, "#FFD700", 3.0),
        };
        
        if commit.is_merge {
            svg.push_str(&format!(
                r#"<polygon points="{},{} {},{} {},{} {},{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                cx, cy - commit_radius,
                cx + commit_radius, cy,
                cx, cy + commit_radius,
                cx - commit_radius, cy,
                fill, stroke, stroke_width.max(1.0)
            ));
        } else {
            svg.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                cx, cy, commit_radius, fill, stroke, stroke_width
            ));
        }
        svg.push('\n');
        
        // Draw commit ID to the right
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" class="commit-text">{}</text>"#,
            cx + commit_radius + 5.0, cy + 4.0, commit.id
        ));
        svg.push('\n');
    }
    
    // Draw branch labels (sorted by col for deterministic output)
    for (branch_name, _branch_col) in &sorted_branches {
        if let Some(first_commit) = graph.commits.iter()
            .filter(|c| &c.branch == *branch_name)
            .next() 
        {
            let (x, y) = commit_positions[&first_commit.id];
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="branch-text">({})</text>"#,
                x + commit_radius + 5.0 + first_commit.id.len() as f64 * 8.0,
                y + 4.0,
                branch_name
            ));
            svg.push('\n');
        }
    }
    
    svg.push_str("</svg>\n");
    svg
}
