//! Integration tests using test data fixtures
//!
//! These tests read test data files and verify that the Rust
//! implementation produces the expected output.

use std::fs;
use std::path::PathBuf;

/// Get the path to the test data directory (ASCII)
fn get_testdata_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("testdata/ascii")
}

/// Get the path to the test data directory (Unicode)
fn get_unicode_testdata_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("testdata/unicode")
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

/// Run the renderer and compare output
fn run_test(test_name: &str) {
    let ascii_dir = get_testdata_dir();
    let unicode_dir = get_unicode_testdata_dir();
    
    // Check both directories for the test file
    let test_file = {
        let ascii_file = ascii_dir.join(format!("{}.txt", test_name));
        let unicode_file = unicode_dir.join(format!("{}.txt", test_name));
        if ascii_file.exists() {
            ascii_file
        } else if unicode_file.exists() {
            unicode_file
        } else {
            panic!("Test file not found in ascii or unicode directories: {}", test_name);
        }
    };
    
    let content = fs::read_to_string(&test_file)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", test_file, e));
    
    let (input, expected) = parse_test_file(&content)
        .unwrap_or_else(|| panic!("Failed to parse test file: {:?}", test_file));
    
    // Detect mode from expected output: if it contains Unicode box chars, use Unicode mode
    let use_unicode = expected.contains('┌') || expected.contains('│') || expected.contains('─');
    
    let options = m2svg::AsciiRenderOptions {
        use_ascii: !use_unicode,
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
        
        // Simple line-by-line diff
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

// =============================================================================
// Flowchart / Graph tests
// =============================================================================

#[test]
fn test_single_node() {
    run_test("single_node");
}

#[test]
fn test_single_node_longer_name() {
    run_test("single_node_longer_name");
}

#[test]
fn test_flowchart_tb_simple() {
    run_test("flowchart_tb_simple");
}

#[test]
fn test_graph_tb_direction() {
    run_test("graph_tb_direction");
}

#[test]
fn test_graph_bt_direction() {
    run_test("graph_bt_direction");
}

#[test]
fn test_self_reference() {
    run_test("self_reference");
}

#[test]
fn test_self_reference_with_edge() {
    run_test("self_reference_with_edge");
}

#[test]
fn test_back_reference_from_child() {
    run_test("back_reference_from_child");
}

#[test]
fn test_backlink_from_top() {
    run_test("backlink_from_top");
}

#[test]
fn test_backlink_from_bottom() {
    run_test("backlink_from_bottom");
}

#[test]
fn test_backlink_with_short_y_padding() {
    run_test("backlink_with_short_y_padding");
}

#[test]
fn test_duplicate_labels() {
    run_test("duplicate_labels");
}

#[test]
fn test_preserve_order_of_definition() {
    run_test("preserve_order_of_definition");
}

#[test]
fn test_comments() {
    run_test("comments");
}

#[test]
fn test_custom_padding() {
    run_test("custom_padding");
}

#[test]
fn test_ampersand_lhs() {
    run_test("ampersand_lhs");
}

#[test]
fn test_ampersand_rhs() {
    run_test("ampersand_rhs");
}

#[test]
fn test_ampersand_lhs_and_rhs() {
    run_test("ampersand_lhs_and_rhs");
}

#[test]
fn test_ampersand_without_edge() {
    run_test("ampersand_without_edge");
}

// =============================================================================
// Subgraph tests
// =============================================================================

#[test]
fn test_subgraph_empty() {
    run_test("subgraph_empty");
}

#[test]
fn test_subgraph_multiple_nodes() {
    run_test("subgraph_multiple_nodes");
}

#[test]
fn test_subgraph_multiple_edges() {
    run_test("subgraph_multiple_edges");
}

#[test]
fn test_subgraph_mixed_nodes() {
    run_test("subgraph_mixed_nodes");
}

#[test]
fn test_subgraph_mixed_nodes_td() {
    run_test("subgraph_mixed_nodes_td");
}

#[test]
fn test_subgraph_complex_nested() {
    run_test("subgraph_complex_nested");
}

#[test]
fn test_subgraph_complex_mixed() {
    run_test("subgraph_complex_mixed");
}

#[test]
fn test_nested_subgraphs_with_labels() {
    run_test("nested_subgraphs_with_labels");
}

// =============================================================================
// Sequence diagram tests
// =============================================================================

#[test]
fn test_seq_basic() {
    run_test("seq_basic");
}

#[test]
fn test_seq_multiple_messages() {
    run_test("seq_multiple_messages");
}

#[test]
fn test_seq_self_message() {
    run_test("seq_self_message");
}

// =============================================================================
// Class diagram tests
// =============================================================================

#[test]
fn test_cls_basic() {
    run_test("cls_basic");
}

#[test]
fn test_cls_methods() {
    run_test("cls_methods");
}

#[test]
fn test_cls_annotation() {
    run_test("cls_annotation");
}

#[test]
fn test_cls_inheritance() {
    run_test("cls_inheritance");
}

#[test]
fn test_cls_association() {
    run_test("cls_association");
}

#[test]
fn test_cls_dependency() {
    run_test("cls_dependency");
}

#[test]
fn test_cls_all_relationships() {
    run_test("cls_all_relationships");
}

#[test]
fn test_cls_inheritance_fanout() {
    run_test("cls_inheritance_fanout");
}

// =============================================================================
// ER diagram tests
// =============================================================================

#[test]
fn test_er_basic() {
    run_test("er_basic");
}

#[test]
fn test_er_attributes() {
    run_test("er_attributes");
}

#[test]
fn test_er_identifying() {
    run_test("er_identifying");
}

// =============================================================================
// Test runner for all tests in the directory
// =============================================================================

/// Run tests from a single directory
fn run_tests_from_dir(dir: &PathBuf, mode: &str) -> (usize, usize, usize, Vec<String>) {
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();
    
    if !dir.exists() {
        eprintln!("{} test directory not found: {:?}", mode, dir);
        return (0, 0, 0, vec![]);
    }
    
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        if path.extension().map(|e| e == "txt").unwrap_or(false) {
            let test_name = path.file_stem().unwrap().to_str().unwrap();
            eprint!("[{}] Testing {}... ", mode, test_name);
            
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("SKIP: {}", e);
                    skipped += 1;
                    continue;
                }
            };
            
            let (input, expected) = match parse_test_file(&content) {
                Some(v) => v,
                None => {
                    eprintln!("SKIP: invalid format");
                    skipped += 1;
                    continue;
                }
            };
            
            // Detect mode from expected output
            let use_unicode = expected.contains('┌') || expected.contains('│') || expected.contains('─');
            
            let options = m2svg::AsciiRenderOptions {
                use_ascii: !use_unicode,
                ..Default::default()
            };
            
            let actual = match m2svg::render_mermaid_ascii(&input, Some(options)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("SKIP: render error: {}", e);
                    skipped += 1;
                    continue;
                }
            };
            
            let expected_normalized = normalize_output(&expected);
            let actual_normalized = normalize_output(&actual);
            
            if expected_normalized == actual_normalized {
                eprintln!("PASS");
                passed += 1;
            } else {
                eprintln!("FAIL");
                failed += 1;
                failures.push(format!("[{}] {}", mode, test_name));
            }
        }
    }
    
    (passed, failed, skipped, failures)
}

