//! ASCII renderer type definitions

/// Logical grid coordinate — nodes occupy 3x3 blocks on this grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCoord {
    pub x: i32,
    pub y: i32,
}

impl GridCoord {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn key(&self) -> String {
        format!("{},{}", self.x, self.y)
    }
}

/// Character-level coordinate on the 2D text canvas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawingCoord {
    pub x: i32,
    pub y: i32,
}

impl DrawingCoord {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Direction constants for positions on a node's 3x3 grid block
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Direction {
    pub x: i32,
    pub y: i32,
}

pub const UP: Direction = Direction { x: 1, y: 0 };
pub const DOWN: Direction = Direction { x: 1, y: 2 };
pub const LEFT: Direction = Direction { x: 0, y: 1 };
pub const RIGHT: Direction = Direction { x: 2, y: 1 };
pub const UPPER_RIGHT: Direction = Direction { x: 2, y: 0 };
pub const UPPER_LEFT: Direction = Direction { x: 0, y: 0 };
pub const LOWER_RIGHT: Direction = Direction { x: 2, y: 2 };
pub const LOWER_LEFT: Direction = Direction { x: 0, y: 2 };
pub const MIDDLE: Direction = Direction { x: 1, y: 1 };

/// 2D text canvas — column-major (canvas[x][y])
pub type Canvas = Vec<Vec<char>>;

/// Graph direction for layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphDirection {
    LR,
    TD,
}

/// Configuration for ASCII rendering
#[derive(Debug, Clone)]
pub struct AsciiConfig {
    pub use_ascii: bool,
    pub padding_x: usize,
    pub padding_y: usize,
    pub box_border_padding: usize,
    pub graph_direction: GraphDirection,
}

/// A node in the ASCII graph
#[derive(Debug, Clone)]
pub struct AsciiNode {
    pub name: String,
    pub display_label: String,
    pub index: usize,
    pub grid_coord: Option<GridCoord>,
    pub drawing_coord: Option<DrawingCoord>,
    pub drawing: Option<Canvas>,
    pub drawn: bool,
}

impl AsciiNode {
    pub fn new(name: String, display_label: String, index: usize) -> Self {
        Self {
            name,
            display_label,
            index,
            grid_coord: None,
            drawing_coord: None,
            drawing: None,
            drawn: false,
        }
    }
}

/// An edge in the ASCII graph
#[derive(Debug, Clone)]
pub struct AsciiEdge {
    pub from_idx: usize,
    pub to_idx: usize,
    pub text: String,
    pub path: Vec<GridCoord>,
    pub label_line: Vec<GridCoord>,
    pub start_dir: Direction,
    pub end_dir: Direction,
}

impl AsciiEdge {
    pub fn new(from_idx: usize, to_idx: usize, text: String) -> Self {
        Self {
            from_idx,
            to_idx,
            text,
            path: Vec::new(),
            label_line: Vec::new(),
            start_dir: DOWN,
            end_dir: UP,
        }
    }
}

/// A subgraph container with bounding box
#[derive(Debug, Clone)]
pub struct AsciiSubgraph {
    pub name: String,
    pub node_indices: Vec<usize>,
    pub parent_idx: Option<usize>,
    pub children_idx: Vec<usize>,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl AsciiSubgraph {
    pub fn new(name: String) -> Self {
        Self {
            name,
            node_indices: Vec::new(),
            parent_idx: None,
            children_idx: Vec::new(),
            min_x: 0,
            min_y: 0,
            max_x: 0,
            max_y: 0,
        }
    }
}

/// Full ASCII graph state
#[derive(Debug, Clone)]
pub struct AsciiGraph {
    pub nodes: Vec<AsciiNode>,
    pub edges: Vec<AsciiEdge>,
    pub canvas: Canvas,
    pub grid: std::collections::HashMap<String, usize>,
    pub column_width: std::collections::HashMap<i32, usize>,
    pub row_height: std::collections::HashMap<i32, usize>,
    pub subgraphs: Vec<AsciiSubgraph>,
    pub config: AsciiConfig,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl AsciiGraph {
    pub fn new(config: AsciiConfig) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            canvas: vec![vec![' '; 1]; 1],
            grid: std::collections::HashMap::new(),
            column_width: std::collections::HashMap::new(),
            row_height: std::collections::HashMap::new(),
            subgraphs: Vec::new(),
            config,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

/// Apply a direction offset to a grid coordinate
pub fn grid_coord_direction(c: GridCoord, dir: Direction) -> GridCoord {
    GridCoord {
        x: c.x + dir.x,
        y: c.y + dir.y,
    }
}

/// Get the opposite direction
pub fn get_opposite(d: Direction) -> Direction {
    if d == UP {
        return DOWN;
    }
    if d == DOWN {
        return UP;
    }
    if d == LEFT {
        return RIGHT;
    }
    if d == RIGHT {
        return LEFT;
    }
    if d == UPPER_RIGHT {
        return LOWER_LEFT;
    }
    if d == UPPER_LEFT {
        return LOWER_RIGHT;
    }
    if d == LOWER_RIGHT {
        return UPPER_LEFT;
    }
    if d == LOWER_LEFT {
        return UPPER_RIGHT;
    }
    MIDDLE
}

/// Determine 8-way direction from one coordinate to another
pub fn determine_direction(from: GridCoord, to: GridCoord) -> Direction {
    if from.x == to.x {
        if from.y < to.y {
            DOWN
        } else {
            UP
        }
    } else if from.y == to.y {
        if from.x < to.x {
            RIGHT
        } else {
            LEFT
        }
    } else if from.x < to.x {
        if from.y < to.y {
            LOWER_RIGHT
        } else {
            UPPER_RIGHT
        }
    } else if from.y < to.y {
        LOWER_LEFT
    } else {
        UPPER_LEFT
    }
}

pub fn determine_direction_drawing(from: DrawingCoord, to: DrawingCoord) -> Direction {
    if from.x == to.x {
        if from.y < to.y {
            DOWN
        } else {
            UP
        }
    } else if from.y == to.y {
        if from.x < to.x {
            RIGHT
        } else {
            LEFT
        }
    } else if from.x < to.x {
        if from.y < to.y {
            LOWER_RIGHT
        } else {
            UPPER_RIGHT
        }
    } else if from.y < to.y {
        LOWER_LEFT
    } else {
        UPPER_LEFT
    }
}
