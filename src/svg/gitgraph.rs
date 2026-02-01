//! SVG renderer for GitGraph diagrams

use super::DiagramColors;
use crate::types::{CommitType, GitGraph, GitGraphConfig, GitGraphDirection};
use std::collections::HashMap;

/// Render a GitGraph to SVG
pub fn render_gitgraph_svg(
    graph: &GitGraph,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    match graph.direction {
        GitGraphDirection::LR => render_horizontal_svg(graph, colors, font, transparent),
        GitGraphDirection::TB => render_vertical_svg(graph, colors, font, transparent, false),
        GitGraphDirection::BT => render_vertical_svg(graph, colors, font, transparent, true),
    }
}

/// Branch colors (matching mermaid.js default theme)
const BRANCH_COLORS: &[&str] = &[
    "#0000ED", // main - blue
    "#DEDC00", // develop - yellow
    "#00DE00", // feature - green
    "#0078D7", // other - azure
    "#00DED4", // cyan
    "#00DE76", // mint
    "#DE00DE", // magenta
    "#DE0000", // red
];

/// Get branch color, checking config overrides first
fn get_branch_color_with_config(branch_index: usize, config: &GitGraphConfig) -> String {
    let idx = branch_index % 8;
    if let Some(ref color) = config.branch_colors[idx] {
        color.clone()
    } else {
        BRANCH_COLORS[branch_index % BRANCH_COLORS.len()].to_string()
    }
}

/// Get highlight commit color, checking config overrides first
#[allow(dead_code)]
fn get_highlight_color_with_config(branch_index: usize, config: &GitGraphConfig) -> Option<String> {
    let idx = branch_index % 8;
    config.highlight_colors[idx].clone()
}

/// Get tag label styling from config
fn get_tag_fill(config: &GitGraphConfig) -> &str {
    config.tag_label_background.as_deref().unwrap_or("#FFFFDE")
}

fn get_tag_border(config: &GitGraphConfig) -> &str {
    config.tag_label_border.as_deref().unwrap_or("#333")
}

fn get_tag_text_fill(config: &GitGraphConfig) -> &str {
    config.tag_label_color.as_deref().unwrap_or("#333")
}

/// Draw a tag label centered above a commit as a rectangle badge.
/// Uses a simple rect + text. Width is estimated from character count.
fn draw_tag_label(svg: &mut String, cx: f64, tag_y: f64, tag_text: &str, config: &GitGraphConfig) {
    let char_w = 6.0_f64;
    let pad_x = 6.0_f64;
    let pad_y = 4.0_f64;
    let h = 16.0_f64;
    let w = (tag_text.len() as f64) * char_w + pad_x * 2.0;
    let rx = cx - w / 2.0;
    let ry = tag_y - h / 2.0;
    let tag_fill = get_tag_fill(config);
    let tag_border = get_tag_border(config);
    let tag_text_fill = get_tag_text_fill(config);
    let font_size = config.tag_label_font_size.as_deref().unwrap_or("10px");
    svg.push_str(&format!(
        r##"<rect x="{}" y="{}" width="{}" height="{}" rx="2" fill="{}" stroke="{}" stroke-width="1"/>"##,
        rx, ry, w, h, tag_fill, tag_border
    ));
    svg.push_str(&format!(
        r#"<text x="{}" y="{}" class="tag-text" text-anchor="middle" fill="{}" font-size="{}">{}</text>"#,
        cx, tag_y + pad_y, tag_text_fill, font_size, tag_text
    ));
}

