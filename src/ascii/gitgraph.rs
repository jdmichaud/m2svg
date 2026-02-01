//! ASCII/Unicode renderer for GitGraph diagrams
//!
//! Renders git graphs with proper branch/merge visualization.

use crate::ascii::canvas::{mk_canvas, set_char, draw_text, canvas_to_string};
use crate::types::{GitGraph, GitGraphDirection};
use std::collections::{HashMap, HashSet};

/// Characters to use for rendering
struct GitChars {
    h_line: char,
    v_line: char,
    fork_down: char,   // \
    merge_up: char,    // /
}

impl GitChars {
    fn ascii() -> Self {
        Self {
            h_line: '-',
            v_line: '|',
            fork_down: '\\',
            merge_up: '/',
        }
    }

    fn unicode() -> Self {
        Self {
            h_line: '─',
            v_line: '│',
            fork_down: '╲',
            merge_up: '╱',
        }
    }
}

/// Render a GitGraph to ASCII/Unicode text
pub fn render_gitgraph(graph: &GitGraph, use_ascii: bool) -> String {
    match graph.direction {
        GitGraphDirection::LR => render_horizontal(graph, use_ascii),
        GitGraphDirection::TB => render_vertical_tb(graph, use_ascii),
        GitGraphDirection::BT => render_vertical_bt(graph, use_ascii),
    }
}

