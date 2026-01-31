//! 2D text canvas operations

use super::types::{Canvas, DrawingCoord};

/// Create a blank canvas filled with spaces
pub fn mk_canvas(width: usize, height: usize) -> Canvas {
    let mut canvas = Vec::with_capacity(width + 1);
    for _ in 0..=width {
        canvas.push(vec![' '; height + 1]);
    }
    canvas
}

/// Get canvas dimensions (max_x, max_y)
pub fn get_canvas_size(canvas: &Canvas) -> (usize, usize) {
    if canvas.is_empty() {
        return (0, 0);
    }
    (canvas.len().saturating_sub(1), canvas[0].len().saturating_sub(1))
}

/// Create a copy of a canvas with same dimensions
pub fn copy_canvas(source: &Canvas) -> Canvas {
    let (max_x, max_y) = get_canvas_size(source);
    mk_canvas(max_x, max_y)
}

/// Grow the canvas to fit at least (new_x, new_y)
pub fn increase_size(canvas: &mut Canvas, new_x: usize, new_y: usize) {
    let (curr_x, curr_y) = get_canvas_size(canvas);
    let target_x = new_x.max(curr_x);
    let target_y = new_y.max(curr_y);
    
    // Extend existing columns
    for col in canvas.iter_mut() {
        col.resize(target_y + 1, ' ');
    }
    
    // Add new columns
    while canvas.len() <= target_x {
        canvas.push(vec![' '; target_y + 1]);
    }
}

/// Set a character at position (x, y) in the canvas
pub fn set_char(canvas: &mut Canvas, x: i32, y: i32, c: char) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    increase_size(canvas, x, y);
    canvas[x][y] = c;
}

/// Get a character at position (x, y) from the canvas
pub fn get_char(canvas: &Canvas, x: i32, y: i32) -> char {
    if x < 0 || y < 0 {
        return ' ';
    }
    let x = x as usize;
    let y = y as usize;
    if x < canvas.len() && y < canvas[x].len() {
        canvas[x][y]
    } else {
        ' '
    }
}

/// ASCII line characters for junction merging
const ASCII_LINE_CHARS: &[char] = &['-', '|', '+', '>', '<', '^', 'v'];

pub fn is_ascii_line_char(c: char) -> bool {
    ASCII_LINE_CHARS.contains(&c)
}

/// Merge two ASCII junction characters
pub fn merge_ascii_junctions(c1: char, c2: char) -> char {
    match (c1, c2) {
        // Crossing lines create +
        ('-', '|') | ('|', '-') => '+',
        
        // Line meets junction - keep junction
        ('-', '+') | ('+', '-') => '+',
        ('|', '+') | ('+', '|') => '+',
        
        // Arrow meets line - keep arrow
        ('>', '-') | ('>', '+') => '>',
        ('-', '>') | ('+', '>') => '>',
        ('<', '-') | ('<', '+') => '<',
        ('-', '<') | ('+', '<') => '<',
        ('^', '|') | ('^', '+') => '^',
        ('|', '^') | ('+', '^') => '^',
        ('v', '|') | ('v', '+') => 'v',
        ('|', 'v') | ('+', 'v') => 'v',
        
        // Line meets itself - keep line
        ('-', '-') => '-',
        ('|', '|') => '|',
        ('+', '+') => '+',
        
        _ => c2,  // Default to the new character
    }
}

/// All Unicode box-drawing characters that participate in junction merging
const JUNCTION_CHARS: &[char] = &[
    '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '╴', '╵', '╶', '╷',
];

pub fn is_junction_char(c: char) -> bool {
    JUNCTION_CHARS.contains(&c)
}

/// Merge two junction characters
pub fn merge_junctions(c1: char, c2: char) -> char {
    match (c1, c2) {
        ('─', '│') | ('│', '─') => '┼',
        ('─', '┌') | ('┌', '─') => '┬',
        ('─', '┐') | ('┐', '─') => '┬',
        ('─', '└') | ('└', '─') => '┴',
        ('─', '┘') | ('┘', '─') => '┴',
        ('─', '├') | ('├', '─') => '┼',
        ('─', '┤') | ('┤', '─') => '┼',
        ('│', '┌') | ('┌', '│') => '├',
        ('│', '┐') | ('┐', '│') => '┤',
        ('│', '└') | ('└', '│') => '├',
        ('│', '┘') | ('┘', '│') => '┤',
        ('│', '┬') | ('┬', '│') => '┼',
        ('│', '┴') | ('┴', '│') => '┼',
        ('│', '├') | ('├', '│') => '├',  // T-junction going right
        ('│', '┤') | ('┤', '│') => '┤',  // T-junction going left
        // Corner merging: opposite corners combine to full cross
        ('┌', '┘') | ('┘', '┌') => '┼',
        ('┐', '└') | ('└', '┐') => '┼',
        // Corner merging: same-side corners combine to T-junctions
        ('┌', '└') | ('└', '┌') => '├',  // Both have RIGHT arm → ├
        ('┐', '┘') | ('┘', '┐') => '┤',  // Both have LEFT arm → ┤
        ('┌', '┐') | ('┐', '┌') => '┬',  // Both have DOWN arm → ┬
        ('└', '┘') | ('┘', '└') => '┴',  // Both have UP arm → ┴
        // T-junction merging
        ('┬', '┴') | ('┴', '┬') => '┼',
        ('├', '┤') | ('┤', '├') => '┼',
        // T-junction + corner = full cross or enhanced T
        ('├', '┐') | ('┐', '├') => '┼',  // ├ (UP,DOWN,RIGHT) + ┐ (LEFT,DOWN) → ┼
        ('├', '┘') | ('┘', '├') => '┼',  // ├ (UP,DOWN,RIGHT) + ┘ (LEFT,UP) → ┼
        ('┤', '┌') | ('┌', '┤') => '┼',  // ┤ (UP,DOWN,LEFT) + ┌ (RIGHT,DOWN) → ┼
        ('┤', '└') | ('└', '┤') => '┼',  // ┤ (UP,DOWN,LEFT) + └ (RIGHT,UP) → ┼
        ('┬', '└') | ('└', '┬') => '┼',  // ┬ (LEFT,RIGHT,DOWN) + └ (RIGHT,UP) → ┼
        ('┬', '┘') | ('┘', '┬') => '┼',  // ┬ (LEFT,RIGHT,DOWN) + ┘ (LEFT,UP) → ┼
        ('┴', '┌') | ('┌', '┴') => '┼',  // ┴ (LEFT,RIGHT,UP) + ┌ (RIGHT,DOWN) → ┼
        ('┴', '┐') | ('┐', '┴') => '┼',  // ┴ (LEFT,RIGHT,UP) + ┐ (LEFT,DOWN) → ┼
        _ => c2,  // Default to the new character
    }
}

