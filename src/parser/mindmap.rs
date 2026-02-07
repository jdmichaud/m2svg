/// Parser for Mermaid mindmap diagrams

#[derive(Debug, Clone, PartialEq)]
pub enum NodeShape {
    Default, // plain text
    Square,  // [text]
    Rounded, // (text)
    Circle,  // ((text))
    Bang,    // ))text((
    Cloud,   // )text(
    Hexagon, // {{text}}
}

#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
    pub depth: usize,
    pub children: Vec<MindmapNode>,
    pub classes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Mindmap {
    pub root: Option<MindmapNode>,
}

impl Mindmap {
    pub fn new() -> Self {
        Self { root: None }
    }
}

impl Default for Mindmap {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a mindmap diagram
pub fn parse(input: &str) -> Result<Mindmap, String> {
    let mut mindmap = Mindmap::new();
    let mut node_stack: Vec<(usize, MindmapNode)> = Vec::new();
    let mut node_counter = 0;

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }

        // Skip the mindmap keyword
        if trimmed == "mindmap" {
            continue;
        }

        // Calculate indentation (depth)
        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2; // Assume 2-space or 4-space indentation

        // Parse the node
        let (label, shape, classes) = parse_node_content(trimmed);

        let node = MindmapNode {
            id: format!("node_{}", node_counter),
            label,
            shape,
            depth,
            children: Vec::new(),
            classes,
        };
        node_counter += 1;

        // Find the correct parent by popping nodes with depth >= current depth
        while let Some((stack_depth, _)) = node_stack.last() {
            if *stack_depth >= depth {
                let (_, completed_node) = node_stack.pop().unwrap();
                if let Some((_, parent)) = node_stack.last_mut() {
                    parent.children.push(completed_node);
                } else {
                    mindmap.root = Some(completed_node);
                }
            } else {
                break;
            }
        }

        node_stack.push((depth, node));
    }

    // Pop remaining nodes from stack
    while let Some((_, completed_node)) = node_stack.pop() {
        if let Some((_, parent)) = node_stack.last_mut() {
            parent.children.push(completed_node);
        } else {
            mindmap.root = Some(completed_node);
        }
    }

    Ok(mindmap)
}

/// Parse node content to extract label, shape, and classes
fn parse_node_content(content: &str) -> (String, NodeShape, Vec<String>) {
    let mut text = content.to_string();
    let mut classes = Vec::new();

    // Extract classes (:::class1 class2)
    if let Some(class_idx) = text.find(":::") {
        let class_part = text[class_idx + 3..].trim();
        classes = class_part
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        text = text[..class_idx].trim().to_string();
    }

    // Determine shape and extract label
    let (label, shape) = if text.starts_with("((") && text.ends_with("))") {
        // Circle shape
        let label = text[2..text.len() - 2].to_string();
        (label, NodeShape::Circle)
    } else if text.starts_with("))") && text.ends_with("((") {
        // Bang shape
        let label = text[2..text.len() - 2].to_string();
        (label, NodeShape::Bang)
    } else if text.starts_with("{{") && text.ends_with("}}") {
        // Hexagon shape
        let label = text[2..text.len() - 2].to_string();
        (label, NodeShape::Hexagon)
    } else if text.starts_with(")") && text.ends_with("(") && text.len() > 2 {
        // Cloud shape
        let label = text[1..text.len() - 1].to_string();
        (label, NodeShape::Cloud)
    } else if text.starts_with("[") && text.ends_with("]") {
        // Square shape
        let label = text[1..text.len() - 1].to_string();
        (label, NodeShape::Square)
    } else if text.starts_with("(") && text.ends_with(")") && !text.starts_with("((") {
        // Rounded shape
        let label = text[1..text.len() - 1].to_string();
        (label, NodeShape::Rounded)
    } else {
        // Default shape (or handle id(label) format)
        // Check for id[label], id(label), id((label)), etc.
        if let Some(bracket_idx) = text.find('[') {
            if text.ends_with(']') {
                let label = text[bracket_idx + 1..text.len() - 1].to_string();
                return (label, NodeShape::Square, classes);
            }
        }
        if let Some(paren_idx) = text.find("((") {
            if text.ends_with("))") {
                let label = text[paren_idx + 2..text.len() - 2].to_string();
                return (label, NodeShape::Circle, classes);
            }
        }
        if let Some(paren_idx) = text.find("{{") {
            if text.ends_with("}}") {
                let label = text[paren_idx + 2..text.len() - 2].to_string();
                return (label, NodeShape::Hexagon, classes);
            }
        }
        if let Some(paren_idx) = text.find("))") {
            if text.ends_with("((") {
                let label = text[paren_idx + 2..text.len() - 2].to_string();
                return (label, NodeShape::Bang, classes);
            }
        }
        if let Some(paren_idx) = text.find(')') {
            if paren_idx > 0 && text.ends_with('(') {
                let label = text[paren_idx + 1..text.len() - 1].to_string();
                return (label, NodeShape::Cloud, classes);
            }
        }
        if let Some(paren_idx) = text.find('(') {
            if text.ends_with(')') && !text.ends_with("))") {
                let label = text[paren_idx + 1..text.len() - 1].to_string();
                return (label, NodeShape::Rounded, classes);
            }
        }

        (text, NodeShape::Default)
    };

    (label, shape, classes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let input = r#"mindmap
Root
    A
      B
      C"#;
        let result = parse(input).unwrap();
        assert!(result.root.is_some());
        let root = result.root.unwrap();
        assert_eq!(root.label, "Root");
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn test_parse_shapes() {
        let input = r#"mindmap
    root((Central))
        Square[I am a square]
        Rounded(I am rounded)
        Circle((I am a circle))"#;
        let result = parse(input).unwrap();
        let root = result.root.unwrap();
        assert_eq!(root.label, "Central");
        assert_eq!(root.shape, NodeShape::Circle);
    }
}
