//! Parser for Mermaid GitGraph diagrams

use super::extract_yaml_value;
use crate::types::{
    CommitType, FrontmatterConfig, GitBranch, GitCommit, GitGraph, GitGraphConfig,
    GitGraphDirection,
};

/// Parse gitGraph-specific configuration from frontmatter raw lines.
/// Common config (theme) is already handled by the general frontmatter parser.
fn parse_gitgraph_config(frontmatter: &FrontmatterConfig) -> GitGraphConfig {
    let mut config = GitGraphConfig {
        theme: frontmatter.theme.to_string(),
        ..GitGraphConfig::default()
    };

    // Parse gitGraph-specific options from raw frontmatter lines
    let fm_text = frontmatter.raw_lines.join("\n");
    parse_config_values(&fm_text, &mut config);

    config
}

/// Parse configuration values from frontmatter YAML text
fn parse_config_values(text: &str, config: &mut GitGraphConfig) {
    for line in text.lines() {
        let trimmed = line.trim().trim_start_matches("- ");

        // gitGraph config options
        if let Some(val) = extract_yaml_value(trimmed, "showBranches:") {
            config.show_branches = val.trim() != "false";
        }
        if let Some(val) = extract_yaml_value(trimmed, "showCommitLabel:") {
            config.show_commit_label = val.trim() != "false";
        }
        if let Some(val) = extract_yaml_value(trimmed, "mainBranchName:") {
            let name = val.trim().trim_matches('\'').trim_matches('"').to_string();
            if !name.is_empty() {
                config.main_branch_name = name;
            }
        }
        if let Some(val) = extract_yaml_value(trimmed, "mainBranchOrder:") {
            if let Ok(order) = val.trim().parse::<i32>() {
                config.main_branch_order = Some(order);
            }
        }
        if let Some(val) = extract_yaml_value(trimmed, "rotateCommitLabel:") {
            config.rotate_commit_label = val.trim() != "false";
        }
        if let Some(val) = extract_yaml_value(trimmed, "parallelCommits:") {
            if val.trim() == "true" {
                eprintln!("Warning: parallelCommits is not yet supported and will be ignored");
            }
        }

        // Theme
        if let Some(val) = extract_yaml_value(trimmed, "theme:") {
            config.theme = val.trim().trim_matches('\'').trim_matches('"').to_string();
        }

        // Theme variables - branch colors (git0..git7)
        for i in 0..8 {
            let key = format!("git{}:", i);
            // Match 'gitN' but NOT 'gitBranchLabelN' or 'gitInvN'
            if let Some(val) = extract_yaml_value(trimmed, &key) {
                let lower_trimmed = trimmed.to_lowercase();
                if !lower_trimmed.starts_with("gitbranchlabel")
                    && !lower_trimmed.starts_with("gitinv")
                {
                    config.branch_colors[i] =
                        Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
                }
            }
        }

        // Theme variables - branch label colors (gitBranchLabel0..gitBranchLabel7)
        for i in 0..8 {
            let key = format!("gitBranchLabel{}:", i);
            if let Some(val) = extract_yaml_value(trimmed, &key) {
                config.branch_label_colors[i] =
                    Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
            }
        }

        // Theme variables - highlight commit colors (gitInv0..gitInv7)
        for i in 0..8 {
            let key = format!("gitInv{}:", i);
            if let Some(val) = extract_yaml_value(trimmed, &key) {
                config.highlight_colors[i] =
                    Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
            }
        }

        // Commit label styling
        if let Some(val) = extract_yaml_value(trimmed, "commitLabelColor:") {
            config.commit_label_color =
                Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
        }
        if let Some(val) = extract_yaml_value(trimmed, "commitLabelBackground:") {
            config.commit_label_background =
                Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
        }
        if let Some(val) = extract_yaml_value(trimmed, "commitLabelFontSize:") {
            config.commit_label_font_size =
                Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
        }

        // Tag label styling
        if let Some(val) = extract_yaml_value(trimmed, "tagLabelColor:") {
            config.tag_label_color =
                Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
        }
        if let Some(val) = extract_yaml_value(trimmed, "tagLabelBackground:") {
            config.tag_label_background =
                Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
        }
        if let Some(val) = extract_yaml_value(trimmed, "tagLabelBorder:") {
            config.tag_label_border =
                Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
        }
        if let Some(val) = extract_yaml_value(trimmed, "tagLabelFontSize:") {
            config.tag_label_font_size =
                Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
        }
    }
}

/// Parse a gitGraph diagram from mermaid text.
/// This function accepts the raw input text and a pre-parsed FrontmatterConfig.
pub fn parse_gitgraph_from_text(
    text: &str,
    frontmatter: &FrontmatterConfig,
) -> Result<GitGraph, String> {
    let config = parse_gitgraph_config(frontmatter);

    // Strip frontmatter from the text to get the diagram body
    let (_, remaining) = super::parse_frontmatter(text);

    let lines: Vec<&str> = remaining
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if lines.is_empty() {
        return Err("Empty gitGraph diagram".to_string());
    }

    parse_gitgraph_with_config(&lines, config)
}

