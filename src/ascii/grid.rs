//! Grid-based layout operations

use super::types::{
    AsciiGraph, AsciiNode, DrawingCoord, Direction, GridCoord,
    GraphDirection, grid_coord_direction, determine_direction, get_opposite,
    UP, DOWN, LEFT, RIGHT, UPPER_LEFT, UPPER_RIGHT, LOWER_LEFT, LOWER_RIGHT, MIDDLE,
};
use super::pathfinder::{get_path, merge_path};
use super::canvas::set_canvas_size_to_grid;

/// Check if a node is in any subgraph
fn is_node_in_any_subgraph(graph: &AsciiGraph, node_idx: usize) -> bool {
    graph.subgraphs.iter().any(|sg| sg.node_indices.contains(&node_idx))
}

/// Get the subgraph index that contains a node (first match)
fn get_node_subgraph(graph: &AsciiGraph, node_idx: usize) -> Option<usize> {
    for (sg_idx, sg) in graph.subgraphs.iter().enumerate() {
        if sg.node_indices.contains(&node_idx) {
            return Some(sg_idx);
        }
    }
    None
}

/// Check if a node has an incoming edge from outside its subgraph
/// AND is the topmost such node in its subgraph.
fn has_incoming_edge_from_outside_subgraph(graph: &AsciiGraph, node_idx: usize) -> bool {
    let node_sg_idx = match get_node_subgraph(graph, node_idx) {
        Some(idx) => idx,
        None => return false,
    };
    
    let mut has_external_edge = false;
    for edge in &graph.edges {
        if edge.to_idx == node_idx {
            let source_sg_idx = get_node_subgraph(graph, edge.from_idx);
            if source_sg_idx != Some(node_sg_idx) {
                has_external_edge = true;
                break;
            }
        }
    }
    
    if !has_external_edge {
        return false;
    }
    
    // Only return true for the topmost node with an external incoming edge
    let node_gc = match graph.nodes[node_idx].grid_coord {
        Some(gc) => gc,
        None => return false,
    };
    
    for &other_idx in &graph.subgraphs[node_sg_idx].node_indices {
        if other_idx == node_idx {
            continue;
        }
        let other_gc = match graph.nodes[other_idx].grid_coord {
            Some(gc) => gc,
            None => continue,
        };
        
        // Check if this other node also has an external incoming edge
        let mut other_has_external = false;
        for edge in &graph.edges {
            if edge.to_idx == other_idx {
                let source_sg_idx = get_node_subgraph(graph, edge.from_idx);
                if source_sg_idx != Some(node_sg_idx) {
                    other_has_external = true;
                    break;
                }
            }
        }
        
        if other_has_external && other_gc.y < node_gc.y {
            return false; // Another node is topmost
        }
    }
    
    true
}

/// Convert a grid coordinate to a drawing (character) coordinate
pub fn grid_to_drawing_coord(graph: &AsciiGraph, c: GridCoord, dir: Option<Direction>) -> DrawingCoord {
    let target = if let Some(d) = dir {
        GridCoord::new(c.x + d.x, c.y + d.y)
    } else {
        c
    };
    
    let mut x = 0i32;
    for col in 0..target.x {
        x += *graph.column_width.get(&col).unwrap_or(&0) as i32;
    }
    
    let mut y = 0i32;
    for row in 0..target.y {
        y += *graph.row_height.get(&row).unwrap_or(&0) as i32;
    }
    
    let col_w = *graph.column_width.get(&target.x).unwrap_or(&0) as i32;
    let row_h = *graph.row_height.get(&target.y).unwrap_or(&0) as i32;
    
    DrawingCoord::new(
        x + col_w / 2 + graph.offset_x,
        y + row_h / 2 + graph.offset_y,
    )
}

