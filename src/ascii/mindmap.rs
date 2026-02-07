/// ASCII/Unicode renderer for mindmap diagrams - Horizontal radial layout
use crate::parser::mindmap::{Mindmap, MindmapNode, NodeShape};

/// Characters for drawing
struct DrawChars {
    h_line: char,      // horizontal line: '-' or '─'
    v_line: char,      // vertical line: '|' or '│'
    branch: char,      // branch point: '+' or '┬'
    corner_last: char, // last child corner: '+' or '└'
    corner_mid: char,  // middle child corner: '+' or '├'
}

impl DrawChars {
    fn ascii() -> Self {
        Self {
            h_line: '-',
            v_line: '|',
            branch: '+',
            corner_last: '+',
            corner_mid: '+',
        }
    }

    fn unicode() -> Self {
        Self {
            h_line: '─',
            v_line: '│',
            branch: '┬',
            corner_last: '└',
            corner_mid: '├',
        }
    }
}

/// Render a mindmap to ASCII/Unicode art with horizontal radial layout
pub fn render(mindmap: &Mindmap, use_ascii: bool) -> String {
    let Some(root) = &mindmap.root else {
        return String::new();
    };

    let chars = if use_ascii {
        DrawChars::ascii()
    } else {
        DrawChars::unicode()
    };

    // Build the mindmap as a list of lines
    let mut result_lines: Vec<String> = Vec::new();

    // Render the tree structure
    render_horizontal(root, &mut result_lines, &chars, use_ascii);

    result_lines.join("\n")
}

/// Render the mindmap horizontally with root on left
fn render_horizontal(
    root: &MindmapNode,
    lines: &mut Vec<String>,
    chars: &DrawChars,
    use_ascii: bool,
) {
    // Get the rendered subtree for each child
    let child_blocks: Vec<Vec<String>> = root
        .children
        .iter()
        .map(|child| render_subtree(child, chars, use_ascii))
        .collect();

    let root_text = format_node(root, use_ascii);

    if child_blocks.is_empty() {
        // No children - just the root
        lines.push(root_text);
        return;
    }

    // Build the output - root on first line
    let root_width = root_text.chars().count();

    for (child_idx, block) in child_blocks.iter().enumerate() {
        let is_last_child = child_idx == child_blocks.len() - 1;
        let is_first_child = child_idx == 0;

        for (line_idx, line) in block.iter().enumerate() {
            let mut output_line = String::new();

            let is_first_line_of_block = line_idx == 0;

            // Root text only on first line of first block
            if child_idx == 0 && line_idx == 0 {
                output_line.push_str(&root_text);
                output_line.push(' ');
            } else {
                // Pad with spaces for root width + 1 (to align after root text)
                for _ in 0..=root_width {
                    output_line.push(' ');
                }
            }

            // Determine connector for this row (all 3 chars, key char at pos 1)
            if is_first_line_of_block {
                // This line connects to a child
                if root.children.len() == 1 {
                    // Single child: ---
                    output_line.push(chars.h_line);
                    output_line.push(chars.h_line);
                    output_line.push(chars.h_line);
                } else if is_first_child {
                    // First of multiple: -+-
                    output_line.push(chars.h_line);
                    output_line.push(chars.branch);
                    output_line.push(chars.h_line);
                } else if is_last_child {
                    // Last child:  └-
                    output_line.push(' ');
                    output_line.push(chars.corner_last);
                    output_line.push(chars.h_line);
                } else {
                    // Middle child:  ├-
                    output_line.push(' ');
                    output_line.push(chars.corner_mid);
                    output_line.push(chars.h_line);
                }
            } else {
                // Continuation line - vertical bar if more children
                if !is_last_child {
                    output_line.push(' '); // align │ under ┬
                    output_line.push(chars.v_line);
                    output_line.push(' ');
                } else {
                    output_line.push_str("   ");
                }
            }

            output_line.push(' ');
            output_line.push_str(line);
            lines.push(output_line.trim_end().to_string());
        }
    }
}

/// Render a subtree (child and its descendants) as a block of lines
fn render_subtree(node: &MindmapNode, chars: &DrawChars, use_ascii: bool) -> Vec<String> {
    let node_text = format_node(node, use_ascii);

    if node.children.is_empty() {
        return vec![node_text];
    }

    // Get blocks for all grandchildren
    let child_blocks: Vec<Vec<String>> = node
        .children
        .iter()
        .map(|child| render_subtree(child, chars, use_ascii))
        .collect();

    let total_height: usize = child_blocks.iter().map(|b| b.len()).sum();
    let mut result = Vec::with_capacity(total_height);

    let node_width = node_text.chars().count();

    for (child_idx, block) in child_blocks.iter().enumerate() {
        let is_last_child = child_idx == child_blocks.len() - 1;
        let is_first_child = child_idx == 0;

        for (line_idx, line) in block.iter().enumerate() {
            let mut output_line = String::new();

            let is_first_line_of_block = line_idx == 0;

            // First line of first block gets the node name
            if child_idx == 0 && line_idx == 0 {
                output_line.push_str(&node_text);
                output_line.push(' ');
            } else {
                // Pad with spaces (node_width + 1)
                for _ in 0..=node_width {
                    output_line.push(' ');
                }
            }

            // Add connector (all 3 chars, key char at pos 1)
            if is_first_line_of_block {
                if node.children.len() == 1 {
                    // Single child: ---
                    output_line.push(chars.h_line);
                    output_line.push(chars.h_line);
                    output_line.push(chars.h_line);
                } else if is_first_child {
                    // First of multiple: -+-
                    output_line.push(chars.h_line);
                    output_line.push(chars.branch);
                    output_line.push(chars.h_line);
                } else if is_last_child {
                    // Last child:  └-
                    output_line.push(' ');
                    output_line.push(chars.corner_last);
                    output_line.push(chars.h_line);
                } else {
                    // Middle child:  ├-
                    output_line.push(' ');
                    output_line.push(chars.corner_mid);
                    output_line.push(chars.h_line);
                }
            } else {
                // Continuation - vertical bar if more children
                if !is_last_child {
                    output_line.push(' '); // align │ under ┬
                    output_line.push(chars.v_line);
                    output_line.push(' ');
                } else {
                    output_line.push_str("   ");
                }
            }

            output_line.push(' ');
            output_line.push_str(line);
            result.push(output_line);
        }
    }

    result
}

fn format_node(node: &MindmapNode, use_ascii: bool) -> String {
    let label = &node.label;

    match node.shape {
        NodeShape::Square => format!("[{}]", label),
        NodeShape::Rounded => format!("({})", label),
        NodeShape::Circle => format!("(({}))", label),
        NodeShape::Bang => format!(")){}((", label),
        NodeShape::Cloud => format!("){}(", label),
        NodeShape::Hexagon => {
            format!("{{{{{}}}}}", label)
        }
        NodeShape::Default => label.clone(),
    }
}
