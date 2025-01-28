use crate::datatypes::{
    ClockworkCostMatrix, CustomCostMatrix, LocalIndex, MultiroomGenericMap, OptionalCache, PositionIndex, RoomIndex, Path
};
use crate::log;
use lazy_static::lazy_static;
use screeps::{Direction, RoomName};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use wasm_bindgen::prelude::*;
use js_sys::Function;

#[derive(Copy, Clone, Eq, PartialEq)]
struct NodeInfo {
    f_score: usize,
    g_score: usize,
    position: PositionIndex,
    parent: PositionIndex,
}

impl Ord for NodeInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for NodeInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn heuristic(position: PositionIndex, goal: PositionIndex) -> usize {
    let dx = (position.room.x as i32 - goal.room.x as i32).abs() as usize * 50 +
             (position.local.x as i32 - goal.local.x as i32).abs() as usize;
    let dy = (position.room.y as i32 - goal.room.y as i32).abs() as usize * 50 +
             (position.local.y as i32 - goal.local.y as i32).abs() as usize;
    ((dx * dx + dy * dy) as f64).sqrt() as usize
}

fn line_of_sight(
    start: PositionIndex,
    end: PositionIndex,
    get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>,
    blocked_rooms: &HashSet<RoomIndex>,
) -> bool {
    if blocked_rooms.contains(&start.room) || blocked_rooms.contains(&end.room) {
        return false;
    }
    
    let dx = (end.local.x as i32 - start.local.x as i32) + 
             (end.room.x as i32 - start.room.x as i32) * 50;
    let dy = (end.local.y as i32 - start.local.y as i32) +
             (end.room.y as i32 - start.room.y as i32) * 50;
    
    let n = dx.abs().max(dy.abs()) as usize;
    if n == 0 {
        return true;
    }
    
    let x_inc = dx as f64 / n as f64;
    let y_inc = dy as f64 / n as f64;
    
    let mut x = start.local.x as f64 + (start.room.x as i32 * 50) as f64;
    let mut y = start.local.y as f64 + (start.room.y as i32 * 50) as f64;
    
    for _ in 0..=n {
        let room_x = (x / 50.0).floor() as i32;
        let room_y = (y / 50.0).floor() as i32;
        let local_x = (x % 50.0) as u8;
        let local_y = (y % 50.0) as u8;
        
        let room = RoomIndex::new(room_x as u32, room_y as u32);
        if blocked_rooms.contains(&room) {
            return false;
        }
        
        let local = LocalIndex::new(local_x, local_y);
        let pos = PositionIndex::new(room, local);
        
        if let Some(matrix) = get_cost_matrix(pos.room.to_room_name()) {
            if matrix.get_cost(pos.local) == 255 {
                return false;
            }
        } else {
            return false;
        }
        
        x += x_inc;
        y += y_inc;
    }
    
    true
}