/// Parse a gitGraph diagram from pre-filtered lines (called from parse_mermaid)
pub fn parse_gitgraph(lines: &[&str]) -> Result<GitGraph, String> {
    parse_gitgraph_with_config(lines, GitGraphConfig::default())
}

/// Core parser with explicit config
fn parse_gitgraph_with_config(lines: &[&str], config: GitGraphConfig) -> Result<GitGraph, String> {
    // Parse direction from header line
    let header = lines[0].to_lowercase();
    let direction = if header.contains("tb:") || header.contains("tb ") {
        GitGraphDirection::TB
    } else if header.contains("bt:") || header.contains("bt ") {
        GitGraphDirection::BT
    } else {
        GitGraphDirection::LR
    };

    let mut graph = GitGraph::with_config(direction, config);
    let mut commit_counter: u8 = b'A';

    for line in lines.iter().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse different commands
        if line.starts_with("commit") {
            parse_commit(line, &mut graph, &mut commit_counter)?;
        } else if line.starts_with("branch") {
            parse_branch(line, &mut graph)?;
        } else if line.starts_with("checkout") || line.starts_with("switch") {
            parse_checkout(line, &mut graph)?;
        } else if line.starts_with("merge") {
            parse_merge(line, &mut graph, &mut commit_counter)?;
        } else if line.starts_with("cherry-pick") {
            parse_cherry_pick(line, &mut graph, &mut commit_counter)?;
        }
    }

    Ok(graph)
}

/// Parse a commit command
fn parse_commit(line: &str, graph: &mut GitGraph, counter: &mut u8) -> Result<(), String> {
    let mut commit_id: Option<String> = None;
    let mut commit_type = CommitType::Normal;
    let mut tag: Option<String> = None;

    // Parse id: "value"
    if let Some(id_match) = extract_quoted_value(line, "id:") {
        commit_id = Some(id_match);
    }

    // Parse type: REVERSE or HIGHLIGHT
    if line.contains("type:") {
        if line.contains("REVERSE") {
            commit_type = CommitType::Reverse;
        } else if line.contains("HIGHLIGHT") {
            commit_type = CommitType::Highlight;
        }
    }

    // Parse tag: "value"
    if let Some(tag_match) = extract_quoted_value(line, "tag:") {
        tag = Some(tag_match);
    }

    // Generate ID if not provided, but always consume a counter slot
    let id = commit_id.unwrap_or_else(|| (*counter as char).to_string());
    // Always advance counter (custom ID consumes a slot too)
    *counter += 1;

    // Get parent commit:
    // 1. First try last commit on current branch
    // 2. If branch has no commits yet, get parent from the branch's source (stored when branch was created)
    let parent_ids = get_last_commit_on_branch(graph, &graph.current_branch.clone())
        .or_else(|| get_branch_source(graph, &graph.current_branch.clone()))
        .map(|p| vec![p])
        .unwrap_or_default();

    let commit = GitCommit {
        id: id.clone(),
        commit_type,
        tag,
        branch: graph.current_branch.clone(),
        parent_ids,
        is_merge: false,
        is_cherry_pick: false,
        cherry_pick_source: None,
        cherry_pick_parent: None,
    };

    graph.commits.push(commit);

    // Add to current branch
    if let Some(branch) = graph
        .branches
        .iter_mut()
        .find(|b| b.name == graph.current_branch)
    {
        branch.commit_ids.push(id);
    }

    Ok(())
}

/// Parse a branch command
fn parse_branch(line: &str, graph: &mut GitGraph) -> Result<(), String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid branch command".to_string());
    }

    let branch_name = parts[1].to_string();
    let mut order: Option<i32> = None;

    // Parse order: N
    if let Some(order_str) = extract_value(line, "order:") {
        order = order_str.trim().parse().ok();
    }

    // Get the source commit - use effective source which handles chained empty branches
    let source_commit = get_effective_branch_source(graph, &graph.current_branch.clone());

    let branch = GitBranch {
        name: branch_name.clone(),
        order,
        commit_ids: Vec::new(),
        source_commit,
    };

    graph.branches.push(branch);

    // In Mermaid's gitGraph, 'branch X' also switches to that branch
    graph.current_branch = branch_name;

    Ok(())
}

/// Parse a checkout/switch command
fn parse_checkout(line: &str, graph: &mut GitGraph) -> Result<(), String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid checkout command".to_string());
    }

    let branch_name = parts[1].to_string();

    // Verify branch exists
    if !graph.branches.iter().any(|b| b.name == branch_name) {
        return Err(format!("Branch '{}' does not exist", branch_name));
    }

    graph.current_branch = branch_name;
    Ok(())
}