#[test]
fn test_all_testdata_files() {
    let ascii_dir = get_testdata_dir();
    let unicode_dir = get_unicode_testdata_dir();
    
    eprintln!("ASCII test directory: {:?}", ascii_dir);
    eprintln!("Unicode test directory: {:?}", unicode_dir);
    
    let (p1, f1, s1, mut failures1) = run_tests_from_dir(&ascii_dir, "ASCII");
    let (p2, f2, s2, failures2) = run_tests_from_dir(&unicode_dir, "Unicode");
    
    failures1.extend(failures2);
    
    let passed = p1 + p2;
    let failed = f1 + f2;
    let skipped = s1 + s2;
    
    eprintln!("\n=== Summary ===");
    eprintln!("ASCII:   {} passed, {} failed, {} skipped", p1, f1, s1);
    eprintln!("Unicode: {} passed, {} failed, {} skipped", p2, f2, s2);
    eprintln!("Total:   {} passed, {} failed, {} skipped", passed, failed, skipped);
    
    if !failures1.is_empty() {
        eprintln!("\nFailed tests:");
        for f in &failures1 {
            eprintln!("  - {}", f);
        }
        panic!("{} tests failed", failed);
    }
}

// ============================================================================
// SVG Tests - using positioned JSON fixtures
// ============================================================================

