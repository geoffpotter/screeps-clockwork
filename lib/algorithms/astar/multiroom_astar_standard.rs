use crate::datatypes::{
    ClockworkCostMatrix, CustomCostMatrix, LocalIndex, OptionalCache, PositionIndex, RoomIndex,
    MultiroomGenericMap
};
use crate::log;
use crate::utils::set_panic_hook;
use lazy_static::lazy_static;
use screeps::{CircleStyle, Direction, RoomCoordinate, RoomName, RoomVisual, RoomXY};
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::throw_val;
use js_sys::Function;
use screeps::Position;
use screeps::game::cpu;
use crate::datatypes::Path;

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    position: PositionIndex,
    parent: PositionIndex,
    f_score: usize,
    g_score: usize,
    h_score: usize,
    steps: usize,
    open_direction: Option<Direction>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            f_score: usize::MAX,
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

fn next_directions(open_direction: Option<Direction>) -> &'static [Direction] {
    &DIRECTION_LOOKUP[open_direction.map(|d| d as usize).unwrap_or(0)]
}

fn heuristic(position: PositionIndex, goal: PositionIndex) -> usize {
    position.distance_to(&goal) as usize
}

pub fn astar_path_standard(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_ops: usize,
    max_path_length: usize,
) -> Option<Vec<PositionIndex>> {
    set_panic_hook();
    
    // Priority queue implemented as buckets of nodes with same f_score
    let mut open_by_f_score: Vec<VecDeque<Node>> = vec![VecDeque::new()];
    let mut current_f_score = 0;
    let mut parent_map = MultiroomGenericMap::new();
    let mut g_score_map = MultiroomGenericMap::new();
    let mut visited_nodes = 0;
    let cost_matrices = OptionalCache::new(|room: RoomName| get_cost_matrix(room));

    // Initialize start node
    let start_h_score = heuristic(start, goal);
    let start_node = Node {
        f_score: start_h_score,
        g_score: 0,
        h_score: start_h_score,
        steps: 0,
        position: start,
        parent: start,
        open_direction: None,
    };
    
    // Initialize buckets up to start_h_score
    open_by_f_score = Vec::with_capacity(start_h_score + 1);
    open_by_f_score.resize_with(start_h_score + 1, VecDeque::new);
    open_by_f_score[start_h_score].push_back(start_node);

    // Initialize start position in maps
    parent_map.set(start, start);
    g_score_map.set(start, 0);

    let mut current_room = start.room_name();
    let mut current_room_cost_matrix = if let Some(cost_matrix) = cost_matrices.get_or_create(current_room) {
        cost_matrix
    } else {
        log(&format!("No cost matrix for room: {:?}", current_room));
        return None;
    };

    while current_f_score < open_by_f_score.len() {
        // Find next non-empty bucket
        while current_f_score < open_by_f_score.len() && open_by_f_score[current_f_score].is_empty() {
            current_f_score += 1;
        }
        if current_f_score >= open_by_f_score.len() {
            break;
        }

        let current_node = open_by_f_score[current_f_score].pop_front().unwrap();
        visited_nodes += 1;
        if visited_nodes >= max_ops {
            return None;
        }

        let current_pos = current_node.position;
        
        if current_pos == goal {
            let mut path = Vec::with_capacity(current_node.steps);
            let mut current = current_pos;
            
            while current != start {
                path.push(current);
                current = *parent_map.get(current).unwrap();
            }
            
            path.reverse();
            return Some(path);
        }

        // Check if this node's f_score is greater than current minimum
        if current_node.f_score > current_f_score {
            // Ensure we have enough buckets
            while open_by_f_score.len() <= current_node.f_score {
                open_by_f_score.push(VecDeque::new());
            }
            open_by_f_score[current_node.f_score].push_back(current_node);
            continue;
        }

        for direction in next_directions(current_node.open_direction) {
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

            let neighbor_g_score = current_node.g_score.saturating_add(movement_cost as usize);
            if neighbor_g_score >= max_path_length {
                continue;
            }

            // Skip if we've found a better path to this node
            if let Some(&existing_g_score) = g_score_map.get(neighbor_pos) {
                if neighbor_g_score >= existing_g_score {
                    continue;
                }
            }

            let neighbor_h_score = heuristic(neighbor_pos, goal);
            let neighbor_f_score = neighbor_g_score + neighbor_h_score;

            let neighbor_node = Node {
                f_score: neighbor_f_score,
                g_score: neighbor_g_score,
                h_score: neighbor_h_score,
                steps: current_node.steps + 1,
                position: neighbor_pos,
                parent: current_pos,
                open_direction: Some(*direction),
            };

            // Update maps with new best path
            parent_map.set(neighbor_pos, current_pos);
            g_score_map.set(neighbor_pos, neighbor_g_score);

            // Ensure we have enough buckets
            while open_by_f_score.len() <= neighbor_f_score {
                open_by_f_score.push(VecDeque::new());
            }
            open_by_f_score[neighbor_f_score].push_back(neighbor_node);
        }
    }

    None
}

#[wasm_bindgen]
pub fn js_astar_path_standard(
    start_packed: u32,
    goal_packed: u32,
    get_cost_matrix: Function,
    max_ops: u32,
    max_path_length: u32,
) -> Option<Path> {
    let start = PositionIndex::from(Position::from_packed(start_packed));
    let goal = PositionIndex::from(Position::from_packed(goal_packed));

    let result = astar_path_standard(
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