//! Integration tests using test data fixtures
//!
//! Each test file in testdata/ascii/ and testdata/unicode/ gets its own test function.
//! Run all tests with: cargo test

use std::fs;
use std::path::PathBuf;
use std::collections::BTreeMap;

/// Get the path to the ASCII test data directory
fn get_ascii_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/ascii")
}

/// Get the path to the Unicode test data directory
fn get_unicode_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/unicode")
}

/// Parse a test file into (input, expected_output)
fn parse_test_file(content: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = content.splitn(2, "\n---\n").collect();
    if parts.len() != 2 {
        return None;
    }
    Some((parts[0].to_string(), parts[1].trim_end().to_string()))
}

/// Normalize output for comparison (trim trailing whitespace from each line)
fn normalize_output(s: &str) -> String {
    s.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

/// Run a test from the ASCII directory with subdirectory
fn run_ascii_test(subdir: &str, test_name: &str) {
    let test_file = get_ascii_dir().join(subdir).join(format!("{}.txt", test_name));
    run_test_file(&test_file, test_name, true);
}

/// Run a test from the Unicode directory with subdirectory
fn run_unicode_test(subdir: &str, test_name: &str) {
    let test_file = get_unicode_dir().join(subdir).join(format!("{}.txt", test_name));
    run_test_file(&test_file, test_name, false);
}

/// Run a specific test file
fn run_test_file(test_file: &PathBuf, test_name: &str, use_ascii: bool) {
    let content = fs::read_to_string(test_file)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", test_file, e));
    
    let (input, expected) = parse_test_file(&content)
        .unwrap_or_else(|| panic!("Failed to parse test file: {:?}", test_file));
    
    let options = m2svg::AsciiRenderOptions {
        use_ascii,
        ..Default::default()
    };
    
    let actual = m2svg::render_mermaid_ascii(&input, Some(options))
        .unwrap_or_else(|e| panic!("Failed to render: {}", e));
    
    let expected_normalized = normalize_output(&expected);
    let actual_normalized = normalize_output(&actual);
    
    if expected_normalized != actual_normalized {
        eprintln!("=== Test: {} ===", test_name);
        eprintln!("Input:\n{}", input);
        eprintln!("\n--- Expected ---");
        eprintln!("{}", expected_normalized);
        eprintln!("\n--- Actual ---");
        eprintln!("{}", actual_normalized);
        eprintln!("\n--- Diff ---");
        
        let expected_lines: Vec<_> = expected_normalized.lines().collect();
        let actual_lines: Vec<_> = actual_normalized.lines().collect();
        let max_lines = expected_lines.len().max(actual_lines.len());
        
        for i in 0..max_lines {
            let exp = expected_lines.get(i).unwrap_or(&"<missing>");
            let act = actual_lines.get(i).unwrap_or(&"<missing>");
            if exp != act {
                eprintln!("Line {}: expected {:?}", i + 1, exp);
                eprintln!("Line {}: actual   {:?}", i + 1, act);
            }
        }
        
        panic!("Output mismatch for test: {}", test_name);
    }
}

/// Macro to generate ASCII test functions
macro_rules! ascii_test {
    ($subdir:ident, $name:ident) => {
        paste::paste! {
            #[test]
            fn [<ascii_ $name>]() {
                run_ascii_test(stringify!($subdir), stringify!($name));
            }
        }
    };
}

/// Macro to generate Unicode test functions
macro_rules! unicode_test {
    ($subdir:ident, $name:ident) => {
        paste::paste! {
            #[test]
            fn [<unicode_ $name>]() {
                run_unicode_test(stringify!($subdir), stringify!($name));
            }
        }
    };
}

// =============================================================================
// ASCII Flowchart tests
// =============================================================================

ascii_test!(flowchart, ampersand_lhs);
ascii_test!(flowchart, ampersand_lhs_and_rhs);
ascii_test!(flowchart, ampersand_rhs);
ascii_test!(flowchart, ampersand_without_edge);
ascii_test!(flowchart, back_reference_from_child);
ascii_test!(flowchart, backlink_from_bottom);
ascii_test!(flowchart, backlink_from_top);
ascii_test!(flowchart, backlink_with_short_y_padding);
ascii_test!(flowchart, comments);
ascii_test!(flowchart, custom_padding);
ascii_test!(flowchart, duplicate_labels);
ascii_test!(flowchart, flowchart_tb_simple);
ascii_test!(flowchart, graph_bt_direction);
ascii_test!(flowchart, graph_tb_direction);
ascii_test!(flowchart, nested_subgraphs_with_labels);
ascii_test!(flowchart, preserve_order_of_definition);
ascii_test!(flowchart, self_reference);
ascii_test!(flowchart, self_reference_with_edge);
ascii_test!(flowchart, single_node);
ascii_test!(flowchart, single_node_longer_name);
ascii_test!(flowchart, subgraph_complex_mixed);
ascii_test!(flowchart, subgraph_complex_nested);
ascii_test!(flowchart, subgraph_empty);
ascii_test!(flowchart, subgraph_mixed_nodes);
ascii_test!(flowchart, subgraph_mixed_nodes_td);
ascii_test!(flowchart, subgraph_multiple_edges);
ascii_test!(flowchart, subgraph_multiple_nodes);
ascii_test!(flowchart, subgraph_nested);
ascii_test!(flowchart, subgraph_nested_with_external);
ascii_test!(flowchart, subgraph_node_outside_lr);
ascii_test!(flowchart, subgraph_single_node);
ascii_test!(flowchart, subgraph_td_direction);
ascii_test!(flowchart, subgraph_td_multiple);
ascii_test!(flowchart, subgraph_td_multiple_paddingy);
ascii_test!(flowchart, subgraph_three_levels_nested);
ascii_test!(flowchart, subgraph_three_separate);
ascii_test!(flowchart, subgraph_two_separate);
ascii_test!(flowchart, subgraph_with_labels);
ascii_test!(flowchart, three_nodes);
ascii_test!(flowchart, three_nodes_single_line);
ascii_test!(flowchart, two_layer_single_graph);
ascii_test!(flowchart, two_layer_single_graph_longer_names);
ascii_test!(flowchart, two_nodes_linked);
ascii_test!(flowchart, two_nodes_longer_names);
ascii_test!(flowchart, two_root_nodes);
ascii_test!(flowchart, two_root_nodes_longer_names);
ascii_test!(flowchart, two_single_root_nodes);

// =============================================================================
// ASCII Class diagram tests
// =============================================================================

ascii_test!(class, cls_all_relationships);
ascii_test!(class, cls_annotation);
ascii_test!(class, cls_association);
ascii_test!(class, cls_basic);
ascii_test!(class, cls_dependency);
ascii_test!(class, cls_inheritance);
ascii_test!(class, cls_inheritance_fanout);
ascii_test!(class, cls_methods);

// =============================================================================
// ASCII ER diagram tests
// =============================================================================

ascii_test!(er, er_attributes);
ascii_test!(er, er_basic);
ascii_test!(er, er_identifying);

// =============================================================================
// ASCII Sequence diagram tests
// =============================================================================

ascii_test!(sequence, seq_basic);
ascii_test!(sequence, seq_multiple_messages);
ascii_test!(sequence, seq_self_message);

// =============================================================================
// ASCII GitGraph tests
// =============================================================================

ascii_test!(gitgraph, gitgraph_basic);
ascii_test!(gitgraph, gitgraph_bottom_to_top);
ascii_test!(gitgraph, gitgraph_branch_order);
ascii_test!(gitgraph, gitgraph_branches);
ascii_test!(gitgraph, gitgraph_cherry_pick);
ascii_test!(gitgraph, gitgraph_commit_types);
ascii_test!(gitgraph, gitgraph_custom_ids);
ascii_test!(gitgraph, gitgraph_merge_complex);
ascii_test!(gitgraph, gitgraph_tags);
ascii_test!(gitgraph, gitgraph_vertical);

// =============================================================================
// Unicode Flowchart tests
// =============================================================================

unicode_test!(flowchart, ampersand_lhs);
unicode_test!(flowchart, ampersand_lhs_and_rhs);
unicode_test!(flowchart, ampersand_rhs);
unicode_test!(flowchart, ampersand_without_edge);
unicode_test!(flowchart, back_reference_from_child);
unicode_test!(flowchart, backlink_from_bottom);
unicode_test!(flowchart, backlink_from_top);
unicode_test!(flowchart, comments);
unicode_test!(flowchart, duplicate_labels);
unicode_test!(flowchart, graph_bt_direction);
unicode_test!(flowchart, preserve_order_of_definition);
unicode_test!(flowchart, self_reference);
unicode_test!(flowchart, self_reference_with_edge);
unicode_test!(flowchart, single_node);
unicode_test!(flowchart, single_node_longer_name);
unicode_test!(flowchart, three_nodes);
unicode_test!(flowchart, three_nodes_single_line);
unicode_test!(flowchart, two_layer_single_graph);
unicode_test!(flowchart, two_layer_single_graph_longer_names);
unicode_test!(flowchart, two_nodes_linked);
unicode_test!(flowchart, two_nodes_longer_names);
unicode_test!(flowchart, two_root_nodes);
unicode_test!(flowchart, two_root_nodes_longer_names);
unicode_test!(flowchart, two_single_root_nodes);

// =============================================================================
// Unicode Class diagram tests
// =============================================================================

unicode_test!(class, cls_all_relationships);
unicode_test!(class, cls_annotation);
unicode_test!(class, cls_association);
unicode_test!(class, cls_basic);
unicode_test!(class, cls_dependency);
unicode_test!(class, cls_inheritance);
unicode_test!(class, cls_inheritance_fanout);
unicode_test!(class, cls_methods);

// =============================================================================
// Unicode ER diagram tests
// =============================================================================

unicode_test!(er, er_attributes);
unicode_test!(er, er_basic);
unicode_test!(er, er_identifying);

// =============================================================================
// Unicode Sequence diagram tests
// =============================================================================

unicode_test!(sequence, seq_basic);
unicode_test!(sequence, seq_multiple_messages);
unicode_test!(sequence, seq_self_message);

// =============================================================================
// SVG tests
// =============================================================================

/// Get the path to the SVG test data directory
fn get_svg_dir() -> std::path::PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/svg")
}