/// Render horizontal (left-to-right) git graph
/// 
/// Expected output format:
/// ```text
/// A---B-------[M]---F---G  (main)
///      \     /
///       C---D  (develop)
/// ```
/// 
/// Row layout:
/// - Row 0: branch 0 commits
/// - Row 1: connectors (\ and /)
/// - Row 2: branch 1 commits
/// etc.
fn render_horizontal(graph: &GitGraph, use_ascii: bool) -> String {
    let chars = if use_ascii { GitChars::ascii() } else { GitChars::unicode() };
    
    // Step 1: Assign branches to rows, respecting order attribute
    // Branches with order are sorted by order value
    // Branches without order keep their creation order but come after ordered ones
    // Exception: main branch always comes first if it has no order
    let mut sorted_branches: Vec<_> = graph.branches.iter().enumerate().collect();
    sorted_branches.sort_by(|(ia, a), (ib, b)| {
        match (&a.order, &b.order) {
            (Some(ao), Some(bo)) => ao.cmp(bo),
            (Some(_), None) => {
                // Ordered branch vs unordered: check if unordered is main
                if b.name == "main" { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less }
            },
            (None, Some(_)) => {
                // Unordered vs ordered: check if unordered is main
                if a.name == "main" { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
            },
            (None, None) => ia.cmp(ib), // preserve creation order
        }
    });
    
    let mut branch_rows: HashMap<String, usize> = HashMap::new();
    
    // Check if any branches have explicit order attributes
    let has_ordered_branches = sorted_branches.iter().any(|(_, b)| b.order.is_some());
    
    for (idx, (_, branch)) in sorted_branches.iter().enumerate() {
        let row = if has_ordered_branches {
            // When using order attributes: main at row 0, ordered branches at (order * 2 - 1)
            if idx == 0 {
                0
            } else if let Some(order) = branch.order {
                order as usize * 2 - 1
            } else {
                idx * 2
            }
        } else {
            // Default: 2-row spacing for all branches
            idx * 2
        };
        branch_rows.insert(branch.name.clone(), row);
    }
    let max_row = branch_rows.values().copied().max().unwrap_or(0);
    let total_height = max_row + 1;
    
    // Step 2: Identify forks, merges, and cherry-picks
    let mut fork_info: HashMap<String, String> = HashMap::new(); // first_commit_on_branch -> parent
    let mut merge_info: HashMap<String, String> = HashMap::new(); // merge_commit -> source_commit
    let mut cherry_pick_info: HashMap<String, String> = HashMap::new(); // cherry_pick_commit -> source_commit
    
    for commit in &graph.commits {
        if commit.is_merge && commit.parent_ids.len() >= 2 {
            if let Some(parent_id) = commit.parent_ids.get(1) {
                merge_info.insert(commit.id.clone(), parent_id.clone());
            }
        }
        
        // Track cherry-picks
        if commit.is_cherry_pick {
            if let Some(ref source_id) = commit.cherry_pick_source {
                cherry_pick_info.insert(commit.id.clone(), source_id.clone());
            }
        }
        
        if !commit.parent_ids.is_empty() {
            if let Some(parent_id) = commit.parent_ids.first() {
                if let Some(parent) = graph.commits.iter().find(|c| &c.id == parent_id) {
                    if parent.branch != commit.branch {
                        fork_info.insert(commit.id.clone(), parent_id.clone());
                    }
                }
            }
        }
    }
    
    // Find branches that have cherry-picks - we won't draw fork lines for these
    let branches_with_cherry_picks: HashSet<_> = cherry_pick_info.keys()
        .filter_map(|id| graph.commits.iter().find(|c| &c.id == id))
        .map(|c| c.branch.clone())
        .collect();
    
    // Group forks by parent to handle cascading forks (multiple branches from same commit)
    let mut forks_by_parent: HashMap<String, Vec<String>> = HashMap::new();
    for (child_id, parent_id) in &fork_info {
        forks_by_parent
            .entry(parent_id.clone())
            .or_default()
            .push(child_id.clone());
    }
    
    // Step 3: Layout - position all commits with consistent spacing
    let mut commit_cols: HashMap<String, usize> = HashMap::new();
    let mut branch_next_col: HashMap<String, usize> = HashMap::new();
    let base_spacing = 3; // "---" between commits
    
    for commit in &graph.commits {
        let label_len = if commit.is_merge { commit.id.len() + 2 } else { commit.id.len() };
        
        // Start with branch's current column
        let mut col = branch_next_col.get(&commit.branch).copied().unwrap_or(0);
        
        // If forking from another branch, position based on diagonal distance
        if let Some(parent_id) = fork_info.get(&commit.id) {
            if let Some(&parent_col) = commit_cols.get(parent_id) {
                if let Some(parent) = graph.commits.iter().find(|c| &c.id == parent_id) {
                    let parent_row = branch_rows[&parent.branch];
                    let child_row = branch_rows[&commit.branch];
                    let parent_len = parent.id.len();
                    
                    // Check if this is part of a cascading fork (multiple branches from same parent)
                    let siblings = forks_by_parent.get(parent_id).map(|v| v.len()).unwrap_or(1);
                    
                    let fork_col = if has_ordered_branches {
                        // For ordered branches: column = parent_end + diagonal distance
                        let row_diff = if child_row > parent_row { 
                            child_row - parent_row 
                        } else { 
                            parent_row - child_row 
                        };
                        parent_col + parent_len + row_diff.saturating_sub(1)
                    } else if siblings > 1 {
                        // Cascading fork: multiple branches from same parent
                        // Find the maximum row among all siblings
                        let max_sibling_row = forks_by_parent.get(parent_id)
                            .map(|sibs| {
                                sibs.iter()
                                    .filter_map(|sib_id| {
                                        graph.commits.iter()
                                            .find(|c| &c.id == sib_id)
                                            .map(|c| branch_rows[&c.branch])
                                    })
                                    .max()
                                    .unwrap_or(child_row)
                            })
                            .unwrap_or(child_row);
                        
                        let row_diff = if child_row > parent_row { 
                            child_row - parent_row 
                        } else { 
                            parent_row - child_row 
                        };
                        
                        if child_row == max_sibling_row {
                            // This is the furthest branch - position at diagonal end
                            // Diagonal advances once per row starting from row 1
                            parent_col + parent_len + row_diff - 1
                        } else {
                            // This branch is above furthest - needs horizontal connection
                            // Add extra space for horizontal connection after diagonal
                            parent_col + parent_len + row_diff + 3
                        }
                    } else {
                        // Single fork: just +1 for the fork line
                        parent_col + parent_len + 1
                    };
                    col = col.max(fork_col);
                }
            }
        }
        
        // If this is a merge, position after source branch end + merge line
        if let Some(source_id) = merge_info.get(&commit.id) {
            if let Some(&source_col) = commit_cols.get(source_id) {
                if let Some(source) = graph.commits.iter().find(|c| &c.id == source_id) {
                    let source_len = source.id.len();
                    let merge_col = source_col + source_len + 1; // +1 for the / line
                    col = col.max(merge_col);
                }
            }
        }
        
        // For cherry-picks: position at the source commit's column + offset for diagonal
        if let Some(source_id) = cherry_pick_info.get(&commit.id) {
            if let Some(&source_col) = commit_cols.get(source_id) {
                if let Some(source) = graph.commits.iter().find(|c| &c.id == source_id) {
                    let source_row = branch_rows[&source.branch];
                    let cherry_row = branch_rows[&commit.branch];
                    let source_len = if source.is_merge { source.id.len() + 2 } else { source.id.len() };
                    
                    // Position after source + diagonal distance
                    // Diagonal advances (row_diff - 1) columns (last step lands on target row)
                    let row_diff = if cherry_row > source_row { 
                        cherry_row - source_row 
                    } else { 
                        source_row - cherry_row 
                    };
                    let cherry_col = source_col + source_len + row_diff - 1;
                    col = col.max(cherry_col);
                }
            }
        }
        
        // Cherry-pick commits are invisible (don't take space) - their successor shows the connection
        let effective_len = if commit.is_cherry_pick { 0 } else { label_len };
        
        commit_cols.insert(commit.id.clone(), col);
        branch_next_col.insert(commit.branch.clone(), col + effective_len + if commit.is_cherry_pick { 0 } else { base_spacing });
    }
    
    // Step 4: Stretch child branches to fill space between fork and merge
    // For each branch that merges back, redistribute commits to fill the gap
    for branch in graph.branches.iter().skip(1) {
        // Get commits on this branch (excluding cherry-picks)
        let branch_commits: Vec<_> = graph.commits.iter()
            .filter(|c| c.branch == branch.name && !c.is_cherry_pick)
            .collect();
        
        if branch_commits.is_empty() {
            continue;
        }
        
        // Find if any commit on this branch is a merge parent
        // (the merge might happen before later commits are added to the branch)
        let mut merge_parent_idx = None;
        let mut merge_commit_ref = None;
        
        for (idx, branch_commit) in branch_commits.iter().enumerate() {
            if let Some(merge) = graph.commits.iter()
                .find(|c| c.is_merge && c.parent_ids.contains(&branch_commit.id))
            {
                merge_parent_idx = Some(idx);
                merge_commit_ref = Some(merge);
                break;
            }
        }
        
        if merge_commit_ref.is_none() {
            continue;
        }
        
        let merge = merge_commit_ref.unwrap();
        let merge_idx = merge_parent_idx.unwrap();
        let merge_col = commit_cols[&merge.id];
        
        // Only stretch commits up to and including the merge parent
        let commits_to_stretch: Vec<_> = branch_commits[..=merge_idx].to_vec();
        
        if commits_to_stretch.is_empty() {
            continue;
        }
        
        let first = commits_to_stretch.first().unwrap();
        let last = commits_to_stretch.last().unwrap();
        
        // First commit position (after fork)
        let first_col = commit_cols[&first.id];
        
        // Last commit (merge parent) should end at merge_col - 1 (for the / line)
        let last_len = last.id.len();
        let target_last_col = merge_col.saturating_sub(1).saturating_sub(last_len);
        
        // Only stretch if we need to (target is further right than current)
        let current_last_col = commit_cols[&last.id];
        if target_last_col <= current_last_col {
            continue;
        }
        
        // Redistribute commits to stretch
        let num_commits = commits_to_stretch.len();
        if num_commits == 1 {
            commit_cols.insert(first.id.clone(), target_last_col);
        } else {
            // Calculate total label lengths
            let total_labels: usize = commits_to_stretch.iter()
                .map(|c| c.id.len())
                .sum();
            
            // Available space for gaps
            let total_space = target_last_col + last_len - first_col;
            let gap_space = total_space.saturating_sub(total_labels);
            let num_gaps = num_commits - 1;
            let per_gap = if num_gaps > 0 { gap_space / num_gaps } else { 0 };
            let per_gap = per_gap.max(1);
            
            // Reposition commits
            let mut col = first_col;
            for commit in &commits_to_stretch {
                let label_len = commit.id.len();
                commit_cols.insert(commit.id.clone(), col);
                col += label_len + per_gap;
            }
        }
        
        // Update position of commits after merge parent (if any)
        // They should continue from where the stretched last commit ends
        if merge_idx + 1 < branch_commits.len() {
            let last_stretched = commits_to_stretch.last().unwrap();
            let last_stretched_end = commit_cols[&last_stretched.id] + last_stretched.id.len();
            let mut col = last_stretched_end + 3; // base_spacing
            
            for commit in &branch_commits[(merge_idx + 1)..] {
                let label_len = commit.id.len();
                commit_cols.insert(commit.id.clone(), col);
                col += label_len + 3;
            }
        }
    }
    
    // Build the canvas
    let max_col = commit_cols.values().max().copied().unwrap_or(0) + 30;
    let mut canvas = mk_canvas(max_col, total_height);
    
    // Step 5: Calculate branch spans (for drawing dashes)
    // end is the last column of the last commit (exclusive, so we use ..)
    // Skip cherry-pick commits (they're invisible)
    let mut branch_spans: HashMap<String, (usize, usize)> = HashMap::new();
    
    for commit in &graph.commits {
        if commit.is_cherry_pick {
            continue;  // Skip cherry-picks for span calculation
        }
        
        let c = commit_cols[&commit.id];
        let label_len = if commit.is_merge { commit.id.len() + 2 } else { commit.id.len() };
        
        branch_spans
            .entry(commit.branch.clone())
            .and_modify(|(start, end)| {
                *start = (*start).min(c);
                *end = (*end).max(c + label_len);
            })
            .or_insert((c, c + label_len));
    }
    
    // Step 6: Draw branch lines (dashes) - use exclusive end
    for (branch_name, (start, end)) in &branch_spans {
        let row = branch_rows[branch_name];
        for x in *start..*end {
            set_char(&mut canvas, x as i32, row as i32, chars.h_line);
        }
    }
    
    // Step 7: Draw fork lines - BEFORE commits so commits overwrite
    // Handle cascading forks: when multiple branches fork from same parent,
    // draw one continuous diagonal with horizontal branches to each child
    
    // First, draw cascading forks (grouped by parent)
    for (parent_id, children) in &forks_by_parent {
        if let Some(&parent_col) = commit_cols.get(parent_id) {
            if let Some(parent) = graph.commits.iter().find(|c| &c.id == parent_id) {
                let parent_row = branch_rows[&parent.branch];
                let parent_len = parent.id.len();
                
                // Find the furthest child row (for the continuous diagonal)
                // Include all children, even those with cherry-picks
                let mut max_child_row = parent_row;
                for child_id in children {
                    if let Some(child) = graph.commits.iter().find(|c| &c.id == child_id) {
                        let child_row = branch_rows[&child.branch];
                        if child_row > max_child_row {
                            max_child_row = child_row;
                        }
                    }
                }
                
                if max_child_row == parent_row {
                    continue; // No valid children to draw
                }
                
                // Draw the continuous diagonal from parent to furthest child
                // For cascading forks, draw on ALL rows (including branch rows) to reach lower branches
                let mut x = parent_col + parent_len;
                for row in (parent_row + 1)..=max_child_row {
                    // Don't draw diagonal on the final row - that's where the child commit is
                    if row == max_child_row {
                        break;
                    }
                    set_char(&mut canvas, x as i32, row as i32, chars.fork_down);
                    x += 1;
                }
                
                // For each child (except those with cherry-picks), draw horizontal connection
                for child_id in children {
                    if let Some(&child_col) = commit_cols.get(child_id) {
                        if let Some(child) = graph.commits.iter().find(|c| &c.id == child_id) {
                            // Skip horizontal connection for branches with cherry-picks
                            // (they get their connection from the cherry-pick source)
                            if branches_with_cherry_picks.contains(&child.branch) {
                                continue;
                            }
                            
                            let child_row = branch_rows[&child.branch];
                            if child_row > parent_row {
                                // Calculate where diagonal is at this child's row
                                let diag_x = if has_ordered_branches {
                                    parent_col + parent_len + (child_row - parent_row - 1)
                                } else {
                                    // For odd-row-only diagonals, count odd rows
                                    let odd_rows_before = (parent_row + 1..child_row).filter(|r| r % 2 == 1).count();
                                    parent_col + parent_len + odd_rows_before
                                };
                                
                                // Draw horizontal dashes from diagonal to child
                                let dash_start = diag_x + 1;
                                for dx in dash_start..child_col {
                                    set_char(&mut canvas, dx as i32, child_row as i32, chars.h_line);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Step 7b: Draw upward fork lines (/) when branch ordering puts parent below child
    // This happens when a child branch has a lower order number than its parent
    for (child_id, parent_id) in &fork_info {
        if let Some(&parent_col) = commit_cols.get(parent_id) {
            if let Some(parent) = graph.commits.iter().find(|c| &c.id == parent_id) {
                if let Some(child) = graph.commits.iter().find(|c| &c.id == child_id) {
                    let parent_row = branch_rows[&parent.branch];
                    let child_row = branch_rows[&child.branch];
                    
                    // Only handle upward forks (parent row > child row)
                    if parent_row > child_row {
                        let parent_len = parent.id.len();
                        // Draw / from parent upward to child
                        let mut x = parent_col + parent_len;
                        for row in (child_row + 1..parent_row).rev() {
                            set_char(&mut canvas, x as i32, row as i32, chars.merge_up);
                            x += 1;
                        }
                    }
                }
            }
        }
    }
    
    // Step 8: Draw merge lines (/) - BEFORE commits
    for (merge_id, source_id) in &merge_info {
        if let Some(&source_col) = commit_cols.get(source_id) {
            if let Some(source) = graph.commits.iter().find(|c| &c.id == source_id) {
                if let Some(merge) = graph.commits.iter().find(|c| &c.id == merge_id) {
                    let source_row = branch_rows[&source.branch];
                    let merge_row = branch_rows[&merge.branch];
                    
                    if source_row > merge_row {
                        let source_len = source.id.len();
                        let connector_row = source_row - 1;
                        let x = source_col + source_len;
                        set_char(&mut canvas, x as i32, connector_row as i32, chars.merge_up);
                    }
                }
            }
        }
    }
    
    // Step 8b: Draw cherry-pick lines (\) - from source commit down to cherry-pick target
    for (cherry_id, source_id) in &cherry_pick_info {
        if let Some(&source_col) = commit_cols.get(source_id) {
            if let Some(source) = graph.commits.iter().find(|c| &c.id == source_id) {
                if let Some(cherry) = graph.commits.iter().find(|c| &c.id == cherry_id) {
                    let source_row = branch_rows[&source.branch];
                    let cherry_row = branch_rows[&cherry.branch];
                    let source_len = if source.is_merge { source.id.len() + 2 } else { source.id.len() };
                    
                    if cherry_row > source_row {
                        // Cherry-pick target is below source: draw \ diagonal on all rows
                        let mut x = source_col + source_len;
                        for row in (source_row + 1)..cherry_row {
                            set_char(&mut canvas, x as i32, row as i32, chars.fork_down);
                            x += 1;
                        }
                    }
                }
            }
        }
    }
    
    // Step 9: Draw commits (overwriting dashes and fork lines)
    // Skip cherry-pick commits - they're invisible, connection shown via diagonal
    for commit in &graph.commits {
        if commit.is_cherry_pick {
            continue;  // Don't draw cherry-pick commits
        }
        
        let x = commit_cols[&commit.id];
        let row = branch_rows[&commit.branch];
        
        let label = if commit.is_merge {
            format!("[{}]", commit.id)
        } else {
            commit.id.clone()
        };
        
        draw_text(&mut canvas, x as i32, row as i32, &label);
    }
    
    // Step 10: Draw branch labels (right after last commit)
    // Account for any cherry-pick diagonals that might pass through this row
    for (branch_name, (_, end)) in &branch_spans {
        let row = branch_rows[branch_name];
        let label = format!("  ({})", branch_name);
        
        // Check if any cherry-pick diagonal passes through this row
        let mut label_pos = *end;
        for (cherry_id, source_id) in &cherry_pick_info {
            if let Some(&source_col) = commit_cols.get(source_id) {
                if let Some(source) = graph.commits.iter().find(|c| &c.id == source_id) {
                    if let Some(cherry) = graph.commits.iter().find(|c| &c.id == cherry_id) {
                        let source_row = branch_rows[&source.branch];
                        let cherry_row = branch_rows[&cherry.branch];
                        let source_len = if source.is_merge { source.id.len() + 2 } else { source.id.len() };
                        
                        // Check if this cherry-pick diagonal passes through our row
                        if source_row < row && row < cherry_row {
                            // Calculate diagonal position at this row
                            let diag_col = source_col + source_len + (row - source_row - 1);
                            // If diagonal is at or after our label position, push label right
                            if diag_col >= label_pos {
                                label_pos = label_pos.max(diag_col + 5); // Add padding after diagonal
                            }
                        }
                    }
                }
            }
        }
        
        draw_text(&mut canvas, label_pos as i32, row as i32, &label);
    }
    
    // Step 11: Handle tags
    let mut tag_lines: Vec<(usize, usize, String)> = Vec::new(); // (commit_x, commit_len, tag)
    for commit in &graph.commits {
        if let Some(ref tag) = commit.tag {
            let x = commit_cols[&commit.id];
            let commit_len = if commit.is_merge { commit.id.len() + 2 } else { commit.id.len() };
            tag_lines.push((x, commit_len, format!("[{}]", tag)));
        }
    }
    
    if !tag_lines.is_empty() {
        let mut output = String::new();
        let mut tag_label_line = String::new();
        let mut tag_connector_line = String::new();
        
        for (commit_x, commit_len, tag) in &tag_lines {
            // Center the tag over the commit
            let commit_center = commit_x + commit_len / 2;
            let tag_display_len = tag.chars().count();
            let tag_start = commit_center.saturating_sub(tag_display_len / 2);
            
            // Use chars().count() for display width, not byte length
            while tag_label_line.chars().count() < tag_start {
                tag_label_line.push(' ');
            }
            tag_label_line.push_str(tag);
            
            // Connector at commit center
            while tag_connector_line.chars().count() < commit_center {
                tag_connector_line.push(' ');
            }
            tag_connector_line.push(chars.v_line);
        }
        
        output.push_str(&tag_label_line.trim_end());
        output.push('\n');
        output.push_str(&tag_connector_line.trim_end());
        output.push('\n');
        output.push_str(&canvas_to_string(&canvas));
        return output;
    }
    
    canvas_to_string(&canvas)
}

/// Render vertical (top-to-bottom) git graph
/// 
/// Expected output format:
/// ```text
/// A  (main)
/// |
/// B
/// |\
/// | C  (develop)
/// | |
/// | D
/// E |
/// | |
/// F |
/// |/
/// [M]
/// ```
fn render_vertical_tb(graph: &GitGraph, use_ascii: bool) -> String {
    let chars = if use_ascii { GitChars::ascii() } else { GitChars::unicode() };
    
    // Assign branches to columns
    let mut branch_cols: HashMap<String, usize> = HashMap::new();
    for branch in &graph.branches {
        let col = branch_cols.len();
        branch_cols.insert(branch.name.clone(), col);
    }
    
    // Find fork and merge info
    let mut fork_commits: HashMap<String, String> = HashMap::new(); // child -> parent (fork point)
    let mut merge_commits: HashMap<String, String> = HashMap::new(); // merge -> source branch last commit
    let mut merge_source_commits: HashSet<String> = HashSet::new(); // commits that are merge sources
    
    for commit in &graph.commits {
        if commit.is_merge && commit.parent_ids.len() >= 2 {
            if let Some(parent_id) = commit.parent_ids.get(1) {
                merge_commits.insert(commit.id.clone(), parent_id.clone());
                merge_source_commits.insert(parent_id.clone());
            }
        }
        
        if !commit.parent_ids.is_empty() {
            if let Some(parent_id) = commit.parent_ids.first() {
                if let Some(parent) = graph.commits.iter().find(|c| &c.id == parent_id) {
                    if parent.branch != commit.branch {
                        fork_commits.insert(commit.id.clone(), parent_id.clone());
                    }
                }
            }
        }
    }
    
    // Build output line by line
    let mut lines: Vec<String> = Vec::new();
    let num_cols = branch_cols.len().max(1);
    
    // Track which branches are active at each point
    let mut active_branches: Vec<bool> = vec![false; num_cols];
    
    for (i, commit) in graph.commits.iter().enumerate() {
        let commit_col = branch_cols[&commit.branch];
        
        // Check if this is a fork point
        let is_fork = fork_commits.contains_key(&commit.id);
        let fork_parent_col = if is_fork {
            fork_commits.get(&commit.id)
                .and_then(|parent_id| graph.commits.iter().find(|c| &c.id == parent_id))
                .map(|parent| branch_cols[&parent.branch])
        } else {
            None
        };
        
        // Check if this is a merge commit
        let is_merge_commit = merge_commits.contains_key(&commit.id);
        let merge_source_col = if is_merge_commit {
            merge_commits.get(&commit.id)
                .and_then(|source_id| graph.commits.iter().find(|c| &c.id == source_id))
                .map(|source| branch_cols[&source.branch])
        } else {
            None
        };
        
        // Draw merge connector line BEFORE the merge commit (├──╯ style for unicode)
        if let Some(source_col) = merge_source_col {
            if source_col > commit_col {
                let mut merge_line = String::new();
                for c in 0..num_cols {
                    if c == commit_col {
                        if use_ascii {
                            merge_line.push(chars.v_line);
                            merge_line.push(chars.merge_up);
                        } else {
                            merge_line.push('├');
                            merge_line.push('─');
                            merge_line.push('─');
                        }
                    } else if c == source_col {
                        if use_ascii {
                            merge_line.push(' ');
                            merge_line.push(' ');
                        } else {
                            merge_line.push('╯');
                        }
                    } else if c > commit_col && c < source_col {
                        if use_ascii {
                            merge_line.push(' ');
                            merge_line.push(' ');
                        } else {
                            merge_line.push('─');
                            merge_line.push('─');
                        }
                    } else if active_branches[c] && c < source_col {
                        merge_line.push(chars.v_line);
                        if !use_ascii {
                            merge_line.push(' ');
                        } else {
                            merge_line.push(' ');
                        }
                    } else {
                        merge_line.push(' ');
                        if !use_ascii && c < source_col {
                            merge_line.push(' ');
                        } else {
                            merge_line.push(' ');
                        }
                    }
                }
                lines.push(merge_line.trim_end().to_string());
                
                // Deactivate the merged branch
                active_branches[source_col] = false;
            }
        }
        
        // Now activate this branch (after processing merge)
        active_branches[commit_col] = true;
        
        // Draw commit line - for forks in unicode, use ├── style on same line as commit
        let mut commit_line = String::new();
        
        if is_fork && !use_ascii {
            // Unicode fork: ├──C  (develop)
            if let Some(parent_col) = fork_parent_col {
                if commit_col > parent_col {
                    for c in 0..num_cols {
                        if c == parent_col {
                            commit_line.push('├');
                            commit_line.push('─');
                            commit_line.push('─');
                        } else if c == commit_col {
                            // Draw commit label
                            let label = if commit.is_merge {
                                format!("[{}]", commit.id)
                            } else {
                                commit.id.clone()
                            };
                            commit_line.push_str(&label);
                            
                            // Add branch label on first commit of each branch
                            let is_first_on_branch = graph.commits.iter()
                                .filter(|cc| cc.branch == commit.branch)
                                .next()
                                .map(|cc| cc.id == commit.id)
                                .unwrap_or(false);
                            
                            if is_first_on_branch {
                                commit_line.push_str(&format!("  ({})", commit.branch));
                            }
                        } else if c > parent_col && c < commit_col {
                            commit_line.push('─');
                            commit_line.push('─');
                        } else if active_branches[c] {
                            commit_line.push(chars.v_line);
                            commit_line.push(' ');
                        } else {
                            commit_line.push(' ');
                            commit_line.push(' ');
                        }
                    }
                    lines.push(commit_line.trim_end().to_string());
                    // Skip the normal commit line generation
                    
                    // Draw vertical connectors (if not last commit)
                    if i < graph.commits.len() - 1 {
                        let next_commit = &graph.commits[i + 1];
                        let next_is_fork = fork_commits.contains_key(&next_commit.id);
                        let next_is_merge = merge_commits.contains_key(&next_commit.id);
                        
                        if !next_is_fork && !next_is_merge {
                            let mut connector_line = String::new();
                            for c in 0..num_cols {
                                if active_branches[c] {
                                    connector_line.push(chars.v_line);
                                    connector_line.push(' ');
                                    if !use_ascii {
                                        connector_line.push(' ');
                                    }
                                } else {
                                    connector_line.push(' ');
                                    connector_line.push(' ');
                                    if !use_ascii {
                                        connector_line.push(' ');
                                    }
                                }
                            }
                            lines.push(connector_line.trim_end().to_string());
                        }
                    }
                    continue;
                }
            }
        }
        
        // ASCII fork: draw |\ on separate line before commit
        if is_fork && use_ascii {
            if let Some(parent_col) = fork_parent_col {
                if commit_col > parent_col {
                    let mut fork_line = String::new();
                    for c in 0..num_cols {
                        if c == parent_col {
                            fork_line.push(chars.v_line);
                            fork_line.push(chars.fork_down);
                        } else if c == commit_col {
                            // Don't draw anything at the commit column - the \ leads here
                            fork_line.push(' ');
                            fork_line.push(' ');
                        } else if active_branches[c] {
                            fork_line.push(chars.v_line);
                            fork_line.push(' ');
                        } else {
                            fork_line.push(' ');
                            fork_line.push(' ');
                        }
                    }
                    lines.push(fork_line.trim_end().to_string());
                }
            }
        }
        
        // Draw normal commit line
        for c in 0..num_cols {
            if c == commit_col {
                // Draw commit label
                let label = if commit.is_merge {
                    format!("[{}]", commit.id)
                } else {
                    commit.id.clone()
                };
                commit_line.push_str(&label);
                
                // Add branch label on first commit of each branch
                let is_first_on_branch = graph.commits.iter()
                    .filter(|cc| cc.branch == commit.branch)
                    .next()
                    .map(|cc| cc.id == commit.id)
                    .unwrap_or(false);
                
                if is_first_on_branch {
                    commit_line.push_str(&format!("  ({})", commit.branch));
                } else if c < num_cols - 1 && active_branches.iter().skip(c + 1).any(|&b| b) {
                    // If there are active branches after this commit, add spacing
                    let col_width = if use_ascii { 2 } else { 3 };
                    let needed_width = col_width * (c + 1);
                    while commit_line.chars().count() < needed_width {
                        commit_line.push(' ');
                    }
                }
            } else if active_branches[c] {
                commit_line.push(chars.v_line);
                commit_line.push(' ');
                if !use_ascii {
                    commit_line.push(' ');
                }
            } else {
                commit_line.push(' ');
                commit_line.push(' ');
                if !use_ascii {
                    commit_line.push(' ');
                }
            }
        }
        lines.push(commit_line.trim_end().to_string());
        
        // Draw vertical connectors (if not last commit)
        if i < graph.commits.len() - 1 {
            let next_commit = &graph.commits[i + 1];
            let next_is_fork = fork_commits.contains_key(&next_commit.id);
            let next_is_merge = merge_commits.contains_key(&next_commit.id);
            
            if !next_is_fork && !next_is_merge {
                let mut connector_line = String::new();
                for c in 0..num_cols {
                    if active_branches[c] {
                        connector_line.push(chars.v_line);
                        connector_line.push(' ');
                        if !use_ascii {
                            connector_line.push(' ');
                        }
                    } else {
                        connector_line.push(' ');
                        connector_line.push(' ');
                        if !use_ascii {
                            connector_line.push(' ');
                        }
                    }
                }
                lines.push(connector_line.trim_end().to_string());
            }
        }
    }
    
    lines.join("\n")
}

/// Render vertical (bottom-to-top) git graph
fn render_vertical_bt(graph: &GitGraph, use_ascii: bool) -> String {
    let chars = if use_ascii { GitChars::ascii() } else { GitChars::unicode() };
    
    // Render TB first, then reverse and swap fork/merge characters
    let tb_output = render_vertical_tb(graph, use_ascii);
    
    tb_output.lines()
        .rev()
        .map(|line| {
            // Swap fork_down (\) and merge_up (/) characters
            // Also swap ╯ ↔ ╮ for unicode
            line.chars()
                .map(|c| {
                    if c == chars.fork_down {
                        chars.merge_up
                    } else if c == chars.merge_up {
                        chars.fork_down
                    } else if c == '╯' {
                        '╮'
                    } else if c == '╮' {
                        '╯'
                    } else {
                        c
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
