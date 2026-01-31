//! Integration tests using test data fixtures
//!
//! Each test file in testdata/ascii/ and testdata/unicode/ gets its own test function.
//! Run all tests with: cargo test

use std::fs;
use std::path::PathBuf;

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

/// Run a test from the ASCII directory
fn run_ascii_test(test_name: &str) {
    let test_file = get_ascii_dir().join(format!("{}.txt", test_name));
    run_test_file(&test_file, test_name, true);
}

/// Run a test from the Unicode directory
fn run_unicode_test(test_name: &str) {
    let test_file = get_unicode_dir().join(format!("{}.txt", test_name));
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
    ($name:ident) => {
        #[test]
        fn $name() {
            run_ascii_test(stringify!($name));
        }
    };
}

/// Macro to generate Unicode test functions  
macro_rules! unicode_test {
    ($name:ident) => {
        paste::paste! {
            #[test]
            fn [<unicode_ $name>]() {
                run_unicode_test(stringify!($name));
            }
        }
    };
}

// =============================================================================
// ASCII tests (61 files)
// =============================================================================

ascii_test!(ampersand_lhs);
ascii_test!(ampersand_lhs_and_rhs);
ascii_test!(ampersand_rhs);
ascii_test!(ampersand_without_edge);
ascii_test!(back_reference_from_child);
ascii_test!(backlink_from_bottom);
ascii_test!(backlink_from_top);
ascii_test!(backlink_with_short_y_padding);
ascii_test!(cls_all_relationships);
ascii_test!(cls_annotation);
ascii_test!(cls_association);
ascii_test!(cls_basic);
ascii_test!(cls_dependency);
ascii_test!(cls_inheritance);
ascii_test!(cls_inheritance_fanout);
ascii_test!(cls_methods);
ascii_test!(comments);
ascii_test!(custom_padding);
ascii_test!(duplicate_labels);
ascii_test!(er_attributes);
ascii_test!(er_basic);
ascii_test!(er_identifying);
ascii_test!(flowchart_tb_simple);
ascii_test!(graph_bt_direction);
ascii_test!(graph_tb_direction);
ascii_test!(nested_subgraphs_with_labels);
ascii_test!(preserve_order_of_definition);
ascii_test!(self_reference);
ascii_test!(self_reference_with_edge);
ascii_test!(seq_basic);
ascii_test!(seq_multiple_messages);
ascii_test!(seq_self_message);
ascii_test!(single_node);
ascii_test!(single_node_longer_name);
ascii_test!(subgraph_complex_mixed);
ascii_test!(subgraph_complex_nested);
ascii_test!(subgraph_empty);
ascii_test!(subgraph_mixed_nodes);
ascii_test!(subgraph_mixed_nodes_td);
ascii_test!(subgraph_multiple_edges);
ascii_test!(subgraph_multiple_nodes);
ascii_test!(subgraph_nested);
ascii_test!(subgraph_nested_with_external);
ascii_test!(subgraph_node_outside_lr);
ascii_test!(subgraph_single_node);
ascii_test!(subgraph_td_direction);
ascii_test!(subgraph_td_multiple);
ascii_test!(subgraph_td_multiple_paddingy);
ascii_test!(subgraph_three_levels_nested);
ascii_test!(subgraph_three_separate);
ascii_test!(subgraph_two_separate);
ascii_test!(subgraph_with_labels);
ascii_test!(three_nodes);
ascii_test!(three_nodes_single_line);
ascii_test!(two_layer_single_graph);
ascii_test!(two_layer_single_graph_longer_names);
ascii_test!(two_nodes_linked);
ascii_test!(two_nodes_longer_names);
ascii_test!(two_root_nodes);
ascii_test!(two_root_nodes_longer_names);
ascii_test!(two_single_root_nodes);

// =============================================================================
// Unicode tests (38 files)
// =============================================================================

unicode_test!(ampersand_lhs);
unicode_test!(ampersand_lhs_and_rhs);
unicode_test!(ampersand_rhs);
unicode_test!(ampersand_without_edge);
unicode_test!(back_reference_from_child);
unicode_test!(backlink_from_bottom);
unicode_test!(backlink_from_top);
unicode_test!(cls_all_relationships);
unicode_test!(cls_annotation);
unicode_test!(cls_association);
unicode_test!(cls_basic);
unicode_test!(cls_dependency);
unicode_test!(cls_inheritance);
unicode_test!(cls_inheritance_fanout);
unicode_test!(cls_methods);
unicode_test!(comments);
unicode_test!(duplicate_labels);
unicode_test!(er_attributes);
unicode_test!(er_basic);
unicode_test!(er_identifying);
unicode_test!(graph_bt_direction);
unicode_test!(preserve_order_of_definition);
unicode_test!(self_reference);
unicode_test!(self_reference_with_edge);
unicode_test!(seq_basic);
unicode_test!(seq_multiple_messages);
unicode_test!(seq_self_message);
unicode_test!(single_node);
unicode_test!(single_node_longer_name);
unicode_test!(three_nodes);
unicode_test!(three_nodes_single_line);
unicode_test!(two_layer_single_graph);
unicode_test!(two_layer_single_graph_longer_names);
unicode_test!(two_nodes_linked);
unicode_test!(two_nodes_longer_names);
unicode_test!(two_root_nodes);
unicode_test!(two_root_nodes_longer_names);
unicode_test!(two_single_root_nodes);

// =============================================================================
// SVG tests
// =============================================================================

#[test]
#[ignore] // Run with: cargo test test_svg -- --ignored
fn test_svg_flowchart() {
    let input = r#"graph TD
        A[Start] --> B[End]"#;
    
    let result = m2svg::render_to_svg(input);
    assert!(result.is_ok(), "SVG render failed: {:?}", result.err());
    
    let svg = result.unwrap();
    assert!(svg.contains("<svg"), "Output should contain SVG tag");
    assert!(svg.contains("Start"), "Output should contain node text");
}

#[test]
#[ignore] // Run with: cargo test test_svg -- --ignored
fn test_svg_sequence() {
    let input = r#"sequenceDiagram
        Alice->>Bob: Hello"#;
    
    let result = m2svg::render_to_svg(input);
    assert!(result.is_ok(), "SVG render failed: {:?}", result.err());
    
    let svg = result.unwrap();
    assert!(svg.contains("<svg"), "Output should contain SVG tag");
}