/// Convert a grid coordinate to top-left drawing coordinate (for box placement)
pub fn grid_to_drawing_coord_topleft(graph: &AsciiGraph, c: GridCoord) -> DrawingCoord {
    let mut x = 0i32;
    for col in 0..c.x {
        x += *graph.column_width.get(&col).unwrap_or(&0) as i32;
    }
    
    let mut y = 0i32;
    for row in 0..c.y {
        y += *graph.row_height.get(&row).unwrap_or(&0) as i32;
    }
    
    DrawingCoord::new(
        x + graph.offset_x,
        y + graph.offset_y,
    )
}

/// Convert a path of grid coords to drawing coords
pub fn line_to_drawing(graph: &AsciiGraph, line: &[GridCoord]) -> Vec<DrawingCoord> {
    line.iter().map(|c| grid_to_drawing_coord(graph, *c, None)).collect()
}

/// Reserve a 3x3 block in the grid for a node
pub fn reserve_spot_in_grid(
    graph: &mut AsciiGraph,
    node_idx: usize,
    requested: GridCoord,
) -> GridCoord {
    if graph.grid.contains_key(&requested.key()) {
        // Collision — shift perpendicular to main flow direction
        let new_pos = if graph.config.graph_direction == GraphDirection::LR {
            GridCoord::new(requested.x, requested.y + 4)
        } else {
            GridCoord::new(requested.x + 4, requested.y)
        };
        return reserve_spot_in_grid(graph, node_idx, new_pos);
    }
    
    // Reserve the 3x3 block
    for dx in 0..3 {
        for dy in 0..3 {
            let reserved = GridCoord::new(requested.x + dx, requested.y + dy);
            graph.grid.insert(reserved.key(), node_idx);
        }
    }
    
    graph.nodes[node_idx].grid_coord = Some(requested);
    requested
}

/// Set column widths and row heights for a node's 3x3 grid block
pub fn set_column_width(graph: &mut AsciiGraph, node_idx: usize) {
    let gc = match graph.nodes[node_idx].grid_coord {
        Some(c) => c,
        None => return,
    };
    let label_len = graph.nodes[node_idx].display_label.len();
    let padding = graph.config.box_border_padding;
    
    // 3 columns: [border=1] [content=2*padding+labelLen] [border=1]
    let col_widths = [1, 2 * padding + label_len, 1];
    // 3 rows: [border=1] [content=1+2*padding] [border=1]
    let row_heights = [1, 1 + 2 * padding, 1];
    
    for (idx, &w) in col_widths.iter().enumerate() {
        let x_coord = gc.x + idx as i32;
        let current = *graph.column_width.get(&x_coord).unwrap_or(&0);
        graph.column_width.insert(x_coord, current.max(w));
    }
    
    for (idx, &h) in row_heights.iter().enumerate() {
        let y_coord = gc.y + idx as i32;
        let current = *graph.row_height.get(&y_coord).unwrap_or(&0);
        graph.row_height.insert(y_coord, current.max(h));
    }
    
    // Padding column/row before the node
    if gc.x > 0 {
        let current = *graph.column_width.get(&(gc.x - 1)).unwrap_or(&0);
        graph.column_width.insert(gc.x - 1, current.max(graph.config.padding_x));
    }
    
    if gc.y > 0 {
        let mut base_padding = graph.config.padding_y;
        // Extra vertical padding for nodes with incoming edges from outside their subgraph
        if has_incoming_edge_from_outside_subgraph(graph, node_idx) {
            let subgraph_overhead = 4;
            base_padding += subgraph_overhead;
        }
        let current = *graph.row_height.get(&(gc.y - 1)).unwrap_or(&0);
        graph.row_height.insert(gc.y - 1, current.max(base_padding));
    }
}

/// Increase grid size for path coordinates
pub fn increase_grid_size_for_path(graph: &mut AsciiGraph, path: &[GridCoord]) {
    for c in path {
        if !graph.column_width.contains_key(&c.x) {
            graph.column_width.insert(c.x, graph.config.padding_x / 2);
        }
        if !graph.row_height.contains_key(&c.y) {
            graph.row_height.insert(c.y, graph.config.padding_y / 2);
        }
    }
}