pub fn lazy_theta_star_path(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_ops: usize,
    max_path_length: usize,
) -> Option<Vec<PositionIndex>> {
    let mut ops = 0;
    let mut open = BinaryHeap::new();
    let mut closed = HashMap::new();
    let mut g_scores = HashMap::new();
    let mut parents = HashMap::new();
    let mut blocked_rooms = HashSet::new();
    
    // Initialize blocked rooms set
    let room_range = 3; // Configurable range to check for blocked rooms
    for dx in -room_range..=room_range {
        for dy in -room_range..=room_range {
            let room_x = start.room.x as i32 + dx;
            let room_y = start.room.y as i32 + dy;
            if room_x >= 0 && room_y >= 0 {
                let room = RoomIndex::new(room_x as u32, room_y as u32);
                if get_cost_matrix(room.to_room_name()).is_none() {
                    blocked_rooms.insert(room);
                }
            }
        }
    }
    
    g_scores.insert(start, 0);
    parents.insert(start, start);
    open.push(NodeInfo {
        f_score: heuristic(start, goal),
        g_score: 0,
        position: start,
        parent: start,
    });
    
    while let Some(current) = open.pop() {
        ops += 1;
        if ops >= max_ops {
            return None;
        }
        
        if current.position == goal {
            let mut path = Vec::new();
            let mut pos = goal;
            while pos != start {
                path.push(pos);
                pos = *parents.get(&pos).unwrap();
            }
            path.push(start);
            path.reverse();
            return Some(path);
        }
        
        if closed.contains_key(&current.position) {
            continue;
        }
        
        // Verify line of sight with parent
        let parent = *parents.get(&current.position).unwrap();
        if parent != current.position {
            if !line_of_sight(parent, current.position, &get_cost_matrix, &blocked_rooms) {
                // Find a new parent among visible neighbors
                let mut best_g = usize::MAX;
                let mut best_parent = current.position;
                
                for direction in Direction::all() {
                    if let Some(neighbor) = current.position.step(direction) {
                        if let Some(&neighbor_g) = g_scores.get(&neighbor) {
                            if let Some(cost_matrix) = get_cost_matrix(neighbor.room.to_room_name()) {
                                let cost = cost_matrix.get_cost(current.position.local) as usize;
                                if cost < 255 {
                                    let new_g = neighbor_g + cost;
                                    if new_g < best_g {
                                        best_g = new_g;
                                        best_parent = neighbor;
                                    }
                                }
                            }
                        }
                    }
                }
                
                if best_g == usize::MAX {
                    continue;
                }
                
                g_scores.insert(current.position, best_g);
                parents.insert(current.position, best_parent);
                current.g_score = best_g;
                current.f_score = best_g + heuristic(current.position, goal);
            }
        }
        
        closed.insert(current.position, current);
        
        // Process neighbors
        for direction in Direction::all() {
            if let Some(neighbor) = current.position.step(direction) {
                if closed.contains_key(&neighbor) || blocked_rooms.contains(&neighbor.room) {
                    continue;
                }
                
                if let Some(cost_matrix) = get_cost_matrix(neighbor.room.to_room_name()) {
                    let cost = cost_matrix.get_cost(neighbor.local);
                    if cost == 255 {
                        continue;
                    }
                    
                    // Lazy evaluation: assume we can reach from parent
                    let parent = *parents.get(&current.position).unwrap();
                    let tentative_g = if parent == current.position {
                        current.g_score + cost as usize
                    } else {
                        g_scores[&parent] + heuristic(parent, neighbor)
                    };
                    
                    if !g_scores.contains_key(&neighbor) || tentative_g < *g_scores.get(&neighbor).unwrap() {
                        g_scores.insert(neighbor, tentative_g);
                        parents.insert(neighbor, parent);
                        
                        if !closed.contains_key(&neighbor) {
                            open.push(NodeInfo {
                                f_score: tentative_g + heuristic(neighbor, goal),
                                g_score: tentative_g,
                                position: neighbor,
                                parent,
                            });
                        }
                    }
                }
            }
        }
    }
    
    None
}

#[wasm_bindgen]
pub fn js_lazy_theta_star_path(
    start_packed: u32,
    goal_packed: u32,
    get_cost_matrix: Function,
    max_ops: u32,
    max_path_length: u32,
) -> Option<Path> {
    let start = PositionIndex::from(start_packed);
    let goal = PositionIndex::from(goal_packed);
    
    let get_cost_matrix = move |room_name: RoomName| -> Option<CustomCostMatrix> {
        let result = get_cost_matrix
            .call1(&JsValue::NULL, &JsValue::from(room_name.to_string()))
            .ok()?;
        if result.is_null() || result.is_undefined() {
            return None;
        }
        Some(CustomCostMatrix::try_from(result).ok()?)
    };
    
    lazy_theta_star_path(
        start,
        goal,
        get_cost_matrix,
        max_ops as usize,
        max_path_length as usize,
    ).map(|positions| Path::new(positions))
}
