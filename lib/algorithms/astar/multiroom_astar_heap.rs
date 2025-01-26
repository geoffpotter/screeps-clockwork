use crate::datatypes::{
    ClockworkCostMatrix, CustomCostMatrix, LocalIndex, MultiroomGenericMap, OptionalCache, PositionIndex, RoomIndex
};
use crate::log;
use crate::utils::set_panic_hook;
use lazy_static::lazy_static;
use screeps::{CircleStyle, Direction, RoomCoordinate, RoomName, RoomVisual, RoomXY};
use std::cmp::Ordering;
use std::collections::{HashMap, BinaryHeap};
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

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    f_score: usize,
    position: PositionIndex,
}

// For the priority queue - implement Ord to make it a min-heap based on f_score
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (lower f_score = higher priority)
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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

fn next_directions(open_direction: Option<Direction>) -> &'static [Direction] {
    &DIRECTION_LOOKUP[open_direction.map(|d| d as usize).unwrap_or(0)]
}

fn heuristic(position: PositionIndex, goal: PositionIndex) -> usize {
    // h_score: estimated cost from this position to the goal
    // using chebyshev distance as our heuristic
    position.distance_to(&goal) as usize
}

pub fn astar_path_heap(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_ops: usize,
    max_path_length: usize,
) -> Option<Vec<PositionIndex>> {
    set_panic_hook();
    
    // Priority queue using BinaryHeap
    let mut open_set = BinaryHeap::new();
    let mut visited_nodes = 0;
    let mut node_info = MultiroomGenericMap::<NodeInfo>::new();
    let cost_matrices = OptionalCache::new(|room: RoomName| get_cost_matrix(room));

    // Initialize start node
    let start_h_score = heuristic(start, goal);
    open_set.push(Node {
        f_score: start_h_score,
        position: start,
    });
    
    node_info.set(start, NodeInfo {
        g_score: 0,  // Cost from start to start is 0
        h_score: start_h_score,
        steps: 0,
        position: start,
        parent: start,
        open_direction: None,
    });

    let mut current_room = start.room_name();
    let mut current_room_cost_matrix = if let Some(cost_matrix) = cost_matrices.get_or_create(current_room) {
        cost_matrix
    } else {
        log(&format!("No cost matrix for room: {:?}", current_room));
        return None; // Cannot plan path without a cost matrix
    };

    while let Some(current_node) = open_set.pop() {
        let current_pos = current_node.position;
        visited_nodes += 1;
        if visited_nodes >= max_ops {
            return None;
        }

        let current_node = node_info.get_mut(current_pos).unwrap();
        let current_g_score = current_node.g_score;
        let current_steps = current_node.steps;
        let current_open_direction = current_node.open_direction;

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

        // Explore neighbors
        for direction in next_directions(current_open_direction) {
            let Some(neighbor_pos) = current_pos.r#move(*direction) else { continue; };

            // Update cost matrix if we've entered a new room
            if neighbor_pos.room_name() != current_room {
                let next_matrix = cost_matrices.get_or_create(neighbor_pos.room_name());
                if let Some(cost_matrix) = next_matrix {
                    current_room_cost_matrix = cost_matrix;
                    current_room = neighbor_pos.room_name();
                } else {
                    continue;
                }
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
            if let Some(existing_neighbor) = node_info.get(neighbor_pos) {
                if neighbor_g_score >= existing_neighbor.g_score {
                    continue; // This path to neighbor is not better than existing path
                }
            }

            let neighbor_h_score = heuristic(neighbor_pos, goal);
            let neighbor_f_score = neighbor_g_score + neighbor_h_score;

            // Add neighbor to open set
            node_info.set(neighbor_pos, NodeInfo {
                g_score: neighbor_g_score,
                h_score: neighbor_h_score,
                steps: current_steps + 1,
                position: neighbor_pos,
                parent: current_pos,
                open_direction: Some(*direction),
            });
            open_set.push(Node {
                f_score: neighbor_f_score,
                position: neighbor_pos,
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

    let cost_matrix = if value.is_undefined() {
        None
    } else {
        match CustomCostMatrix::try_from(value.clone()) {
            Ok(matrix) => Some(matrix),
            Err(e) => {
                throw_val(JsValue::from_str(&format!("Invalid CustomCostMatrix: {:?}", e)))
            }
        }
    };
    cost_matrix
}

#[wasm_bindgen]
pub fn js_astar_path_heap(
    start_packed: u32,
    goal_packed: u32,
    get_cost_matrix: Function,
    max_ops: u32,
    max_path_length: u32,
) -> Option<Path> {
    let start = PositionIndex::from(Position::from_packed(start_packed));
    let goal = PositionIndex::from(Position::from_packed(goal_packed));

    // let start_cpu = cpu::get_used();
    let result = astar_path_heap(
        start,
        goal,
        |room_name| {
            get_cost_matrix_for_room(&get_cost_matrix, &room_name)
        },
        max_ops as usize,
        max_path_length as usize,
    )?;

    // let end_cpu = cpu::get_used();
    // log(&format!("rust: A* Path (Heap) Cpu time: {:?}", end_cpu - start_cpu));
    let mut path = Path::new();
    for pos in result {
        path.add(Position::from(pos));
    }
    Some(path)
} 