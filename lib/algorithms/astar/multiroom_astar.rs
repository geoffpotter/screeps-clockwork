use crate::datatypes::{
    ClockworkCostMatrix, CustomCostMatrix, IndexedRoomDataCache, LocalIndex, MultiroomGenericMap, OptionalCache, PositionIndex, RoomDataCache, RoomIndex
};
use crate::log;
use crate::utils::set_panic_hook;
use lazy_static::lazy_static;
use screeps::{CircleStyle, Direction, RoomCoordinate, RoomName, RoomVisual, RoomXY, LocalCostMatrix};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::throw_val;
use js_sys::Function;
use screeps::Position;
use screeps::game::cpu;
use crate::datatypes::Path;

#[derive(Copy, Clone)]
struct State {
    g_score: usize,
    position: PositionIndex,
    parent: PositionIndex,
    open_direction: Option<Direction>,
    room_key: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            g_score: usize::MAX,
            position: PositionIndex::new(RoomIndex::new(0, 0), LocalIndex::new(0, 0)),
            parent: PositionIndex::new(RoomIndex::new(0, 0), LocalIndex::new(0, 0)),
            open_direction: None,
            room_key: 0,
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
    
    // Use bucket-based open list
    let mut open_by_f_score: Vec<Vec<State>> = vec![Default::default()];
    let mut current_f_score = 0;
    let mut visited_nodes = 0;
    
    let mut cached_room_data = IndexedRoomDataCache::new(8, get_cost_matrix); // Cache up to 8 rooms

    // Initialize start node
    let start_h_score = heuristic(start, goal);
    for i in 0..start_h_score {
        open_by_f_score.push(Default::default());
    }
    
    let start_room_key = cached_room_data.get_room_key(start.room())?;
    open_by_f_score[start_h_score].push(State {
        g_score: 0,
        position: start,
        parent: start,
        open_direction: None,
        room_key: start_room_key,
    });

    while current_f_score < open_by_f_score.len() {
        // Find next non-empty bucket
        while current_f_score < open_by_f_score.len() && open_by_f_score[current_f_score].is_empty() {
            current_f_score += 1;
        }
        if current_f_score >= open_by_f_score.len() {
            break;
        }

        let current = open_by_f_score[current_f_score].pop().unwrap();
        visited_nodes += 1;
        if visited_nodes >= max_ops {
            return None;
        }

        let current_pos = current.position;
        let current_g_score = current.g_score;
        let current_open_direction = current.open_direction;
        let current_room_key = current.room_key;

        // Check if we've reached the goal
        if current_pos == goal {
            // Reconstruct path
            let mut path = Vec::new();
            let mut current_state = current;
            while current_state.position != start {
                path.push(current_state.position);
                // Find parent in the same room if possible
                if let Some(parent_state) = open_by_f_score.iter().flatten()
                    .find(|s| s.position == current_state.parent) {
                    current_state = *parent_state;
                } else {
                    return None;
                }
            }
            path.push(start);
            path.reverse();
            return Some(path);
        }

        // Explore neighbors
        for direction in next_directions(current_open_direction) {
            let Some(neighbor_pos) = current_pos.r#move(*direction) else { continue; };

            // Get or create room key for neighbor
            let neighbor_room_key = if neighbor_pos.room_name() == current_pos.room_name() {
                current_room_key
            } else {
                match cached_room_data.get_room_key(neighbor_pos.room()) {
                    Some(key) => key,
                    None => continue, // Room cache full, skip this neighbor
                }
            };

            // Get movement cost to neighbor
            let terrain_cost = if let Some(cost_matrix) = &cached_room_data[neighbor_room_key].cost_matrix {
                let x_coord = RoomCoordinate::new(neighbor_pos.local().x()).unwrap();
                let y_coord = RoomCoordinate::new(neighbor_pos.local().y()).unwrap();
                let xy = RoomXY::new(x_coord, y_coord);
                let cost = cost_matrix.get(xy);
                if cost >= 255 {
                    continue; // Impassable terrain
                }
                cost
            } else {
                continue; // Room is blocked
            };

            // Calculate scores for neighbor
            let neighbor_g_score = current_g_score.saturating_add(terrain_cost as usize);
            if neighbor_g_score >= max_path_length {
                continue;
            }

            // Check if this path to neighbor is better than any previous path
            let is_better_path = open_by_f_score.iter().flatten()
                .find(|s| s.position == neighbor_pos)
                .map_or(true, |existing| neighbor_g_score < existing.g_score);

            if !is_better_path {
                continue;
            }

            let neighbor_h_score = heuristic(neighbor_pos, goal);
            let neighbor_f_score = neighbor_g_score + neighbor_h_score;

            // Ensure we have enough buckets
            while open_by_f_score.len() <= neighbor_f_score {
                open_by_f_score.push(Default::default());
            }

            // Add neighbor to open set
            open_by_f_score[neighbor_f_score].push(State {
                g_score: neighbor_g_score,
                position: neighbor_pos,
                parent: current_pos,
                open_direction: Some(*direction),
                room_key: neighbor_room_key,
            });
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