/// Merge overlay canvases onto a base canvas at the given offset
pub fn merge_canvases(
    base: &Canvas,
    offset: DrawingCoord,
    use_ascii: bool,
    overlays: &[&Canvas],
) -> Canvas {
    let (mut max_x, mut max_y) = get_canvas_size(base);
    
    for overlay in overlays {
        let (o_x, o_y) = get_canvas_size(overlay);
        if offset.x >= 0 && offset.y >= 0 {
            max_x = max_x.max(o_x.saturating_add(offset.x as usize));
            max_y = max_y.max(o_y.saturating_add(offset.y as usize));
        }
    }
    
    let mut merged = mk_canvas(max_x, max_y);
    
    // Copy base
    for x in 0..=max_x {
        for y in 0..=max_y {
            if x < base.len() && y < base[x].len() {
                merged[x][y] = base[x][y];
            }
        }
    }
    
    // Apply overlays
    for overlay in overlays {
        let (o_x, o_y) = get_canvas_size(overlay);
        for x in 0..=o_x {
            for y in 0..=o_y {
                let c = overlay[x][y];
                if c != ' ' {
                    let mx_i32 = x as i32 + offset.x;
                    let my_i32 = y as i32 + offset.y;
                    if mx_i32 >= 0 && my_i32 >= 0 {
                        let mx = mx_i32 as usize;
                        let my = my_i32 as usize;
                        // Grow canvas if needed
                        if mx >= merged.len() || my >= merged.get(0).map(|v| v.len()).unwrap_or(0) {
                            increase_size(&mut merged, mx, my);
                        }
                        if mx < merged.len() && my < merged[mx].len() {
                            let current = merged[mx][my];
                            // Only merge junctions in Unicode mode
                            if !use_ascii && is_junction_char(c) && is_junction_char(current) {
                                merged[mx][my] = merge_junctions(current, c);
                            } else {
                                // In ASCII mode (or non-junction chars), just overwrite
                                merged[mx][my] = c;
                            }
                        }
                    }
                }
            }
        }
    }
    
    merged
}

/// Convert the canvas to a multi-line string
pub fn canvas_to_string(canvas: &Canvas) -> String {
    let (max_x, max_y) = get_canvas_size(canvas);
    let mut lines = Vec::new();
    
    for y in 0..=max_y {
        let mut line = String::new();
        for x in 0..=max_x {
            if x < canvas.len() && y < canvas[x].len() {
                line.push(canvas[x][y]);
            } else {
                line.push(' ');
            }
        }
        lines.push(line);
    }
    
    // Remove trailing empty lines
    while !lines.is_empty() {
        let last = lines.last().unwrap();
        if last.chars().all(|c| c == ' ') {
            lines.pop();
        } else {
            break;
        }
    }
    
    lines.join("\n")
}

/// Flip the canvas vertically
pub fn flip_canvas_vertically(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut flipped: Vec<String> = Vec::with_capacity(lines.len());
    
    for line in lines.iter().rev() {
        let mut new_line = String::with_capacity(line.len());
        for c in line.chars() {
            let flipped_char = match c {
                '▲' => '▼',
                '▼' => '▲',
                '^' => 'v',
                'v' => '^',
                '┌' => '└',
                '└' => '┌',
                '┐' => '┘',
                '┘' => '┐',
                '┬' => '┴',
                '┴' => '┬',
                '╵' => '╷',
                '╷' => '╵',
                _ => c,
            };
            new_line.push(flipped_char);
        }
        flipped.push(new_line);
    }
    
    flipped.join("\n")
}

/// Draw text onto canvas starting at position
pub fn draw_text(canvas: &mut Canvas, x: i32, y: i32, text: &str) {
    for (i, c) in text.chars().enumerate() {
        set_char(canvas, x + i as i32, y, c);
    }
}

/// Set canvas size to match grid dimensions
pub fn set_canvas_size_to_grid(
    canvas: &mut Canvas,
    column_width: &std::collections::HashMap<i32, usize>,
    row_height: &std::collections::HashMap<i32, usize>,
    offset_x: i32,
    offset_y: i32,
) {
    let max_col = column_width.keys().max().copied().unwrap_or(0);
    let max_row = row_height.keys().max().copied().unwrap_or(0);
    
    let mut total_width = 0usize;
    for col in 0..=max_col {
        total_width += column_width.get(&col).copied().unwrap_or(0);
    }
    
    let mut total_height = 0usize;
    for row in 0..=max_row {
        total_height += row_height.get(&row).copied().unwrap_or(0);
    }
    
    increase_size(canvas, (total_width + offset_x as usize).max(1), (total_height + offset_y as usize).max(1));
}
