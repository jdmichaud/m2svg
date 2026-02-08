//! Parser module for Mermaid diagrams

pub mod class;
pub mod er;
pub mod flowchart;
pub mod gitgraph;
pub mod sequence;

use crate::types::{DiagramType, FrontmatterConfig, MermaidTheme, ParsedDiagram};

/// Parse Mermaid diagram text and return the diagram type plus frontmatter config
pub fn parse_mermaid(text: &str) -> Result<ParsedDiagram, String> {
    // Parse frontmatter for common config (theme, etc.)
    let (frontmatter, text_without_frontmatter) = parse_frontmatter(text);

    let lines: Vec<&str> = text_without_frontmatter
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("%%"))
        // Skip configuration lines like paddingX=, paddingY=, etc.
        .filter(|l| !l.contains('=') || l.contains("-->") || l.contains("--") || l.contains("->"))
        .collect();

    if lines.is_empty() {
        return Err("Empty mermaid diagram".to_string());
    }

    let header = lines[0].to_lowercase();

    let diagram = if header.starts_with("sequencediagram") {
        let diagram = sequence::parse_sequence_diagram(&lines)?;
        DiagramType::Sequence(diagram)
    } else if header.starts_with("classdiagram") {
        let diagram = class::parse_class_diagram(&lines)?;
        DiagramType::Class(diagram)
    } else if header.starts_with("erdiagram") {
        let diagram = er::parse_er_diagram(&lines)?;
        DiagramType::Er(diagram)
    } else if header.starts_with("statediagram") {
        let graph = flowchart::parse_state_diagram(&lines)?;
        DiagramType::Flowchart(graph)
    } else if header.starts_with("gitgraph") {
        let graph = gitgraph::parse_gitgraph_from_text(text, &frontmatter)?;
        DiagramType::GitGraph(graph)
    } else {
        let graph = flowchart::parse_flowchart(&lines)?;
        DiagramType::Flowchart(graph)
    };

    Ok(ParsedDiagram {
        diagram,
        frontmatter,
    })
}

/// Parse YAML frontmatter and return common config + remaining text.
/// This is the single source of truth for frontmatter extraction.
pub fn parse_frontmatter(text: &str) -> (FrontmatterConfig, String) {
    let lines: Vec<&str> = text.lines().collect();

    // Find opening ---
    let mut start = None;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "---" {
            start = Some(i);
        }
        break;
    }

    let start = match start {
        Some(s) => s,
        None => return (FrontmatterConfig::default(), text.to_string()),
    };

    // Find closing ---
    let mut end = None;
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        if line.trim() == "---" {
            end = Some(i);
            break;
        }
    }

    let end = match end {
        Some(e) => e,
        None => return (FrontmatterConfig::default(), text.to_string()),
    };

    // Collect the raw frontmatter lines (between the --- delimiters)
    let fm_lines: Vec<String> = lines[start + 1..end]
        .iter()
        .map(|l| l.to_string())
        .collect();
    let fm_text = fm_lines.join("\n");

    // Extract common config
    let mut config = FrontmatterConfig {
        theme: MermaidTheme::Default,
        raw_lines: fm_lines,
    };

    // Parse theme from frontmatter
    for line in fm_text.lines() {
        let trimmed = line.trim().trim_start_matches("- ");
        if let Some(val) = extract_yaml_value(trimmed, "theme:") {
            config.theme = MermaidTheme::from_str(val.trim().trim_matches('\'').trim_matches('"'));
        }
    }

    // Reconstruct text without frontmatter
    let remaining = lines[end + 1..].join("\n");

    (config, remaining)
}

/// Extract value after a YAML key (case-insensitive key match)
pub fn extract_yaml_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let lower = line.to_lowercase();
    let key_lower = key.to_lowercase();
    if let Some(pos) = lower.find(&key_lower) {
        let before = &lower[..pos];
        if before
            .chars()
            .all(|c| c.is_whitespace() || c == '\'' || c == '"')
        {
            let after = &line[pos + key.len()..];
            return Some(after.trim());
        }
    }
    None
}

/// Detect the diagram type from the mermaid source text
pub fn detect_diagram_type(text: &str) -> &'static str {
    let (_, text_clean) = parse_frontmatter(text);
    let first_line = text_clean
        .trim()
        .lines()
        .next()
        .map(|l| l.trim().to_lowercase())
        .unwrap_or_default();

    if first_line.starts_with("sequencediagram") {
        "sequence"
    } else if first_line.starts_with("classdiagram") {
        "class"
    } else if first_line.starts_with("erdiagram") {
        "er"
    } else if first_line.starts_with("gitgraph") {
        "gitgraph"
    } else {
        "flowchart"
    }
}