/// Represents a normalized SVG element for comparison
#[derive(Debug, PartialEq)]
struct SvgElement {
    tag: String,
    attributes: BTreeMap<String, String>,
    text: String,
    children: Vec<SvgElement>,
}

/// Normalize an attribute value (trim whitespace, normalize numbers)
fn normalize_attr_value(value: &str) -> String {
    // Normalize whitespace
    let normalized: String = value.split_whitespace().collect::<Vec<_>>().join(" ");
    
    // Try to normalize numeric values (for things like coordinates)
    // This handles cases like "100.0" vs "100" or "1.5000" vs "1.5"
    if let Ok(num) = normalized.parse::<f64>() {
        // Format with reasonable precision, removing trailing zeros
        let formatted = format!("{:.6}", num);
        formatted.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        normalized
    }
}

/// Recursively build a normalized SVG element tree from roxmltree
fn build_svg_tree(node: roxmltree::Node) -> Option<SvgElement> {
    if node.is_element() {
        let tag = node.tag_name().name().to_string();
        
        // Collect and sort attributes
        let mut attributes = BTreeMap::new();
        for attr in node.attributes() {
            let name = attr.name().to_string();
            let value = normalize_attr_value(attr.value());
            attributes.insert(name, value);
        }
        
        // Get text content (normalized)
        let text: String = node
            .children()
            .filter(|n| n.is_text())
            .map(|n| n.text().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        
        // Recursively process children
        let children: Vec<SvgElement> = node
            .children()
            .filter_map(|child| build_svg_tree(child))
            .collect();
        
        Some(SvgElement {
            tag,
            attributes,
            text,
            children,
        })
    } else {
        None
    }
}

/// Compare two SVG strings semantically
/// Returns Ok(()) if they match, or Err with a description of the difference
fn compare_svg_semantic(expected: &str, actual: &str) -> Result<(), String> {
    let expected_doc = roxmltree::Document::parse(expected)
        .map_err(|e| format!("Failed to parse expected SVG: {}", e))?;
    let actual_doc = roxmltree::Document::parse(actual)
        .map_err(|e| format!("Failed to parse actual SVG: {}", e))?;
    
    let expected_tree = build_svg_tree(expected_doc.root_element())
        .ok_or_else(|| "Expected SVG has no root element".to_string())?;
    let actual_tree = build_svg_tree(actual_doc.root_element())
        .ok_or_else(|| "Actual SVG has no root element".to_string())?;
    
    compare_svg_elements(&expected_tree, &actual_tree, "")
}

/// Recursively compare two SVG element trees
fn compare_svg_elements(expected: &SvgElement, actual: &SvgElement, path: &str) -> Result<(), String> {
    let current_path = if path.is_empty() {
        expected.tag.clone()
    } else {
        format!("{}/{}", path, expected.tag)
    };
    
    // Compare tag names
    if expected.tag != actual.tag {
        return Err(format!(
            "Tag mismatch at {}: expected '{}', got '{}'",
            path, expected.tag, actual.tag
        ));
    }
    
    // Compare attributes
    for (key, exp_val) in &expected.attributes {
        match actual.attributes.get(key) {
            Some(act_val) if exp_val != act_val => {
                return Err(format!(
                    "Attribute '{}' mismatch at {}: expected '{}', got '{}'",
                    key, current_path, exp_val, act_val
                ));
            }
            None => {
                return Err(format!(
                    "Missing attribute '{}' at {} (expected value: '{}')",
                    key, current_path, exp_val
                ));
            }
            _ => {}
        }
    }
    
    // Check for extra attributes in actual
    for key in actual.attributes.keys() {
        if !expected.attributes.contains_key(key) {
            return Err(format!(
                "Unexpected attribute '{}' at {} (value: '{}')",
                key, current_path, actual.attributes.get(key).unwrap()
            ));
        }
    }
    
    // Compare text content
    if expected.text != actual.text {
        return Err(format!(
            "Text mismatch at {}: expected '{}', got '{}'",
            current_path, expected.text, actual.text
        ));
    }
    
    // Compare children count
    if expected.children.len() != actual.children.len() {
        return Err(format!(
            "Children count mismatch at {}: expected {}, got {}",
            current_path, expected.children.len(), actual.children.len()
        ));
    }
    
    // Recursively compare children
    for (i, (exp_child, act_child)) in expected.children.iter().zip(actual.children.iter()).enumerate() {
        let child_path = format!("{}[{}]", current_path, i);
        compare_svg_elements(exp_child, act_child, &child_path)?;
    }
    
    Ok(())
}

/// Run an SVG test from the svg directory (semantic comparison)
fn run_svg_test(subdir: &str, test_name: &str) {
    let mmd_file = get_svg_dir().join(subdir).join(format!("{}.mmd", test_name));
    let svg_file = get_svg_dir().join(subdir).join(format!("{}.svg", test_name));
    
    let input = fs::read_to_string(&mmd_file)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", mmd_file, e));
    let expected = fs::read_to_string(&svg_file)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", svg_file, e));
    
    // Filter out comment lines from input
    let input: String = input
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    
    let actual = m2svg::render_to_svg(&input)
        .unwrap_or_else(|e| panic!("Failed to render SVG: {}", e));
    
    // Use semantic comparison instead of literal string comparison
    if let Err(diff) = compare_svg_semantic(&expected, &actual) {
        eprintln!("=== Test: {} ===", test_name);
        eprintln!("Input:\n{}", input);
        eprintln!("\n--- Difference ---");
        eprintln!("{}", diff);
        eprintln!("\n--- Expected SVG ---");
        eprintln!("{}", expected);
        eprintln!("\n--- Actual SVG ---");
        eprintln!("{}", actual);
        panic!("SVG output mismatch for test: {}", test_name);
    }
}

/// Macro to generate SVG test functions
macro_rules! svg_test {
    ($subdir:ident, $name:ident) => {
        paste::paste! {
            #[test]
            fn [<svg_ $name>]() {
                run_svg_test(stringify!($subdir), stringify!($name));
            }
        }
    };
}

// SVG tests from mermaid.js.org examples
svg_test!(class, class_annotation);
svg_test!(class, class_bankaccount);
svg_test!(class, class_basic);
svg_test!(class, class_cardinality);
svg_test!(class, class_generics);
svg_test!(class, class_inheritance);
svg_test!(class, class_namespace);
svg_test!(class, class_relationships);
svg_test!(er, er_attributes);
svg_test!(er, er_basic);
svg_test!(er, er_order_system);
svg_test!(er, er_zero_or_one);
svg_test!(flowchart, flowchart_arrow_link);
svg_test!(flowchart, flowchart_basic_node);
svg_test!(flowchart, flowchart_chaining);
svg_test!(flowchart, flowchart_circle);
svg_test!(flowchart, flowchart_comprehensive);
svg_test!(flowchart, flowchart_cylinder);
svg_test!(flowchart, flowchart_decision_tree);
svg_test!(flowchart, flowchart_diamond);
svg_test!(flowchart, flowchart_dotted_link);
svg_test!(flowchart, flowchart_double_circle);
svg_test!(flowchart, flowchart_flag);
svg_test!(flowchart, flowchart_hexagon);
svg_test!(flowchart, flowchart_link_with_text);
svg_test!(flowchart, flowchart_loop_back);
svg_test!(flowchart, flowchart_lr_direction);
svg_test!(flowchart, flowchart_node_with_text);
svg_test!(flowchart, flowchart_parallel_links);
svg_test!(flowchart, flowchart_round_edges);
svg_test!(flowchart, flowchart_stadium);
svg_test!(flowchart, flowchart_styling);
svg_test!(flowchart, flowchart_subgraphs);
svg_test!(flowchart, flowchart_subroutine);
svg_test!(flowchart, flowchart_td_direction);
svg_test!(flowchart, flowchart_thick_link);
svg_test!(flowchart, flowchart_trapezoid);
svg_test!(sequence, sequence_activation);
svg_test!(sequence, sequence_actors);
svg_test!(sequence, sequence_aliases);
svg_test!(sequence, sequence_alt);
svg_test!(sequence, sequence_basic);
svg_test!(sequence, sequence_break);
svg_test!(sequence, sequence_critical);
svg_test!(sequence, sequence_loop);
svg_test!(sequence, sequence_notes);
svg_test!(sequence, sequence_parallel);
svg_test!(sequence, sequence_participants);
svg_test!(sequence, sequence_rect);
svg_test!(sequence, sequence_stacked_activation);

// Legacy SVG tests (kept for backwards compatibility)
