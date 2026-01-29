//! A* pathfinding for edge routing

use super::types::GridCoord;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

/// Priority queue item
#[derive(Debug, Clone, Eq, PartialEq)]
struct PQItem {
    coord: GridCoord,
    priority: i32,
}

impl Ord for PQItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for PQItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Manhattan distance with corner penalty
pub fn heuristic(a: GridCoord, b: GridCoord) -> i32 {
    let abs_x = (a.x - b.x).abs();
    let abs_y = (a.y - b.y).abs();
    if abs_x == 0 || abs_y == 0 {
        abs_x + abs_y
    } else {
        abs_x + abs_y + 1
    }
}

/// 4-directional movement
const MOVE_DIRS: [(i32, i32); 4] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
];

/// Check if a grid cell is free
fn is_free_in_grid(grid: &HashMap<String, usize>, c: GridCoord) -> bool {
    if c.x < 0 || c.y < 0 {
        return false;
    }
    !grid.contains_key(&c.key())
}

/// Maximum iterations for A* to prevent infinite loops
const MAX_ITERATIONS: usize = 100_000;

/// Find a path from `from` to `to` using A*
pub fn get_path(
    grid: &HashMap<String, usize>,
    from: GridCoord,
    to: GridCoord,
) -> Option<Vec<GridCoord>> {
    let mut pq = BinaryHeap::new();
    pq.push(PQItem { coord: from, priority: 0 });
    
    let mut cost_so_far: HashMap<String, i32> = HashMap::new();
    cost_so_far.insert(from.key(), 0);
    
    let mut came_from: HashMap<String, Option<GridCoord>> = HashMap::new();
    came_from.insert(from.key(), None);
    
    let mut iterations = 0;
    while let Some(current) = pq.pop() {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            return None; // Give up after too many iterations
        }
        if current.coord == to {
            // Reconstruct path
            let mut path = Vec::new();
            let mut c: Option<GridCoord> = Some(current.coord);
            while let Some(coord) = c {
                path.push(coord);
                c = came_from.get(&coord.key()).and_then(|&o| o);
            }
            path.reverse();
            return Some(path);
        }
        
        let current_cost = *cost_so_far.get(&current.coord.key()).unwrap_or(&0);
        
        for (dx, dy) in MOVE_DIRS {
            let next = GridCoord::new(current.coord.x + dx, current.coord.y + dy);
            
            // Allow moving to destination even if occupied
            if !is_free_in_grid(grid, next) && next != to {
                continue;
            }
            
            let new_cost = current_cost + 1;
            let next_key = next.key();
            
            let existing_cost = cost_so_far.get(&next_key).copied();
            
            if existing_cost.is_none() || new_cost < existing_cost.unwrap() {
                cost_so_far.insert(next_key.clone(), new_cost);
                let priority = new_cost + heuristic(next, to);
                pq.push(PQItem { coord: next, priority });
                came_from.insert(next_key, Some(current.coord));
            }
        }
    }
    
    None
}

/// Simplify a path by removing intermediate waypoints on straight segments
pub fn merge_path(path: Vec<GridCoord>) -> Vec<GridCoord> {
    if path.len() <= 2 {
        return path;
    }
    
    let mut to_remove = std::collections::HashSet::new();
    
    for idx in 1..path.len() - 1 {
        let prev = path[idx - 1];
        let curr = path[idx];
        let next = path[idx + 1];
        
        let prev_dx = curr.x - prev.x;
        let prev_dy = curr.y - prev.y;
        let dx = next.x - curr.x;
        let dy = next.y - curr.y;
        
        // Same direction — middle point is redundant
        if prev_dx == dx && prev_dy == dy {
            to_remove.insert(idx);
        }
    }
    
    path.into_iter()
        .enumerate()
        .filter(|(i, _)| !to_remove.contains(i))
        .map(|(_, c)| c)
        .collect()
}
