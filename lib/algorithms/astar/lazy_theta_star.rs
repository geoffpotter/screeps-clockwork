use crate::datatypes::{CustomCostMatrix, PositionIndex, Path, LocalIndex};
use screeps::{Direction, RoomName, Position};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::convert::TryFrom;
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

fn heuristic(pos: PositionIndex, goal: PositionIndex) -> usize {
    // Manhattan distance between rooms plus Manhattan distance within room
    let (start_x, start_y) = pos.room().room_xy();
    let (goal_x, goal_y) = goal.room().room_xy();
    let room_distance = ((start_x as i32 - goal_x as i32).abs() + 
                       (start_y as i32 - goal_y as i32).abs()) as usize;
    
    let local_distance = ((pos.local().x() as i32 - goal.local().x() as i32).abs() +
                        (pos.local().y() as i32 - goal.local().y() as i32).abs()) as usize;
    
    room_distance * 50 + local_distance
}

fn line_of_sight(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>,
) -> bool {
    let (start_x, start_y) = start.room().room_xy();
    let (goal_x, goal_y) = goal.room().room_xy();
    let room_dx = ((goal_x as i32 - start_x as i32).abs()) as usize;
    let room_dy = ((goal_y as i32 - start_y as i32).abs()) as usize;
    
    if room_dx > 1 || room_dy > 1 {
        return false;
    }
    
    let start_local_x = start.local().x() as usize;
    let start_local_y = start.local().y() as usize;
    let goal_local_x = goal.local().x() as usize;
    let goal_local_y = goal.local().y() as usize;
    
    let dx = goal_local_x as i32 - start_local_x as i32;
    let dy = goal_local_y as i32 - start_local_y as i32;
    
    let n = dx.abs().max(dy.abs()) as usize;
    if n == 0 {
        return true;
    }
    
    let x_inc = dx as f32 / n as f32;
    let y_inc = dy as f32 / n as f32;
    
    let mut x = start_local_x as f32;
    let mut y = start_local_y as f32;
    
    for _ in 0..n {
        x += x_inc;
        y += y_inc;
        
        let check_x = x.round() as usize;
        let check_y = y.round() as usize;
        
        let check_pos = if check_x >= 50 {
            let next_room = start.room().move_direction(Direction::Right);
            PositionIndex::new(next_room, LocalIndex::new((check_x - 50) as u8, check_y as u8))
        } else if check_x < 0 {
            let next_room = start.room().move_direction(Direction::Left);
            PositionIndex::new(next_room, LocalIndex::new((check_x + 50) as u8, check_y as u8))
        } else if check_y >= 50 {
            let next_room = start.room().move_direction(Direction::Bottom);
            PositionIndex::new(next_room, LocalIndex::new(check_x as u8, (check_y - 50) as u8))
        } else if check_y < 0 {
            let next_room = start.room().move_direction(Direction::Top);
            PositionIndex::new(next_room, LocalIndex::new(check_x as u8, (check_y + 50) as u8))
        } else {
            PositionIndex::new(start.room(), LocalIndex::new(check_x as u8, check_y as u8))
        };
        
        if let Some(cost_matrix) = get_cost_matrix(check_pos.room_name()) {
            let cost = cost_matrix.get_local(check_pos.local());
            if cost >= 255 {
                return false;
            }
        } else {
            return false;
        }
    }
    
    true
}

pub fn lazy_theta_star_path(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_ops: usize,
) -> Option<Vec<PositionIndex>> {
    let mut ops = 0;
    let mut open = BinaryHeap::new();
    let mut closed = HashMap::new();
    let mut g_scores = HashMap::new();
    let mut parents = HashMap::new();
    
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
        
        let current_pos = current.position;
        if current_pos == goal {
            let mut path = Vec::new();
            let mut pos = current_pos;
            path.push(pos);
            
            while pos != start {
                pos = *parents.get(&pos).unwrap();
                path.push(pos);
            }
            
            path.reverse();
            return Some(path);
        }
        
        closed.insert(current_pos, current);
        
        let directions = [
            Direction::Top,
            Direction::TopRight,
            Direction::Right,
            Direction::BottomRight,
            Direction::Bottom,
            Direction::BottomLeft,
            Direction::Left,
            Direction::TopLeft,
        ];
        
        for &dir in &directions {
            if let Some(neighbor) = current_pos.r#move(dir) {
                if closed.contains_key(&neighbor) {
                    continue;
                }
                
                if let Some(cost_matrix) = get_cost_matrix(neighbor.room_name()) {
                    let cost = cost_matrix.get_local(neighbor.local()) as usize;
                    if cost >= 255 {
                        continue;
                    }
                    
                    let parent = *parents.get(&current_pos).unwrap();
                    let mut tentative_g_score = g_scores[&current_pos] + cost;
                    
                    if line_of_sight(parent, neighbor, &get_cost_matrix) {
                        let parent_g = g_scores[&parent];
                        let direct_g = parent_g + heuristic(parent, neighbor);
                        if direct_g < tentative_g_score {
                            tentative_g_score = direct_g;
                            parents.insert(neighbor, parent);
                        } else {
                            parents.insert(neighbor, current_pos);
                        }
                    } else {
                        parents.insert(neighbor, current_pos);
                    }
                    
                    if let Some(&neighbor_g) = g_scores.get(&neighbor) {
                        if tentative_g_score >= neighbor_g {
                            continue;
                        }
                    }
                    
                    g_scores.insert(neighbor, tentative_g_score);
                    let f_score = tentative_g_score + heuristic(neighbor, goal);
                    
                    open.push(NodeInfo {
                        f_score,
                        g_score: tentative_g_score,
                        position: neighbor,
                        parent: *parents.get(&neighbor).unwrap(),
                    });
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
) -> Option<Path> {
    let start = Position::from_packed(start_packed);
    let goal = Position::from_packed(goal_packed);
    let start = PositionIndex::from(start);
    let goal = PositionIndex::from(goal);

    let get_cost_matrix = |room_name: RoomName| {
        let result = get_cost_matrix.call1(
            &JsValue::NULL,
            &JsValue::from_f64(room_name.packed_repr() as f64),
        );
        match result {
            Ok(value) => {
                if value.is_undefined() {
                    None
                } else {
                    CustomCostMatrix::try_from(value).ok()
                }
            }
            Err(_) => None,
        }
    };

    lazy_theta_star_path(
        start,
        goal,
        get_cost_matrix,
        max_ops as usize,
    )
    .map(|positions| {
        let screeps_positions: Vec<Position> = positions.into_iter().map(Position::from).collect();
        Path::from(screeps_positions)
    })
}
