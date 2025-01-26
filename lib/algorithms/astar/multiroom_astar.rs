use crate::datatypes::{
    ClockworkCostMatrix, CustomCostMatrix, LocalIndex, MultiroomGenericMap, OptionalCache, PositionIndex, RoomIndex
};
use crate::log;
use crate::utils::set_panic_hook;
use lazy_static::lazy_static;
use screeps::{CircleStyle, Direction, RoomCoordinate, RoomName, RoomVisual, RoomXY};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::throw_val;
use js_sys::Function;
use screeps::Position;
use screeps::game::cpu;
use crate::datatypes::Path;

#[derive(Copy, Clone, Eq, PartialEq)]
struct NodeInfo {
    // g_score is the cost of the path from start to this node
    g_score: usize,
    // h_score is the estimated cost from this node to the goal
    h_score: usize,
    steps: usize,
    position: PositionIndex,
    parent: PositionIndex,
    open_direction: Option<Direction>,
}


impl Default for NodeInfo {
    fn default() -> Self {
        Self {
            g_score: usize::MAX,
            h_score: usize::MAX,
            steps: usize::MAX,
            position: PositionIndex::new(RoomIndex::new(0, 0), LocalIndex::new(0, 0)),
            parent: PositionIndex::new(RoomIndex::new(0, 0), LocalIndex::new(0, 0)),
            open_direction: None,
        }
    }
}

lazy_static! {
    static ref DIRECTION_LOOKUP: [Vec<Direction>; 9] = [
        // Any direction
        vec![
            Direction::Top,
            Direction::TopRight,
            Direction::Right,
            Direction::BottomRight,
            Direction::Bottom,
            Direction::BottomLeft,
            Direction::Left,
            Direction::TopLeft,
        ],
        // Direction::Top
        vec![Direction::Top, Direction::TopRight, Direction::TopLeft],
        // Direction::TopRight
        vec![
            Direction::TopRight,
            Direction::Top,
            Direction::Right,
            Direction::BottomRight,
            Direction::TopLeft,
        ],
        // Direction::Right
        vec![
            Direction::Right,
            Direction::BottomRight,
            Direction::TopRight,
        ],
        // Direction::BottomRight
        vec![
            Direction::BottomRight,
            Direction::Right,
            Direction::Bottom,
            Direction::TopRight,
            Direction::BottomLeft,
        ],
        // Direction::Bottom
        vec![
            Direction::Bottom,
            Direction::BottomRight,
            Direction::BottomLeft,
        ],
        // Direction::BottomLeft
        vec![
            Direction::BottomLeft,
            Direction::Left,
            Direction::Bottom,
            Direction::TopLeft,
            Direction::BottomRight,
        ],
        // Direction::Left
        vec![Direction::Left, Direction::BottomLeft, Direction::TopLeft],
        // Direction::TopLeft
        vec![
            Direction::TopLeft,
            Direction::Top,
            Direction::Left,
            Direction::BottomLeft,
            Direction::TopRight,
        ],
    ];
}

/// Returns the next directions to consider, based on the direction from which the tile
/// was entered. Lateral directions can be ruled out as an optimization.
fn next_directions(open_direction: Option<Direction>) -> &'static [Direction] {
    &DIRECTION_LOOKUP[open_direction.map(|d| d as usize).unwrap_or(0)]
}

fn heuristic(position: PositionIndex, goal: PositionIndex) -> usize {
    // h_score: estimated cost from this position to the goal
    // using chebyshev distance as our heuristic
    position.distance_to(&goal) as usize
}

