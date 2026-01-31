# Mermaid-ASCII Rust Implementation Test Status

## Summary

| Category | Count |
|----------|-------|
| **Passed** | 31 |
| **Failed** | 26 |
| **Skipped** | 3 |
| **Total** | 60 |

## Passing Tests (31)

### Basic Flowchart Tests
- single_node
- single_node_longer_name
- two_nodes_linked
- two_nodes_longer_names
- three_nodes
- three_nodes_single_line
- flowchart_tb_simple
- comments

### Multi-Root/Disconnected Tests
- two_root_nodes
- two_root_nodes_longer_names
- two_single_root_nodes
- two_layer_single_graph
- two_layer_single_graph_longer_names

### Edge Cases
- self_reference
- self_reference_with_edge
- back_reference_from_child
- backlink_from_bottom
- backlink_from_top
- duplicate_labels
- preserve_order_of_definition

### Ampersand (Multiple Nodes) Tests
- ampersand_lhs
- ampersand_rhs
- ampersand_lhs_and_rhs
- ampersand_without_edge

### Subgraph Tests
- subgraph_empty (empty subgraphs are correctly ignored)

### Sequence Diagram Tests
- seq_basic
- seq_multiple_messages
- seq_self_message

### Class Diagram Tests
- cls_basic
- cls_methods

### ER Diagram Tests
- er_basic

## Skipped Tests (3)

These tests use custom padding syntax (`paddingX=`, `paddingY=`) which is not yet implemented:

- backlink_with_short_y_padding
- custom_padding
- subgraph_td_multiple_paddingy

## Failing Tests (26)

### Subgraph Tests (16)
These require proper subgraph layout with borders:
- graph_tb_direction
- graph_bt_direction
- nested_subgraphs_with_labels
- subgraph_complex_nested
- subgraph_complex_mixed
- subgraph_mixed_nodes
- subgraph_mixed_nodes_td
- subgraph_multiple_edges
- subgraph_multiple_nodes
- subgraph_nested
- subgraph_nested_with_external
- subgraph_node_outside_lr
- subgraph_single_node
- subgraph_td_direction
- subgraph_td_multiple
- subgraph_three_levels_nested
- subgraph_three_separate
- subgraph_two_separate
- subgraph_with_labels

### Class Diagram Tests (5)
These require relationship line rendering:
- cls_all_relationships
- cls_annotation
- cls_association
- cls_dependency
- cls_inheritance

### ER Diagram Tests (2)
These require attribute support:
- er_attributes
- er_identifying

## Known Issues

1. **Subgraph Layout**: Subgraphs with nodes inside them don't render borders correctly. The bounding box calculation and offset handling needs work.

2. **Class Diagram Relationships**: Relationships between classes (inheritance, association, etc.) need proper line drawing with vertical layout.

3. **ER Diagram Attributes**: Entities with attributes need proper multi-row box rendering.

4. **Custom Padding**: The `paddingX=` and `paddingY=` header syntax is not implemented in the parser.

## Next Steps

1. Fix subgraph bounding box calculation and border drawing
2. Implement class diagram relationship line rendering  
3. Add attribute support to ER diagram entities
4. Parse custom padding headers
