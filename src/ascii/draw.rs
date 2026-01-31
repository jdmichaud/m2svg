//! Drawing operations for ASCII rendering

use super::types::{
    AsciiGraph, AsciiNode, Canvas, Direction, DrawingCoord, GridCoord,
    determine_direction_drawing, UP, DOWN, LEFT, RIGHT,
    UPPER_LEFT, UPPER_RIGHT, LOWER_LEFT, LOWER_RIGHT, MIDDLE,
};
use super::canvas::{mk_canvas, copy_canvas, get_canvas_size, set_char, get_char, merge_canvases};
use super::grid::{grid_to_drawing_coord, grid_to_drawing_coord_topleft};

/// Draw a node box with centered label text
pub fn draw_box(node: &AsciiNode, graph: &AsciiGraph) -> Canvas {
    let gc = match node.grid_coord {
        Some(c) => c,
        None => return mk_canvas(0, 0),
    };
    let use_ascii = graph.config.use_ascii;
    
    // Width spans 2 columns (border + content)
    let mut w = 0i32;
    for i in 0..2 {
        w += *graph.column_width.get(&(gc.x + i)).unwrap_or(&0) as i32;
    }
    // Height spans 2 rows (border + content)
    let mut h = 0i32;
    for i in 0..2 {
        h += *graph.row_height.get(&(gc.y + i)).unwrap_or(&0) as i32;
    }
    
    let mut box_canvas = mk_canvas(w.max(0) as usize, h.max(0) as usize);
    
    // Box-drawing characters
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    
    // Draw horizontal lines
    for x in 1..w {
        set_char(&mut box_canvas, x, 0, h_line);
        set_char(&mut box_canvas, x, h, h_line);
    }
    // Draw vertical lines
    for y in 1..h {
        set_char(&mut box_canvas, 0, y, v_line);
        set_char(&mut box_canvas, w, y, v_line);
    }
    // Draw corners
    set_char(&mut box_canvas, 0, 0, tl);
    set_char(&mut box_canvas, w, 0, tr);
    set_char(&mut box_canvas, 0, h, bl);
    set_char(&mut box_canvas, w, h, br);
    
    // Center the label (matching TypeScript: floor(w/2) - ceil(label.len/2) + 1)
    let label = &node.display_label;
    let text_y = h / 2;
    let label_half = (label.len() as i32 + 1) / 2; // ceil division
    let text_x = w / 2 - label_half + 1;
    for (i, c) in label.chars().enumerate() {
        set_char(&mut box_canvas, text_x + i as i32, text_y, c);
    }
    
    box_canvas
}

/// Draw a line between two drawing coordinates
pub fn draw_line(
    canvas: &mut Canvas,
    from: DrawingCoord,
    to: DrawingCoord,
    offset_from: i32,
    offset_to: i32,
    use_ascii: bool,
) -> Vec<DrawingCoord> {
    let dir = determine_direction_drawing(from, to);
    let mut drawn_coords = Vec::new();
    
    let (h_char, v_char, bslash, fslash) = if use_ascii {
        ('-', '|', '\\', '/')
    } else {
        ('─', '│', '╲', '╱')
    };
    
    if dir == UP {
        for y in ((to.y - offset_to)..=(from.y - offset_from)).rev() {
            drawn_coords.push(DrawingCoord::new(from.x, y));
            set_char(canvas, from.x, y, v_char);
        }
    } else if dir == DOWN {
        for y in (from.y + offset_from)..=(to.y + offset_to) {
            drawn_coords.push(DrawingCoord::new(from.x, y));
            set_char(canvas, from.x, y, v_char);
        }
    } else if dir == LEFT {
        for x in ((to.x - offset_to)..=(from.x - offset_from)).rev() {
            drawn_coords.push(DrawingCoord::new(x, from.y));
            set_char(canvas, x, from.y, h_char);
        }
    } else if dir == RIGHT {
        for x in (from.x + offset_from)..=(to.x + offset_to) {
            drawn_coords.push(DrawingCoord::new(x, from.y));
            set_char(canvas, x, from.y, h_char);
        }
    } else if dir == UPPER_LEFT {
        let mut x = from.x;
        let mut y = from.y - offset_from;
        while x >= to.x - offset_to && y >= to.y - offset_to {
            drawn_coords.push(DrawingCoord::new(x, y));
            set_char(canvas, x, y, bslash);
            x -= 1;
            y -= 1;
        }
    } else if dir == UPPER_RIGHT {
        let mut x = from.x;
        let mut y = from.y - offset_from;
        while x <= to.x + offset_to && y >= to.y - offset_to {
            drawn_coords.push(DrawingCoord::new(x, y));
            set_char(canvas, x, y, fslash);
            x += 1;
            y -= 1;
        }
    } else if dir == LOWER_LEFT {
        let mut x = from.x;
        let mut y = from.y + offset_from;
        while x >= to.x - offset_to && y <= to.y + offset_to {
            drawn_coords.push(DrawingCoord::new(x, y));
            set_char(canvas, x, y, fslash);
            x -= 1;
            y += 1;
        }
    } else if dir == LOWER_RIGHT {
        let mut x = from.x;
        let mut y = from.y + offset_from;
        while x <= to.x + offset_to && y <= to.y + offset_to {
            drawn_coords.push(DrawingCoord::new(x, y));
            set_char(canvas, x, y, bslash);
            x += 1;
            y += 1;
        }
    }
    
    drawn_coords
}