/// Determine start and end directions for an edge
pub fn determine_start_and_end_dir(
    from_coord: GridCoord,
    to_coord: GridCoord,
    is_self_ref: bool,
    graph_direction: GraphDirection,
) -> (Direction, Direction, Direction, Direction) {
    if is_self_ref {
        return if graph_direction == GraphDirection::LR {
            (RIGHT, DOWN, DOWN, RIGHT)
        } else {
            (DOWN, RIGHT, RIGHT, DOWN)
        };
    }
    
    let d = determine_direction(from_coord, to_coord);
    
    let is_backwards = if graph_direction == GraphDirection::LR {
        d == LEFT || d == UPPER_LEFT || d == LOWER_LEFT
    } else {
        d == UP || d == UPPER_LEFT || d == UPPER_RIGHT
    };
    
    if d == LOWER_RIGHT {
        if graph_direction == GraphDirection::LR {
            (DOWN, LEFT, RIGHT, UP)
        } else {
            (RIGHT, UP, DOWN, LEFT)
        }
    } else if d == UPPER_RIGHT {
        if graph_direction == GraphDirection::LR {
            (UP, LEFT, RIGHT, DOWN)
        } else {
            (RIGHT, DOWN, UP, LEFT)
        }
    } else if d == LOWER_LEFT {
        if graph_direction == GraphDirection::LR {
            (DOWN, DOWN, LEFT, UP)
        } else {
            (LEFT, UP, DOWN, RIGHT)
        }
    } else if d == UPPER_LEFT {
        if graph_direction == GraphDirection::LR {
            (DOWN, DOWN, LEFT, DOWN)
        } else {
            (RIGHT, RIGHT, UP, RIGHT)
        }
    } else if is_backwards {
        if graph_direction == GraphDirection::LR && d == LEFT {
            (DOWN, DOWN, LEFT, RIGHT)
        } else if graph_direction == GraphDirection::TD && d == UP {
            (RIGHT, RIGHT, UP, DOWN)
        } else {
            (d, get_opposite(d), d, get_opposite(d))
        }
    } else {
        (d, get_opposite(d), d, get_opposite(d))
    }
}

/// Determine the path for an edge
pub fn determine_path(graph: &mut AsciiGraph, edge_idx: usize) {
    let from_idx = graph.edges[edge_idx].from_idx;
    let to_idx = graph.edges[edge_idx].to_idx;
    let is_self_ref = from_idx == to_idx;
    
    let from_coord = match graph.nodes.get(from_idx).and_then(|n| n.grid_coord) {
        Some(c) => c,
        None => return,
    };
    let to_coord = match graph.nodes.get(to_idx).and_then(|n| n.grid_coord) {
        Some(c) => c,
        None => return,
    };
    
    let (pref_dir, pref_opp, alt_dir, alt_opp) = determine_start_and_end_dir(
        from_coord, to_coord, is_self_ref, graph.config.graph_direction,
    );
    
    // Try preferred path
    let pref_from = grid_coord_direction(from_coord, pref_dir);
    let pref_to = grid_coord_direction(to_coord, pref_opp);
    let preferred_path = get_path(&graph.grid, pref_from, pref_to);
    
    if preferred_path.is_none() {
        graph.edges[edge_idx].start_dir = alt_dir;
        graph.edges[edge_idx].end_dir = alt_opp;
        graph.edges[edge_idx].path = Vec::new();
        return;
    }
    let preferred_path = merge_path(preferred_path.unwrap());
    
    // Try alternative path
    let alt_from = grid_coord_direction(from_coord, alt_dir);
    let alt_to = grid_coord_direction(to_coord, alt_opp);
    let alternative_path = get_path(&graph.grid, alt_from, alt_to);
    
    if alternative_path.is_none() {
        graph.edges[edge_idx].start_dir = pref_dir;
        graph.edges[edge_idx].end_dir = pref_opp;
        graph.edges[edge_idx].path = preferred_path;
        return;
    }
    let alternative_path = merge_path(alternative_path.unwrap());
    
    // Pick shorter path
    if preferred_path.len() <= alternative_path.len() {
        graph.edges[edge_idx].start_dir = pref_dir;
        graph.edges[edge_idx].end_dir = pref_opp;
        graph.edges[edge_idx].path = preferred_path;
    } else {
        graph.edges[edge_idx].start_dir = alt_dir;
        graph.edges[edge_idx].end_dir = alt_opp;
        graph.edges[edge_idx].path = alternative_path;
    }
}

