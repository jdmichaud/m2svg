//! Parser module for Mermaid diagrams

pub mod flowchart;
pub mod sequence;
pub mod class;
pub mod er;

use crate::types::DiagramType;

/// Parse Mermaid diagram text and return the appropriate diagram type
pub fn parse_mermaid(text: &str) -> Result<DiagramType, String> {
    let lines: Vec<&str> = text
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
    
    if header.starts_with("sequencediagram") {
        let diagram = sequence::parse_sequence_diagram(&lines)?;
        Ok(DiagramType::Sequence(diagram))
    } else if header.starts_with("classdiagram") {
        let diagram = class::parse_class_diagram(&lines)?;
        Ok(DiagramType::Class(diagram))
    } else if header.starts_with("erdiagram") {
        let diagram = er::parse_er_diagram(&lines)?;
        Ok(DiagramType::Er(diagram))
    } else if header.starts_with("statediagram") {
        let graph = flowchart::parse_state_diagram(&lines)?;
        Ok(DiagramType::Flowchart(graph))
    } else {
        let graph = flowchart::parse_flowchart(&lines)?;
        Ok(DiagramType::Flowchart(graph))
    }
}

/// Detect the diagram type from the mermaid source text
pub fn detect_diagram_type(text: &str) -> &'static str {
    let first_line = text
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
    } else {
        "flowchart"
    }
}
