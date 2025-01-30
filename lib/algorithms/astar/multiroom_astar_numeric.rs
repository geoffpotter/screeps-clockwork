use crate::datatypes::{
    ClockworkCostMatrix, CustomCostMatrix, LocalIndex, MultiroomGenericMap, MultiroomNumericMap,
    OptionalCache, PositionIndex, RoomIndex
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

fn next_directions(open_direction: &Option<Direction>) -> &'static [Direction] {
    &DIRECTION_LOOKUP[open_direction.map(|d| d as usize).unwrap_or(0)]
}

fn heuristic(position: PositionIndex, goal: PositionIndex) -> usize {
    position.distance_to(&goal) as usize
}

pub fn astar_path_numeric(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_ops: usize,
    max_path_length: usize,
) -> Option<Vec<PositionIndex>> {
    set_panic_hook();
    
    // Priority queue implemented as buckets of positions with same f_score
    let mut open_by_f_score: Vec<Vec<PositionIndex>> = vec![Default::default()];
    let mut current_f_score = 0;
    let mut visited_nodes = 0;

    // Create separate numeric maps for each variable
    let mut g_scores = MultiroomNumericMap::new();
    let mut h_scores = MultiroomNumericMap::new();
    let mut steps = MultiroomNumericMap::new();
    let mut parents = MultiroomGenericMap::new();
    let mut open_directions = MultiroomGenericMap::new();
    let cost_matrices = OptionalCache::new(|room: RoomName| get_cost_matrix(room));

    // Initialize start node
    let start_h_score = heuristic(start, goal);
    for i in 0..start_h_score {
        open_by_f_score.push(Default::default());
    }
    open_by_f_score[start_h_score].push(start);
    
    g_scores.set(start, 0);
    h_scores.set(start, start_h_score);
    steps.set(start, 0);
    parents.set(start, start);
    open_directions.set(start, None);

    let mut current_room = start.room_name();
    let mut current_room_cost_matrix = if let Some(cost_matrix) = cost_matrices.get_or_create(current_room) {
        cost_matrix
    } else {
        log(&format!("No cost matrix for room: {:?}", current_room));
        return None;
    };

    while current_f_score < open_by_f_score.len() {
        while current_f_score < open_by_f_score.len() && open_by_f_score[current_f_score].is_empty() {
            current_f_score += 1;
        }
        if current_f_score >= open_by_f_score.len() {
            break;
        }

        let current_pos = open_by_f_score[current_f_score].pop().unwrap();
        visited_nodes += 1;
        if visited_nodes >= max_ops {
            return None;
        }

        let current_g_score = g_scores.get(current_pos);
        let current_steps = steps.get(current_pos);
        let current_open_direction = open_directions.get(current_pos).unwrap_or(&None);

        if current_pos == goal {
            let mut path = Vec::new();
            let mut current = current_pos;
            
            while current != start {
                path.push(current);
                current = *parents.get(current).unwrap_or(&start);
                }
            
            path.reverse();
            return Some(path);
        }

        let current_f_score = current_g_score + h_scores.get(current_pos);
        if current_f_score > current_f_score {
            while open_by_f_score.len() <= current_f_score {
                open_by_f_score.push(Default::default());
            }
            open_by_f_score[current_f_score].push(current_pos);
            continue;
        }

        for direction in next_directions(current_open_direction) {
            let Some(neighbor_pos) = current_pos.r#move(*direction) else { continue; };

            if neighbor_pos.room_name() != current_room {
                let next_matrix = cost_matrices.get_or_create(neighbor_pos.room_name());
                if let Some(cost_matrix) = next_matrix {
                    current_room_cost_matrix = cost_matrix;
                    current_room = neighbor_pos.room_name();
            } else {
                    continue;
                }
            }

            let xy = neighbor_pos.local();
            let movement_cost = current_room_cost_matrix.get_local(xy);
            if movement_cost >= 255 {
                continue;
                }

            let neighbor_g_score = current_g_score.saturating_add(movement_cost as usize);
            if neighbor_g_score >= max_path_length {
                continue;
            }

            if neighbor_g_score >= g_scores.get(neighbor_pos) {
                continue;
            }

            let neighbor_h_score = heuristic(neighbor_pos, goal);
            let neighbor_f_score = neighbor_g_score + neighbor_h_score;
            
            while open_by_f_score.len() <= neighbor_f_score {
                open_by_f_score.push(Default::default());
            }

            g_scores.set(neighbor_pos, neighbor_g_score);
            h_scores.set(neighbor_pos, neighbor_h_score);
            steps.set(neighbor_pos, current_steps + 1);
            parents.set(neighbor_pos, current_pos);
            open_directions.set(neighbor_pos, Some(*direction));
            open_by_f_score[neighbor_f_score].push(neighbor_pos);
        }
    }

    None
}

#[wasm_bindgen]
pub fn js_astar_path_numeric(
    start_packed: u32,
    goal_packed: u32,
    get_cost_matrix: Function,
    max_ops: u32,
    max_path_length: u32,
) -> Option<Path> {
    let start = PositionIndex::from(Position::from_packed(start_packed));
    let goal = PositionIndex::from(Position::from_packed(goal_packed));

    let result = astar_path_numeric(
        start,
        goal,
        |room_name| get_cost_matrix_for_room(&get_cost_matrix, &room_name),
        max_ops as usize,
        max_path_length as usize,
    )?;

    let mut path = Path::new();
    for pos in result {
        path.add(Position::from(pos));
    }
    Some(path)
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

    if value.is_undefined() {
        None
    } else {
        match CustomCostMatrix::try_from(value.clone()) {
            Ok(matrix) => Some(matrix),
            Err(e) => {
                throw_val(JsValue::from_str(&format!("Invalid CustomCostMatrix: {:?}", e)))
            }
        }
    }
} 