/// Draw an arrowhead at the end of a path
pub fn draw_arrow_head(
    canvas: &mut Canvas,
    last_line: &[DrawingCoord],
    fallback_dir: Direction,
    use_ascii: bool,
) {
    if last_line.is_empty() {
        return;
    }
    
    let last_pos = last_line.last().unwrap();
    let dir = if last_line.len() > 1 {
        let from = &last_line[0];
        determine_direction_drawing(*from, *last_pos)
    } else {
        fallback_dir
    };
    
    let c = if !use_ascii {
        match dir {
            d if d == UP => '▲',
            d if d == DOWN => '▼',
            d if d == LEFT => '◄',
            d if d == RIGHT => '►',
            d if d == UPPER_RIGHT => '◥',
            d if d == UPPER_LEFT => '◤',
            d if d == LOWER_RIGHT => '◢',
            d if d == LOWER_LEFT => '◣',
            _ => '●',
        }
    } else {
        match dir {
            d if d == UP => '^',
            d if d == DOWN => 'v',
            d if d == LEFT => '<',
            d if d == RIGHT => '>',
            _ => '*',
        }
    };
    
    set_char(canvas, last_pos.x, last_pos.y, c);
}

/// Draw corner characters at path bends
pub fn draw_corners(graph: &AsciiGraph, path: &[GridCoord]) -> Canvas {
    let mut canvas = copy_canvas(&graph.canvas);
    
    for idx in 1..path.len().saturating_sub(1) {
        let prev = path[idx - 1];
        let coord = path[idx];
        let next = path[idx + 1];
        
        let dc = grid_to_drawing_coord(graph, coord, None);
        let prev_dir = determine_direction_drawing(
            grid_to_drawing_coord(graph, prev, None),
            dc,
        );
        let next_dir = determine_direction_drawing(dc, grid_to_drawing_coord(graph, next, None));
        
        let corner = if graph.config.use_ascii {
            '+'
        } else {
            determine_corner(prev_dir, next_dir)
        };
        
        set_char(&mut canvas, dc.x, dc.y, corner);
    }
    
    canvas
}

/// Determine the correct corner character for a path bend
fn determine_corner(from_dir: Direction, to_dir: Direction) -> char {
    // from_dir: direction of travel BEFORE the corner
    // to_dir: direction of travel AFTER the corner
    // 
    // The corner character connects:
    //   - The opposite of from_dir (where we came from)
    //   - to_dir (where we're going)
    //
    // Corner shapes:
    //   ┌ = RIGHT + DOWN
    //   ┐ = LEFT + DOWN  
    //   └ = RIGHT + UP
    //   ┘ = LEFT + UP
    match (from_dir, to_dir) {
        // Was going UP (came from below), turning to horizontal
        (d1, d2) if d1 == UP && d2 == RIGHT => '┌',
        (d1, d2) if d1 == UP && d2 == LEFT => '┐',
        // Was going DOWN (came from above), turning to horizontal
        (d1, d2) if d1 == DOWN && d2 == RIGHT => '└',
        (d1, d2) if d1 == DOWN && d2 == LEFT => '┘',
        // Was going RIGHT (came from left), turning to vertical
        (d1, d2) if d1 == RIGHT && d2 == DOWN => '┐',
        (d1, d2) if d1 == RIGHT && d2 == UP => '┘',
        // Was going LEFT (came from right), turning to vertical
        (d1, d2) if d1 == LEFT && d2 == DOWN => '┌',
        (d1, d2) if d1 == LEFT && d2 == UP => '└',
        _ => '┼',
    }
}