/// Find the best line segment in an edge's path to place a label on.
/// Picks the first segment wide enough for the label, or the widest segment overall.
/// Also increases the column width at the label position to fit the text.
pub fn determine_label_line(graph: &mut AsciiGraph, edge_idx: usize) {
    let edge = &graph.edges[edge_idx];
    if edge.text.is_empty() || edge.path.len() < 2 {
        return;
    }
    
    let len_label = edge.text.len();
    let mut prev_step = edge.path[0];
    let mut largest_line: (GridCoord, GridCoord) = (prev_step, edge.path[1]);
    let mut largest_line_size = 0;
    
    for i in 1..edge.path.len() {
        let step = edge.path[i];
        let line = (prev_step, step);
        let line_width = calculate_line_width(graph, line);
        
        if line_width >= len_label {
            largest_line = line;
            break;
        } else if line_width > largest_line_size {
            largest_line_size = line_width;
            largest_line = line;
        }
        prev_step = step;
    }
    
    // Ensure column at midpoint is wide enough for the label
    let min_x = largest_line.0.x.min(largest_line.1.x);
    let max_x = largest_line.0.x.max(largest_line.1.x);
    let middle_x = min_x + (max_x - min_x) / 2;
    
    let current = *graph.column_width.get(&middle_x).unwrap_or(&0);
    graph.column_width.insert(middle_x, current.max(len_label + 2));
    
    graph.edges[edge_idx].label_line = vec![largest_line.0, largest_line.1];
}

/// Calculate the total character width of a line segment by summing column widths.
fn calculate_line_width(graph: &AsciiGraph, line: (GridCoord, GridCoord)) -> usize {
    let mut total = 0;
    let start_x = line.0.x.min(line.1.x);
    let end_x = line.0.x.max(line.1.x);
    for x in start_x..=end_x {
        total += *graph.column_width.get(&x).unwrap_or(&0);
    }
    total
}

/// Get children of a node (nodes this node has edges to)
fn get_children(graph: &AsciiGraph, node_idx: usize) -> Vec<usize> {
    let mut children = Vec::new();
    for edge in &graph.edges {
        if edge.from_idx == node_idx {
            children.push(edge.to_idx);
        }
    }
    children
}