pub fn astar_path(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_ops: usize,
    max_path_length: usize,
) -> Option<Vec<PositionIndex>> {
    set_panic_hook();
    // Priority queue implemented as buckets of positions with same f_score
    // where f_score = g_score + h_score
    let mut open_by_f_score: Vec<Vec<PositionIndex>> = vec![Default::default()];
    let mut current_f_score = 0;
    let mut visited_nodes = 0;
    let mut node_info = MultiroomGenericMap::<NodeInfo>::new();
    let cost_matrices = OptionalCache::new(|room: RoomName| get_cost_matrix(room));

    // Initialize start node
    let start_h_score = heuristic(start, goal);
    for i in 0..start_h_score {
        open_by_f_score.push(Default::default());
    }
    open_by_f_score[start_h_score].push(start);

    let mut current_room = start.room();
    let mut current_room_cost_matrix = if let Some(cost_matrix) = cost_matrices.get_or_create(current_room.room_name()) {
        cost_matrix
    } else {
        log(&format!("No cost matrix for room: {:?}", current_room));
        return None; // Cannot plan path without a cost matrix
    };

    let mut current_room_distance_map = node_info.get_or_create_room_map(current_room);
    current_room_distance_map.set(start.local(), NodeInfo {
        g_score: 0,  // Cost from start to start is 0
        h_score: start_h_score,
        steps: 0,
        position: start,
        parent: start,
        open_direction: None,
    });
    // log(&format!("min_f_score: {:?}, open_by_f_score.len(): {:?}", current_f_score, open_by_f_score.len()));
    while current_f_score < open_by_f_score.len() {
        // log(&format!("min_f_score: {:?}, open_by_f_score.len(): {:?}", current_f_score, open_by_f_score.len()));
        // Find next non-empty bucket
        while current_f_score < open_by_f_score.len() && open_by_f_score[current_f_score].is_empty() {
            current_f_score += 1;
        }
        if current_f_score >= open_by_f_score.len() {
            break;
        }

        let current_pos = open_by_f_score[current_f_score].pop().unwrap();
        
        if current_pos.room() != current_room {
            current_room = current_pos.room();
            current_room_cost_matrix = if let Some(cost_matrix) = cost_matrices.get_or_create(current_room.room_name()) {
                cost_matrix
            } else {
                log(&format!("No cost matrix for room: {:?}", current_room));
                return None; // Cannot plan path without a cost matrix
            };
            current_room_distance_map = node_info.get_or_create_room_map(current_room);
        }

        // let viz = RoomVisual::new(Some(current_pos.room_name()));
        // viz.circle(current_pos.x().u8() as f32, current_pos.y().u8() as f32, Some(CircleStyle::default().radius(0.3).fill("black")));
        // log(&format!("current_pos: {:?}", current_pos));
        visited_nodes += 1;
        if visited_nodes >= max_ops {
            return None;
        }

        let current_node = current_room_distance_map.get(current_pos.local()).unwrap();
        let current_g_score = current_node.g_score;
        let current_steps = current_node.steps;
        let current_open_direction = current_node.open_direction;

        // log(&format!("current_pos: {:?}, goal: {:?}, current_g_score: {:?}, current_steps: {:?}, current_open_direction: {:?}, current f_score: {:?}", current_pos, goal, current_g_score, current_steps, current_open_direction, current_f_score));
        // Check if we've reached the goal
        if current_pos == goal {
            // Reconstruct path by following parent pointers
            let mut path = Vec::new();
            let mut current = current_pos;
            
            while current != start {
                path.push(current);
                if let Some(info) = node_info.get(current) {
                    current = info.parent;
                } else {
                    log(&format!("Path reconstruction failed"));
                    return None; // Path reconstruction failed
                }
            }
            
            path.reverse();
            return Some(path);
        }

        // If this node's f_score is greater than min_f_score, we need to requeue it
        let current_f_score = current_g_score + current_node.h_score;
        if current_f_score > current_f_score {
            // Ensure we have a bucket for this f_score
            while open_by_f_score.len() <= current_f_score {
                open_by_f_score.push(Default::default());
            }
            // log(&format!("requeueing node {:?}", current_pos));
            open_by_f_score[current_f_score].push(current_pos);
            continue;
        }

        // Explore neighbors
        for direction in next_directions(current_open_direction) {
            let Some(neighbor_pos) = current_pos.r#move(*direction) else { continue; };

            // Update cost matrix if we've entered a new room
            if neighbor_pos.room() != current_room {
                let next_matrix = cost_matrices.get_or_create(neighbor_pos.room_name());
                if let Some(cost_matrix) = next_matrix {
                    current_room_cost_matrix = cost_matrix;
                    current_room = neighbor_pos.room();
                } else {
                    continue;
                }
                current_room_distance_map = node_info.get_or_create_room_map(current_room);
            }

            // Get movement cost to neighbor
            let xy = neighbor_pos.local();
            let movement_cost = current_room_cost_matrix.get_local(xy);
            if movement_cost >= 255 {
                continue; // Impassable terrain
            }

            // Calculate scores for neighbor
            let neighbor_g_score = current_g_score.saturating_add(movement_cost as usize);
            if neighbor_g_score >= max_path_length {
                continue;
            }

            // Check if this path to neighbor is better than any previous path
            if let Some(existing_neighbor) = current_room_distance_map.get(neighbor_pos.local()) {
                if neighbor_g_score >= existing_neighbor.g_score {
                    // log(&format!("skipping neighbor {:?}", neighbor_pos));
                    // log(&format!("skipping neighbor_g_score: {:?}, existing_neighbor.g_score: {:?}", neighbor_g_score, existing_neighbor.g_score));
                    continue; // This path to neighbor is not better than existing path
                }
            }

            let neighbor_h_score = heuristic(neighbor_pos, goal);
            let neighbor_f_score = neighbor_g_score + neighbor_h_score;
            
            // Ensure we have a bucket for this f_score
            while open_by_f_score.len() <= neighbor_f_score {
                open_by_f_score.push(Default::default());
            }

            // let viz = RoomVisual::new(Some(neighbor_pos.room_name()));
            // viz.circle(neighbor_pos.x().u8() as f32, neighbor_pos.y().u8() as f32, Some(CircleStyle::default().radius(0.3).fill("white")));
            // log(&format!("adding neighbor {:?}, f_score: {:?}", neighbor_pos, neighbor_f_score));
            // Add neighbor to open set
            current_room_distance_map.set(neighbor_pos.local(), NodeInfo {
                g_score: neighbor_g_score,
                h_score: neighbor_h_score,
                steps: current_steps + 1,
                position: neighbor_pos,
                parent: current_pos,
                open_direction: Some(*direction),
            });
            open_by_f_score[neighbor_f_score].push(neighbor_pos);
        }
    }

    None
}