/// Draw the path lines for an edge
fn draw_path(graph: &AsciiGraph, path: &[GridCoord]) -> (Canvas, Vec<Vec<DrawingCoord>>, Vec<Direction>) {
    let mut canvas = copy_canvas(&graph.canvas);
    let mut lines_drawn: Vec<Vec<DrawingCoord>> = Vec::new();
    let mut line_dirs: Vec<Direction> = Vec::new();
    
    if path.is_empty() {
        return (canvas, lines_drawn, line_dirs);
    }
    
    let mut previous_coord = path[0];
    
    for i in 1..path.len() {
        let next_coord = path[i];
        let prev_dc = grid_to_drawing_coord(graph, previous_coord, None);
        let next_dc = grid_to_drawing_coord(graph, next_coord, None);
        
        if prev_dc == next_dc {
            previous_coord = next_coord;
            continue;
        }
        
        let dir = determine_direction_drawing(prev_dc, next_dc);
        let mut segment = draw_line(&mut canvas, prev_dc, next_dc, 1, -1, graph.config.use_ascii);
        if segment.is_empty() {
            segment.push(prev_dc);
        }
        lines_drawn.push(segment);
        line_dirs.push(dir);
        previous_coord = next_coord;
    }
    
    (canvas, lines_drawn, line_dirs)
}

/// Draw a complete arrow (edge) returning separate layer canvases
/// Returns (path, corners, arrowhead, label)
pub fn draw_arrow_layers(graph: &AsciiGraph, edge_idx: usize) -> (Canvas, Canvas, Canvas, Canvas) {
    let edge = &graph.edges[edge_idx];
    let empty = copy_canvas(&graph.canvas);
    
    if edge.path.is_empty() {
        return (empty.clone(), empty.clone(), empty.clone(), empty);
    }
    
    let label_canvas = draw_arrow_label(graph, edge_idx);
    let (path_canvas, lines_drawn, line_dirs) = draw_path(graph, &edge.path);
    
    // Corners
    let corners_canvas = draw_corners(graph, &edge.path);
    
    // Arrowhead
    let mut arrow_head_canvas = copy_canvas(&graph.canvas);
    if !lines_drawn.is_empty() {
        let last_line = lines_drawn.last().unwrap();
        let fallback_dir = line_dirs.last().copied().unwrap_or(DOWN);
        draw_arrow_head(&mut arrow_head_canvas, last_line, fallback_dir, graph.config.use_ascii);
    }
    
    // Also add box start junction to corners canvas in Unicode mode
    let mut combined_corners = corners_canvas;
    if !graph.config.use_ascii && !lines_drawn.is_empty() && edge.path.len() > 1 {
        let first_line = &lines_drawn[0];
        if !first_line.is_empty() {
            let from = first_line[0];
            let dir = determine_direction_drawing(
                grid_to_drawing_coord(graph, edge.path[0], None),
                grid_to_drawing_coord(graph, edge.path[1], None),
            );
            
            if dir == UP {
                set_char(&mut combined_corners, from.x, from.y + 1, '┴');
            } else if dir == DOWN {
                set_char(&mut combined_corners, from.x, from.y - 1, '┬');
            } else if dir == LEFT {
                set_char(&mut combined_corners, from.x + 1, from.y, '┤');
            } else if dir == RIGHT {
                set_char(&mut combined_corners, from.x - 1, from.y, '├');
            }
        }
    }
    
    (path_canvas, combined_corners, arrow_head_canvas, label_canvas)
}

/// Legacy wrapper for draw_arrow
pub fn draw_arrow(graph: &AsciiGraph, edge_idx: usize) -> Vec<Canvas> {
    let (path, corners, arrowhead, label) = draw_arrow_layers(graph, edge_idx);
    vec![path, corners, arrowhead, label]
}

/// Draw an edge label
fn draw_arrow_label(graph: &AsciiGraph, edge_idx: usize) -> Canvas {
    let mut canvas = copy_canvas(&graph.canvas);
    let edge = &graph.edges[edge_idx];
    
    if edge.text.is_empty() || edge.path.len() < 2 {
        return canvas;
    }
    
    // Find a good position for the label (midpoint of path)
    let mid_idx = edge.path.len() / 2;
    let mid_coord = edge.path[mid_idx];
    let dc = grid_to_drawing_coord(graph, mid_coord, None);
    
    // Draw label centered on the midpoint
    let label = &edge.text;
    let start_x = dc.x - (label.len() as i32) / 2;
    for (i, c) in label.chars().enumerate() {
        set_char(&mut canvas, start_x + i as i32, dc.y - 1, c);
    }
    
    canvas
}

/// Draw a subgraph border
pub fn draw_subgraph_border(
    canvas: &mut Canvas,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    use_ascii: bool,
) {
    if max_x <= min_x || max_y <= min_y {
        return;
    }
    
    let (h_line, v_line, tl, tr, bl, br) = if use_ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };
    
    // Draw horizontal lines
    for x in (min_x + 1)..max_x {
        set_char(canvas, x, min_y, h_line);
        set_char(canvas, x, max_y, h_line);
    }
    
    // Draw vertical lines
    for y in (min_y + 1)..max_y {
        set_char(canvas, min_x, y, v_line);
        set_char(canvas, max_x, y, v_line);
    }
    
    // Draw corners
    set_char(canvas, min_x, min_y, tl);
    set_char(canvas, max_x, min_y, tr);
    set_char(canvas, min_x, max_y, bl);
    set_char(canvas, max_x, max_y, br);
}