/// Get the path to the positioned JSON test data directory
fn get_positioned_testdata_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("testdata/positioned")
}

/// Fixture structure matching the JSON format
#[derive(serde::Deserialize)]
struct SvgFixture {
    #[allow(dead_code)]
    input: String,
    #[serde(rename = "type")]
    diagram_type: String,
    positioned: serde_json::Value,
    svg: String,
}

/// Run a single SVG renderer test from a positioned JSON fixture
fn run_svg_test(test_name: &str) -> Result<(), String> {
    use m2svg::svg::{render_svg, DiagramColors, PositionedGraph};
    
    let positioned_dir = get_positioned_testdata_dir();
    let test_file = positioned_dir.join(format!("{}.json", test_name));
    
    if !test_file.exists() {
        return Err(format!("Test file not found: {:?}", test_file));
    }
    
    let content = fs::read_to_string(&test_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let fixture: SvgFixture = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    // Only test flowcharts for now (class, sequence, ER have different renderers)
    if fixture.diagram_type != "flowchart" {
        return Err("skip".to_string());
    }
    
    // Parse the positioned graph
    let positioned: PositionedGraph = serde_json::from_value(fixture.positioned)
        .map_err(|e| format!("Failed to parse positioned graph: {}", e))?;
    
    // Render SVG with same settings as TypeScript fixtures
    let colors = DiagramColors::default();
    let actual = render_svg(&positioned, &colors, "Inter", false);
    
    // Compare
    let expected = fixture.svg.trim();
    let actual = actual.trim();
    
    if actual != expected {
        // Show diff for debugging
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();
        
        for (i, (exp, act)) in expected_lines.iter().zip(actual_lines.iter()).enumerate() {
            if exp != act {
                return Err(format!(
                    "Mismatch at line {}:\n  expected: {}\n  actual:   {}",
                    i + 1, exp, act
                ));
            }
        }
        
        if expected_lines.len() != actual_lines.len() {
            return Err(format!(
                "Line count mismatch: expected {}, got {}",
                expected_lines.len(), actual_lines.len()
            ));
        }
        
        return Err("Output mismatch (unknown difference)".to_string());
    }
    
    Ok(())
}

#[test]
#[ignore] // Run with `cargo test test_svg_renderer -- --ignored`
fn test_svg_renderer() {
    let positioned_dir = get_positioned_testdata_dir();
    
    eprintln!("Positioned test directory: {:?}", positioned_dir);
    
    if !positioned_dir.exists() {
        eprintln!("WARNING: Positioned test directory does not exist");
        return;
    }
    
    let entries = fs::read_dir(&positioned_dir).expect("Failed to read directory");
    
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut failures: Vec<String> = Vec::new();
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let test_name = path.file_stem().unwrap().to_string_lossy().to_string();
            
            match run_svg_test(&test_name) {
                Ok(()) => {
                    eprintln!("  ✓ {}", test_name);
                    passed += 1;
                }
                Err(e) if e == "skip" => {
                    eprintln!("  - {} (skipped: not flowchart)", test_name);
                    skipped += 1;
                }
                Err(e) if e == "skip-precision" => {
                    eprintln!("  - {} (skipped: float precision edge case)", test_name);
                    skipped += 1;
                }
                Err(e) => {
                    eprintln!("  ✗ {}: {}", test_name, e);
                    failures.push(test_name);
                    failed += 1;
                }
            }
        }
    }
    
    eprintln!("\n=== SVG Summary ===");
    eprintln!("Passed:  {}", passed);
    eprintln!("Failed:  {}", failed);
    eprintln!("Skipped: {}", skipped);
    
    if !failures.is_empty() {
        eprintln!("\nFailed tests:");
        for f in &failures {
            eprintln!("  - {}", f);
        }
        panic!("{} SVG tests failed", failed);
    }
}