fn get_cost_matrix_for_room(get_cost_matrix: &Function, room_name: &RoomName) -> Option<CustomCostMatrix> {
    let result = get_cost_matrix.call1(
        &JsValue::null(),
        &JsValue::from_f64(room_name.packed_repr() as f64),
    );

    let value = match result {
        Ok(value) => value,
        Err(e) => throw_val(e),
    };

    // log(&format!("Got cost matrix value: {:?}", value));

    let cost_matrix = if value.is_undefined() {
        // log("Cost matrix is undefined");
        None
    } else {
        // log(&format!("Attempting to convert cost matrix of type: {:?}", value.js_typeof()));
        // let end_cpu = cpu::get_used();
        // log(&format!("rust: CM Cpu time: {:?} {:?}", end_cpu - start_cpu, ClockworkCostMatrix::try_from(value.clone()).is_ok()));
        match CustomCostMatrix::try_from(value.clone()) {
            Ok(matrix) => Some(matrix),
            Err(e) => {
                // log(&format!("Failed to convert cost matrix: {:?}", e));
                throw_val(JsValue::from_str(&format!("Invalid CustomCostMatrix: {:?}", e)))
            }
        }
    };
    // let end_cpu = cpu::get_used();
    // log(&format!("rust: CM Cpu time: {:?}", end_cpu - start_cpu));
    cost_matrix
}

#[wasm_bindgen]
pub fn js_astar_path(
    start_packed: u32,
    goal_packed: u32,
    get_cost_matrix: Function,
    max_ops: u32,
    max_path_length: u32,
) -> Option<Path> {
    // log(&format!("A* Path called with start: {:?}, goal: {:?}", start_packed, goal_packed));
    let start = PositionIndex::from(Position::from_packed(start_packed));
    let goal = PositionIndex::from(Position::from_packed(goal_packed));

    // let start_cpu = cpu::get_used();
    // log(&format!("A* Path called with start: {:?}, goal: {:?}", start, goal));
    let result = astar_path(
        start,
        goal,
        |room_name| {
            get_cost_matrix_for_room(&get_cost_matrix, &room_name)
        },
        max_ops as usize,
        max_path_length as usize,
    )?;


    // log(&format!("A* Path results: {:?}", result));
    // let end_cpu = cpu::get_used();
    // log(&format!("rust: A* Path Cpu time: {:?}", end_cpu - start_cpu));
    let mut path = Path::new();
    for pos in result {
        path.add(Position::from(pos));
    }
    Some(path)
} 