/// Render horizontal (LR) git graph to SVG
fn render_horizontal_svg(
    graph: &GitGraph,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
) -> String {
    let commit_radius = 10.0;
    let commit_spacing_x = 50.0;
    let branch_spacing_y = 50.0;
    let label_margin = 80.0;
    let padding = 40.0;
    let left_offset = label_margin + padding;
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
    let mut x = left_offset;

    for commit in &graph.commits {
        let y = padding + (branch_rows[&commit.branch] as f64) * branch_spacing_y;
        commit_positions.insert(commit.id.clone(), (x, y));
        x += commit_spacing_x;
    }

    let width = x + padding;
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
</style>
<rect width="100%" height="100%" fill="{}"/>
"#,
        width,
        height,
        width,
        height,
        colors.surface.as_deref().unwrap_or(&colors.bg),
        font,
        colors.fg,
        font,
        colors.fg,
        font,
        bg_color
    ));

    // Draw branch lines (sorted by row for deterministic output)
    let mut sorted_branches: Vec<_> = branch_rows.iter().collect();
    sorted_branches.sort_by_key(|(_, &row)| row);

    for (branch_name, branch_row) in &sorted_branches {
        let y = padding + (**branch_row as f64) * branch_spacing_y;
        let color = get_branch_color_with_config(**branch_row, &graph.config);

        // Find first and last commit on this branch
        let commits_on_branch: Vec<_> = graph
            .commits
            .iter()
            .filter(|c| &c.branch == *branch_name)
            .collect();

        if let (Some(first), Some(last)) = (commits_on_branch.first(), commits_on_branch.last()) {
            let (x1, _) = commit_positions[&first.id];
            let (x2, _) = commit_positions[&last.id];
            let line_start = left_offset - 10.0;
            let line_end = width - padding;

            // Dashed grey line before first commit
            if x1 > line_start {
                svg.push_str(&format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="lightgrey" stroke-width="1" stroke-dasharray="2"/>"#,
                    line_start, y, x1, y
                ));
                svg.push('\n');
            }
            // Solid colored line between first and last commit
            if x2 > x1 {
                svg.push_str(&format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2"/>"#,
                    x1, y, x2, y, color
                ));
                svg.push('\n');
            }
            // Dashed grey line after last commit
            if line_end > x2 {
                svg.push_str(&format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="lightgrey" stroke-width="1" stroke-dasharray="2"/>"#,
                    x2, y, line_end, y
                ));
                svg.push('\n');
            }
        }
    }

    // Draw connections (branches and merges)
    // Track parents that have already used their horizontal exit (only one allowed)
    let mut used_horizontal_exit: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for commit in &graph.commits {
        let (cx, cy) = commit_positions[&commit.id];

        for parent_id in &commit.parent_ids {
            if let Some(&(px, py)) = commit_positions.get(parent_id) {
                let parent_branch = graph
                    .commits
                    .iter()
                    .find(|c| &c.id == parent_id)
                    .map(|c| &c.branch);

                let color = if let Some(pb) = parent_branch {
                    get_branch_color_with_config(*branch_rows.get(pb).unwrap_or(&0), &graph.config)
                } else {
                    get_branch_color_with_config(
                        *branch_rows.get(&commit.branch).unwrap_or(&0),
                        &graph.config,
                    )
                };

                if (cy - py).abs() > 1.0 {
                    // Connection shape depends on both ends:
                    //   Parent last on branch → exit horizontally (right)
                    //   Parent NOT last → exit vertically
                    //   Child first on branch → enter from side
                    //   Child NOT first → enter from top/bottom (S-curve)
                    let is_child_first = graph
                        .branches
                        .iter()
                        .find(|b| b.name == commit.branch)
                        .and_then(|b| b.commit_ids.first())
                        .map(|first_id| first_id == &commit.id)
                        .unwrap_or(false);

                    let is_parent_last_on_branch = parent_branch
                        .and_then(|pb| graph.branches.iter().find(|b| &b.name == pb))
                        .and_then(|b| b.commit_ids.last())
                        .map(|last_id| last_id == parent_id)
                        .unwrap_or(false);

                    // Only one merge link can exit horizontally from a last commit
                    let is_parent_last = if is_parent_last_on_branch {
                        if used_horizontal_exit.contains(parent_id) {
                            false
                        } else {
                            used_horizontal_exit.insert(parent_id.clone());
                            true
                        }
                    } else {
                        false
                    };

                    let arc_r = if is_child_first || is_parent_last {
                        20.0
                    } else {
                        10.0
                    };

                    if cy > py {
                        // Child is below parent
                        if is_parent_last && is_child_first {
                            // L-shape: horizontal right from parent, arc down, horizontal to child
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                px, cy - arc_r,
                                arc_r, arc_r,
                                px + arc_r, cy,
                                cx, cy,
                                color
                            ));
                        } else if is_parent_last {
                            // Parent exits right, child enters top: L-shape horizontal then down
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 1 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                cx - arc_r, py,
                                arc_r, arc_r,
                                cx, py + arc_r,
                                cx, cy,
                                color
                            ));
                        } else if is_child_first {
                            // Parent exits vertical, child enters side: L-shape down then right
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                px, cy - arc_r,
                                arc_r, arc_r,
                                px + arc_r, cy,
                                cx, cy,
                                color
                            ));
                        } else {
                            // S-curve: vertical to mid, arc, horizontal at mid, arc, vertical into child
                            let mid_y = (py + cy) / 2.0;
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {} A {} {} 0 0 1 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                px, mid_y - arc_r,
                                arc_r, arc_r,
                                px + arc_r, mid_y,
                                cx - arc_r, mid_y,
                                arc_r, arc_r,
                                cx, mid_y + arc_r,
                                cx, cy,
                                color
                            ));
                        }
                    } else {
                        // Child is above parent
                        if is_parent_last && is_child_first {
                            // L-shape: horizontal right from parent, arc up to child
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                cx - arc_r, py,
                                arc_r, arc_r,
                                cx, py - arc_r,
                                cx, cy,
                                color
                            ));
                        } else if is_parent_last {
                            // Parent exits right, child enters bottom: L-shape horizontal then up
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                cx - arc_r, py,
                                arc_r, arc_r,
                                cx, py - arc_r,
                                cx, cy,
                                color
                            ));
                        } else if is_child_first {
                            // Parent exits vertical, child enters side: shouldn't normally happen going up
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 1 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                px, cy + arc_r,
                                arc_r, arc_r,
                                px + arc_r, cy,
                                cx, cy,
                                color
                            ));
                        } else {
                            // S-curve: vertical up to mid, arc, horizontal at mid, arc, vertical into child
                            let mid_y = (py + cy) / 2.0;
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 1 {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                px, py,
                                px, mid_y + arc_r,
                                arc_r, arc_r,
                                px + arc_r, mid_y,
                                cx - arc_r, mid_y,
                                arc_r, arc_r,
                                cx, mid_y - arc_r,
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

    // Draw cherry-pick connections (bent line from source commit to cherry-pick position)
    for commit in &graph.commits {
        if commit.is_cherry_pick {
            if let Some(ref source_id) = commit.cherry_pick_source {
                if let Some(&(sx, sy)) = commit_positions.get(source_id) {
                    let (cx, cy) = commit_positions[&commit.id];
                    let source_branch = graph
                        .commits
                        .iter()
                        .find(|c| c.id == *source_id)
                        .map(|c| &c.branch);
                    let color = if let Some(sb) = source_branch {
                        get_branch_color_with_config(
                            *branch_rows.get(sb).unwrap_or(&0),
                            &graph.config,
                        )
                    } else {
                        get_branch_color_with_config(
                            *branch_rows.get(&commit.branch).unwrap_or(&0),
                            &graph.config,
                        )
                    };

                    if (cy - sy).abs() > 1.0 {
                        let arc_radius = 10.0;
                        let mid_y = (sy + cy) / 2.0;
                        if cy > sy {
                            // Source is above, cherry-pick is below: S-curve down
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 0 {} {} L {} {} A {} {} 0 0 1 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                sx, sy,
                                sx, mid_y - arc_radius,
                                arc_radius, arc_radius,
                                sx + arc_radius, mid_y,
                                cx - arc_radius, mid_y,
                                arc_radius, arc_radius,
                                cx, mid_y + arc_radius,
                                cx, cy,
                                color
                            ));
                        } else {
                            // Source is below, cherry-pick is above: S-curve up
                            svg.push_str(&format!(
                                r#"<path d="M {} {} L {} {} A {} {} 0 0 1 {} {} L {} {} A {} {} 0 0 0 {} {} L {} {}" stroke="{}" stroke-width="2" fill="none"/>"#,
                                sx, sy,
                                sx, mid_y + arc_radius,
                                arc_radius, arc_radius,
                                sx + arc_radius, mid_y,
                                cx - arc_radius, mid_y,
                                arc_radius, arc_radius,
                                cx, mid_y - arc_radius,
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
        let color = get_branch_color_with_config(branch_row, &graph.config);

        if commit.is_cherry_pick {
            // Cherry-pick icon: circle with two small dots and V-lines (cherry stems)
            svg.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="0"/>"#,
                cx, cy, commit_radius, color, color
            ));
            // Two small white circles (cherries)
            svg.push_str(&format!(
                r##"<circle cx="{}" cy="{}" r="2.75" fill="#fff"/>"##,
                cx - 3.0,
                cy + 2.0
            ));
            svg.push_str(&format!(
                r##"<circle cx="{}" cy="{}" r="2.75" fill="#fff"/>"##,
                cx + 3.0,
                cy + 2.0
            ));
            // V-shaped stems
            svg.push_str(&format!(
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#fff"/>"##,
                cx + 3.0,
                cy + 1.0,
                cx,
                cy - 5.0
            ));
            svg.push_str(&format!(
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#fff"/>"##,
                cx - 3.0,
                cy + 1.0,
                cx,
                cy - 5.0
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
            draw_tag_label(
                &mut svg,
                cx,
                cy - commit_radius - 15.0,
                &tag_text,
                &graph.config,
            );
            svg.push('\n');

            continue;
        }

        // Draw commit circle
        let color_str = color.as_str();
        let (fill, stroke, stroke_width): (&str, &str, f64) = match commit.commit_type {
            CommitType::Normal => (color_str, color_str, 0.0),
            CommitType::Reverse => (colors.bg.as_str(), color_str, 3.0),
            CommitType::Highlight => (color_str, "#FFD700", 3.0),
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
        if graph.config.show_commit_label {
            let font_size = graph
                .config
                .commit_label_font_size
                .as_deref()
                .unwrap_or("12px");
            let label_color = graph
                .config
                .commit_label_color
                .as_deref()
                .unwrap_or(&colors.fg);
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="commit-text" fill="{}" font-size="{}">{}</text>"#,
                cx,
                cy + commit_radius + label_offset,
                label_color,
                font_size,
                commit.id
            ));
            svg.push('\n');
        }

        // Draw tag if present
        if let Some(ref tag) = commit.tag {
            let tag_y = cy - commit_radius - 15.0;
            draw_tag_label(&mut svg, cx, tag_y, tag, &graph.config);
            svg.push('\n');
        }
    }

    // Draw branch labels on the left (sorted by row for deterministic output)
    if graph.config.show_branches {
        for (branch_name, branch_row) in &sorted_branches {
            let y = padding + (**branch_row as f64) * branch_spacing_y;
            let color = get_branch_color_with_config(**branch_row, &graph.config);

            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="branch-text" text-anchor="end" fill="{}">{}</text>"#,
                left_offset - 15.0,
                y + 4.0,
                color,
                branch_name
            ));
            svg.push('\n');
        }
    } // end show_branches

    svg.push_str("</svg>\n");
    svg
}

/// Render vertical (TB/BT) git graph to SVG
fn render_vertical_svg(
    graph: &GitGraph,
    colors: &DiagramColors,
    font: &str,
    transparent: bool,
    reverse: bool,
) -> String {
    let commit_radius = 10.0;
    let commit_spacing_y = 50.0;
    let branch_spacing_x = 50.0;
    let label_margin = 25.0;
    let padding = 40.0;
    let top_offset = padding + label_margin;

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
        let y = top_offset + (row as f64) * commit_spacing_y;
        commit_positions.insert(commit.id.clone(), (x, y));
    }

    let width = padding * 2.0 + (num_cols as f64) * branch_spacing_x + 100.0;
    let height = top_offset + padding + (num_commits as f64) * commit_spacing_y;

    let mut svg = String::new();

    let bg_color = if transparent { "none" } else { &colors.bg };
    svg.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
<style>
  .commit {{ fill: {}; }}
  .commit-text {{ font-family: '{}', sans-serif; font-size: 12px; fill: {}; }}
  .branch-text {{ font-family: '{}', sans-serif; font-size: 12px; fill: {}; }}
  .tag-text {{ font-family: '{}', sans-serif; font-size: 10px; fill: #333; }}
</style>
<rect width="100%" height="100%" fill="{}"/>
"#,
        width,
        height,
        width,
        height,
        colors.surface.as_deref().unwrap_or(&colors.bg),
        font,
        colors.fg,
        font,
        colors.fg,
        font,
        bg_color
    ));

    // Draw branch lines (sorted by col for deterministic output)
    let mut sorted_branches: Vec<_> = branch_cols.iter().collect();
    sorted_branches.sort_by_key(|(_, &col)| col);

    for (branch_name, branch_col) in &sorted_branches {
        let x = padding + (**branch_col as f64) * branch_spacing_x;
        let color = get_branch_color_with_config(**branch_col, &graph.config);

        let commits_on_branch: Vec<_> = graph
            .commits
            .iter()
            .filter(|c| &c.branch == *branch_name)
            .collect();

        if let (Some(first), Some(last)) = (commits_on_branch.first(), commits_on_branch.last()) {
            let (_, y1) = commit_positions[&first.id];
            let (_, y2) = commit_positions[&last.id];
            let (y_start, y_end) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            let line_top = top_offset - 10.0;
            let line_bottom = height - padding;

            // Dashed grey line before first commit
            if y_start > line_top {
                svg.push_str(&format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="lightgrey" stroke-width="1" stroke-dasharray="2"/>"#,
                    x, line_top, x, y_start
                ));
                svg.push('\n');
            }
            // Solid colored line between first and last commit
            if y_end > y_start {
                svg.push_str(&format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2"/>"#,
                    x, y_start, x, y_end, color
                ));
                svg.push('\n');
            }
            // Dashed grey line after last commit
            if line_bottom > y_end {
                svg.push_str(&format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="lightgrey" stroke-width="1" stroke-dasharray="2"/>"#,
                    x, y_end, x, line_bottom
                ));
                svg.push('\n');
            }
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
                    let color = get_branch_color_with_config(
                        *branch_cols.get(&commit.branch).unwrap_or(&0),
                        &graph.config,
                    );

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
        let color = get_branch_color_with_config(branch_col, &graph.config);

        let color_str = color.as_str();
        let (fill, stroke, stroke_width): (&str, &str, f64) = match commit.commit_type {
            CommitType::Normal => (color_str, color_str, 0.0),
            CommitType::Reverse => (colors.bg.as_str(), color_str, 3.0),
            CommitType::Highlight => (color_str, "#FFD700", 3.0),
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
        if graph.config.show_commit_label {
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="commit-text">{}</text>"#,
                cx + commit_radius + 5.0,
                cy + 4.0,
                commit.id
            ));
            svg.push('\n');
        }

        // Draw tag if present (to the left of the commit)
        if let Some(ref tag) = commit.tag {
            let tag_x = cx - commit_radius - 15.0;
            draw_tag_label(&mut svg, tag_x, cy, tag, &graph.config);
            svg.push('\n');
        }
    }

    // Draw branch labels at top (TB) or bottom (BT), sorted by col for deterministic output
    if graph.config.show_branches {
        for (branch_name, branch_col) in &sorted_branches {
            let x = padding + (**branch_col as f64) * branch_spacing_x;
            let color = get_branch_color_with_config(**branch_col, &graph.config);

            if reverse {
                // BT: labels at the bottom
                svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="branch-text" text-anchor="middle" fill="{}">{}</text>"#,
                x, height - padding + 20.0, color, branch_name
            ));
            } else {
                // TB: labels at the top
                svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="branch-text" text-anchor="middle" fill="{}">{}</text>"#,
                x, padding, color, branch_name
            ));
            }
            svg.push('\n');
        }
    } // end show_branches

    svg.push_str("</svg>\n");
    svg
}