/// Draw a subgraph label (centered at top, inside the border)
pub fn draw_subgraph_label(
    canvas: &mut Canvas,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    label: &str,
) {
    if label.is_empty() || max_x <= min_x {
        return;
    }
    
    let width = max_x - min_x;
    let label_y = min_y + 1;  // Second row (inside the border)
    let mut label_x = min_x + width / 2 - (label.len() as i32) / 2;
    if label_x < min_x + 1 {
        label_x = min_x + 1;
    }
    
    for (i, c) in label.chars().enumerate() {
        if (label_x + i as i32) < max_x {
            set_char(canvas, label_x + i as i32, label_y, c);
        }
    }
}

/// Sort subgraphs by depth (shallowest first) for correct layered rendering
fn sort_subgraphs_by_depth(subgraphs: &[super::types::AsciiSubgraph]) -> Vec<usize> {
    fn get_depth(subgraphs: &[super::types::AsciiSubgraph], idx: usize) -> usize {
        match subgraphs[idx].parent_idx {
            None => 0,
            Some(parent) => 1 + get_depth(subgraphs, parent),
        }
    }
    
    let mut indices: Vec<usize> = (0..subgraphs.len()).collect();
    indices.sort_by_key(|&idx| get_depth(subgraphs, idx));
    indices
}

/// Draw the complete graph
/// Drawing order:
/// 1. Subgraph borders (bottom layer)
/// 2. Node boxes
/// 3. Edge paths (lines)
/// 4. Edge corners
/// 5. Arrowheads
/// 6. Box-start junctions
/// 7. Edge labels
/// 8. Subgraph labels (top layer)
pub fn draw_graph(graph: &mut AsciiGraph) {
    let use_ascii = graph.config.use_ascii;
    
    // 1. Draw subgraph borders FIRST (bottom layer)
    let sorted_sg_indices = sort_subgraphs_by_depth(&graph.subgraphs);
    for sg_idx in sorted_sg_indices.iter() {
        let sg = &graph.subgraphs[*sg_idx];
        if sg.node_indices.is_empty() {
            continue;
        }
        draw_subgraph_border(
            &mut graph.canvas,
            sg.min_x,
            sg.min_y,
            sg.max_x,
            sg.max_y,
            use_ascii,
        );
    }
    
    // 2. Draw all nodes
    for i in 0..graph.nodes.len() {
        let node = &graph.nodes[i];
        if node.drawn {
            continue;
        }
        
        let box_canvas = draw_box(node, graph);
        let gc = match node.grid_coord {
            Some(c) => c,
            None => continue,
        };
        
        // Use the stored drawing coordinate (which includes offsets)
        let offset = match node.drawing_coord {
            Some(dc) => dc,
            None => grid_to_drawing_coord_topleft(graph, gc),
        };
        graph.canvas = merge_canvases(&graph.canvas, offset, use_ascii, &[&box_canvas]);
        graph.nodes[i].drawn = true;
    }
    
    // 3-7. Collect all edge layers separately, then merge them in order
    // This ensures corners appear on top of paths, arrowheads on top of corners, etc.
    let mut path_canvases: Vec<Canvas> = Vec::new();
    let mut corner_canvases: Vec<Canvas> = Vec::new();
    let mut arrowhead_canvases: Vec<Canvas> = Vec::new();
    let mut label_canvases: Vec<Canvas> = Vec::new();
    
    for i in 0..graph.edges.len() {
        let (path_c, corner_c, arrowhead_c, label_c) = draw_arrow_layers(graph, i);
        path_canvases.push(path_c);
        corner_canvases.push(corner_c);
        arrowhead_canvases.push(arrowhead_c);
        label_canvases.push(label_c);
    }
    
    // Merge layers in order
    let zero = DrawingCoord::new(0, 0);
    for pc in &path_canvases {
        graph.canvas = merge_canvases(&graph.canvas, zero, use_ascii, &[pc]);
    }
    for cc in &corner_canvases {
        graph.canvas = merge_canvases(&graph.canvas, zero, use_ascii, &[cc]);
    }
    for ac in &arrowhead_canvases {
        graph.canvas = merge_canvases(&graph.canvas, zero, use_ascii, &[ac]);
    }
    for lc in &label_canvases {
        graph.canvas = merge_canvases(&graph.canvas, zero, use_ascii, &[lc]);
    }
    
    // 8. Draw subgraph labels LAST (top layer)
    for sg_idx in sorted_sg_indices {
        let sg = &graph.subgraphs[sg_idx];
        if sg.node_indices.is_empty() {
            continue;
        }
        draw_subgraph_label(
            &mut graph.canvas,
            sg.min_x,
            sg.min_y,
            sg.max_x,
            &sg.name,
        );
    }
}
