use crate::datatypes::{CustomCostMatrix, PositionIndex, LocalIndex, Path};
use crate::log;
use screeps::{CircleStyle, Direction, Position, RoomName, RoomVisual, RoomXY, RoomCoordinate};
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

fn diagonal_distance(pos: PositionIndex, goal: PositionIndex) -> usize {
    // For room level movement, still use Manhattan since rooms are connected cardinally
    let (start_x, start_y) = pos.room().room_xy();
    let (goal_x, goal_y) = goal.room().room_xy();
    let room_distance = ((start_x as i32 - goal_x as i32).abs() + 
                       (start_y as i32 - goal_y as i32).abs()) as usize;
    
    // For local movement, use Chebyshev distance since diagonal movement costs the same as cardinal in Screeps
    let dx = (pos.local().x() as i32 - goal.local().x() as i32).abs() as usize;
    let dy = (pos.local().y() as i32 - goal.local().y() as i32).abs() as usize;
    
    // Use max instead of weighted sum since diagonal movement costs the same as cardinal
    room_distance * 50 + dx.max(dy)
}

fn heuristic(pos: PositionIndex, goal: PositionIndex) -> usize {
    diagonal_distance(pos, goal)
}

fn get_line_points(start: PositionIndex, goal: PositionIndex) -> Vec<PositionIndex> {
    let start_local_x = start.local().x() as i32;
    let start_local_y = start.local().y() as i32;
    let goal_local_x = goal.local().x() as i32;
    let goal_local_y = goal.local().y() as i32;
    
    let dx = (goal_local_x - start_local_x).abs();
    let dy = (goal_local_y - start_local_y).abs();
    
    let sx = if start_local_x < goal_local_x { 1 } else { -1 };
    let sy = if start_local_y < goal_local_y { 1 } else { -1 };
    
    let mut err = dx - dy;
    let mut points = Vec::new();
    let mut x = start_local_x;
    let mut y = start_local_y;
    let mut current_room = start.room();
    
    while x != goal_local_x || y != goal_local_y {
        let mut next_room = current_room;
        let mut next_x = x;
        let mut next_y = y;
        
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            next_x += sx;
            if next_x >= 50 {
                next_room = current_room.move_direction(Direction::Right);
                next_x -= 50;
            } else if next_x < 0 {
                next_room = current_room.move_direction(Direction::Left);
                next_x += 50;
            }
        }
        if e2 < dx {
            err += dx;
            next_y += sy;
            if next_y >= 50 {
                next_room = current_room.move_direction(Direction::Bottom);
                next_y -= 50;
            } else if next_y < 0 {
                next_room = current_room.move_direction(Direction::Top);
                next_y += 50;
            }
        }
        
        if let (Ok(x_coord), Ok(y_coord)) = (
            RoomCoordinate::new(next_x as u8),
            RoomCoordinate::new(next_y as u8)
        ) {
            if let Ok(local_index) = LocalIndex::try_from(RoomXY::new(x_coord, y_coord)) {
                points.push(PositionIndex::new(next_room, local_index));
                x = next_x;
                y = next_y;
                current_room = next_room;
            } else {
                return Vec::new();
            }
        } else {
            return Vec::new();
        }
    }
    
    points
}

fn line_of_sight_cost(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>,
) -> Option<usize> {
    let (start_x, start_y) = start.room().room_xy();
    let (goal_x, goal_y) = goal.room().room_xy();
    let room_dx = ((goal_x as i32 - start_x as i32).abs()) as usize;
    let room_dy = ((goal_y as i32 - start_y as i32).abs()) as usize;
    
    if room_dx > 1 || room_dy > 1 {
        return None;
    }
    
    let points = get_line_points(start, goal);
    if points.is_empty() {
        return None;
    }
    
    let mut total_cost = 0;
    for pos in points {
        if let Some(cost_matrix) = get_cost_matrix(pos.room_name()) {
            let cost = cost_matrix.get_local(pos.local()) as usize;
            if cost >= 255 {
                return None;
            }
            total_cost += cost;
        } else {
            return None;
        }
    }
    
    Some(total_cost)
}

fn line_of_sight(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>,
) -> bool {
    line_of_sight_cost(start, goal, get_cost_matrix).is_some()
}

pub fn theta_star_path(
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
        // log(&format!("closing position: {:?}", current_pos));
        // let viz = RoomVisual::new(Some(current_pos.room_name()));
        // viz.circle(
        //     current_pos.x().u8() as f32,
        //     current_pos.y().u8() as f32,
        //     Some(CircleStyle::default().radius(0.3).stroke("black").fill("black")),
        // );
        if current_pos == goal {
            let mut path = Vec::new();
            let mut pos = current_pos;
            path.push(pos);
            
            while pos != start {
                pos = *parents.get(&pos).unwrap();
                path.push(pos);
            }
            // log(&format!("Path found: {:?}", path));
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
                // log(&format!("checking neighbor: {:?}", neighbor));
                if closed.contains_key(&neighbor) {
                    continue;
                }
                
                if let Some(cost_matrix) = get_cost_matrix(neighbor.room_name()) {
                    let cost = cost_matrix.get_local(neighbor.local()) as usize;
                    // let viz = RoomVisual::new(Some(neighbor.room_name()));
                    if cost >= 255 {
                        // viz.circle(
                        //     neighbor.x().u8() as f32,
                        //     neighbor.y().u8() as f32,
                        //     Some(CircleStyle::default().radius(0.3).stroke("red").fill("red")),
                        // );
                        continue;
                    }
                    // viz.circle(
                    //     neighbor.x().u8() as f32,
                    //     neighbor.y().u8() as f32,
                    //     Some(CircleStyle::default().radius(0.3).stroke("white").fill("white")),
                    // );
                    let parent = *parents.get(&current_pos).unwrap();
                    let mut tentative_g_score = g_scores[&current_pos] + cost;
                    
                    if line_of_sight(parent, neighbor, &get_cost_matrix) {
                        let parent_g = g_scores[&parent];
                        if let Some(path_cost) = line_of_sight_cost(parent, neighbor, &get_cost_matrix) {
                            let direct_g = parent_g + path_cost;
                            if direct_g < tentative_g_score {
                                tentative_g_score = direct_g;
                                parents.insert(neighbor, parent);
                            } else {
                                parents.insert(neighbor, current_pos);
                            }
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
                    // log(&format!("opening position: {:?}", neighbor));
                    // let viz = RoomVisual::new(Some(neighbor.room_name()));
                    // viz.circle(
                    //     neighbor.x().u8() as f32,
                    //     neighbor.y().u8() as f32,
                    //     Some(CircleStyle::default().radius(0.3).stroke("white").fill("white")),
                    // );
                    open.push(NodeInfo {
                        f_score,
                        g_score: tentative_g_score,
                        position: neighbor,
                        parent: *parents.get(&neighbor).unwrap(),
                    });
                } else {
                    log(&format!("no cost matrix for neighbor: {:?}", neighbor));
                }
            }
        }
    }
    
    None
}

#[wasm_bindgen]
pub fn js_theta_star_path(
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

    theta_star_path(
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