/// Create the node-to-grid mapping
pub fn create_mapping(graph: &mut AsciiGraph) {
    let dir = graph.config.graph_direction;
    let mut highest_position_per_level: Vec<i32> = vec![0; 100];
    
    // Identify root nodes — nodes that aren't seen as children before they appear
    // This preserves the order of first definition
    let mut nodes_seen = std::collections::HashSet::new();
    let mut root_indices = Vec::new();
    
    for idx in 0..graph.nodes.len() {
        if !nodes_seen.contains(&idx) {
            root_indices.push(idx);
        }
        nodes_seen.insert(idx);
        for child_idx in get_children(graph, idx) {
            nodes_seen.insert(child_idx);
        }
    }
    
    // If no roots found, pick the first node as root
    let root_indices = if root_indices.is_empty() && !graph.nodes.is_empty() {
        vec![0]
    } else {
        root_indices
    };
    
    // In LR mode with both external and subgraph roots, separate them
    // so subgraph roots are placed one level deeper
    let mut has_external_roots = false;
    let mut has_subgraph_roots_with_edges = false;
    
    for &idx in &root_indices {
        if is_node_in_any_subgraph(graph, idx) {
            if !get_children(graph, idx).is_empty() {
                has_subgraph_roots_with_edges = true;
            }
        } else {
            has_external_roots = true;
        }
    }
    
    let should_separate = dir == GraphDirection::LR && has_external_roots && has_subgraph_roots_with_edges;
    
    let (external_roots, subgraph_roots): (Vec<usize>, Vec<usize>) = if should_separate {
        root_indices.iter().partition(|&&idx| !is_node_in_any_subgraph(graph, idx))
    } else {
        (root_indices.clone(), Vec::new())
    };
    
    // Place external root nodes at level 0
    for &root_idx in &external_roots {
        let level = 0;
        let pos = highest_position_per_level[level as usize];
        let requested = if dir == GraphDirection::LR {
            GridCoord::new(level, pos)
        } else {
            GridCoord::new(pos, level)
        };
        reserve_spot_in_grid(graph, root_idx, requested);
        highest_position_per_level[level as usize] += 4;
    }
    
    // Place subgraph root nodes at level 4 (one level in from the edge)
    if should_separate && !subgraph_roots.is_empty() {
        let subgraph_level = 4i32;
        for &root_idx in &subgraph_roots {
            let pos = highest_position_per_level[subgraph_level as usize];
            let requested = if dir == GraphDirection::LR {
                GridCoord::new(subgraph_level, pos)
            } else {
                GridCoord::new(pos, subgraph_level)
            };
            reserve_spot_in_grid(graph, root_idx, requested);
            highest_position_per_level[subgraph_level as usize] += 4;
        }
    }
    
    // Place child nodes level by level (BFS-style traversal)
    let all_placed_roots: Vec<usize> = external_roots.iter().chain(subgraph_roots.iter()).cloned().collect();
    let mut queue: Vec<usize> = all_placed_roots.clone();
    let mut visited: std::collections::HashSet<usize> = all_placed_roots.iter().cloned().collect();
    
    while !queue.is_empty() {
        let current_idx = queue.remove(0);
        let gc = match graph.nodes[current_idx].grid_coord {
            Some(c) => c,
            None => continue,
        };
        
        let child_level = if dir == GraphDirection::LR { gc.x + 4 } else { gc.y + 4 };
        
        for child_idx in get_children(graph, current_idx) {
            if visited.contains(&child_idx) {
                continue;
            }
            
            if graph.nodes[child_idx].grid_coord.is_some() {
                continue; // Already placed
            }
            
            let highest_position = highest_position_per_level.get(child_level as usize).copied().unwrap_or(0);
            
            let requested = if dir == GraphDirection::LR {
                GridCoord::new(child_level, highest_position)
            } else {
                GridCoord::new(highest_position, child_level)
            };
            
            reserve_spot_in_grid(graph, child_idx, requested);
            
            if (child_level as usize) < highest_position_per_level.len() {
                highest_position_per_level[child_level as usize] = highest_position + 4;
            }
            
            visited.insert(child_idx);
            queue.push(child_idx);
        }
    }
    
    // Set column widths and row heights BEFORE determining paths
    for i in 0..graph.nodes.len() {
        set_column_width(graph, i);
    }
    
    // Determine edge paths (now that column widths are set)
    for i in 0..graph.edges.len() {
        determine_path(graph, i);
        determine_label_line(graph, i);
        increase_grid_size_for_path(graph, &graph.edges[i].path.clone());
    }
    
    // Convert grid coords to drawing coords and generate node box drawings
    for i in 0..graph.nodes.len() {
        if let Some(gc) = graph.nodes[i].grid_coord {
            let dc = grid_to_drawing_coord_topleft(graph, gc);
            graph.nodes[i].drawing_coord = Some(dc);
            
            // Generate the node box drawing
            let drawing = super::draw::draw_box(&graph.nodes[i], graph);
            graph.nodes[i].drawing = Some(drawing);
        }
    }
    
    // Resize canvas
    set_canvas_size_to_grid(
        &mut graph.canvas,
        &graph.column_width,
        &graph.row_height,
        graph.offset_x,
        graph.offset_y,
    );
}