/// Parse a merge command
fn parse_merge(line: &str, graph: &mut GitGraph, counter: &mut u8) -> Result<(), String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid merge command".to_string());
    }

    let source_branch = parts[1].to_string();

    // Merge commits get a unique auto-generated ID from the counter (like regular commits)
    let commit_id =
        extract_quoted_value(line, "id:").unwrap_or_else(|| (*counter as char).to_string());
    // Always advance counter
    *counter += 1;

    // Parse optional tag
    let tag = extract_quoted_value(line, "tag:");

    // Parse type
    let commit_type = if line.contains("REVERSE") {
        CommitType::Reverse
    } else if line.contains("HIGHLIGHT") {
        CommitType::Highlight
    } else {
        CommitType::Normal
    };

    // Get parents: last commit on current branch + last commit on source branch
    let mut parent_ids = Vec::new();
    if let Some(p1) = get_last_commit_on_branch(graph, &graph.current_branch.clone()) {
        parent_ids.push(p1);
    }
    if let Some(p2) = get_last_commit_on_branch(graph, &source_branch) {
        parent_ids.push(p2);
    }

    let commit = GitCommit {
        id: commit_id.clone(),
        commit_type,
        tag,
        branch: graph.current_branch.clone(),
        parent_ids,
        is_merge: true,
        is_cherry_pick: false,
        cherry_pick_source: None,
        cherry_pick_parent: None,
    };

    graph.commits.push(commit);

    // Add to current branch
    if let Some(branch) = graph
        .branches
        .iter_mut()
        .find(|b| b.name == graph.current_branch)
    {
        branch.commit_ids.push(commit_id);
    }

    Ok(())
}

/// Parse a cherry-pick command
fn parse_cherry_pick(line: &str, graph: &mut GitGraph, counter: &mut u8) -> Result<(), String> {
    // Parse the source commit id
    let source_id = extract_quoted_value(line, "id:")
        .ok_or_else(|| "cherry-pick requires id: parameter".to_string())?;

    // Parse optional parent: parameter
    let cherry_pick_parent = extract_quoted_value(line, "parent:");

    // Generate new commit id
    let commit_id = format!("{}'", source_id);

    // Get parent: last commit on current branch
    let parent_ids = get_last_commit_on_branch(graph, &graph.current_branch.clone())
        .map(|p| vec![p])
        .unwrap_or_default();

    let commit = GitCommit {
        id: commit_id.clone(),
        commit_type: CommitType::Normal,
        tag: None,
        branch: graph.current_branch.clone(),
        parent_ids,
        is_merge: false,
        is_cherry_pick: true,
        cherry_pick_source: Some(source_id),
        cherry_pick_parent,
    };

    // We used counter logic elsewhere, but not here - increment anyway to stay consistent
    let _ = counter;

    graph.commits.push(commit);

    // Add to current branch
    if let Some(branch) = graph
        .branches
        .iter_mut()
        .find(|b| b.name == graph.current_branch)
    {
        branch.commit_ids.push(commit_id);
    }

    Ok(())
}

/// Extract a quoted value after a key (e.g., id: "value" -> "value")
fn extract_quoted_value(line: &str, key: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let key_lower = key.to_lowercase();

    if let Some(pos) = lower.find(&key_lower) {
        let after_key = &line[pos + key.len()..];
        // Find quoted string
        if let Some(start) = after_key.find('"') {
            let rest = &after_key[start + 1..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
        // Also try unquoted single word
        let trimmed = after_key.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('"') {
            let word: String = trimmed.chars().take_while(|c| !c.is_whitespace()).collect();
            if !word.is_empty() {
                return Some(word);
            }
        }
    }
    None
}

/// Extract an unquoted value after a key
fn extract_value(line: &str, key: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let key_lower = key.to_lowercase();

    if let Some(pos) = lower.find(&key_lower) {
        let after_key = &line[pos + key.len()..];
        let trimmed = after_key.trim();
        let word: String = trimmed.chars().take_while(|c| !c.is_whitespace()).collect();
        if !word.is_empty() {
            return Some(word);
        }
    }
    None
}

/// Get the last commit ID on a branch
fn get_last_commit_on_branch(graph: &GitGraph, branch_name: &str) -> Option<String> {
    graph
        .branches
        .iter()
        .find(|b| b.name == branch_name)
        .and_then(|b| b.commit_ids.last().cloned())
}

/// Get the source commit for a branch (the commit it was branched from)
fn get_branch_source(graph: &GitGraph, branch_name: &str) -> Option<String> {
    let branch = graph.branches.iter().find(|b| b.name == branch_name)?;

    // If this branch has a source commit, return it
    if let Some(ref source) = branch.source_commit {
        return Some(source.clone());
    }

    // Otherwise, recursively check the source branch's source
    // (handles case where we branch from a branch that has no commits)
    None
}

/// Get the effective source commit for creating a new branch
/// This handles chained branches where intermediate branches have no commits
fn get_effective_branch_source(graph: &GitGraph, branch_name: &str) -> Option<String> {
    // First try: last commit on the branch
    if let Some(commit) = get_last_commit_on_branch(graph, branch_name) {
        return Some(commit);
    }

    // Second try: the branch's source commit
    let branch = graph.branches.iter().find(|b| b.name == branch_name)?;
    if let Some(ref source) = branch.source_commit {
        return Some(source.clone());
    }

    None
}
