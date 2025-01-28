use crate::datatypes::{CustomCostMatrix, Path, PositionIndex};
use screeps::{Direction, Position, RoomName, RoomXY};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    f_score: u32,
    g_score: u32,
    position: PositionIndex,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[wasm_bindgen]
pub fn js_bidirectional_astar_path(
    start_packed: u32,
    goal_packed: u32,
    get_cost_matrix: &js_sys::Function,
    max_ops: u32,
    max_rooms: u32,
) -> Option<Path> {
    let start = Position::from_packed(start_packed);
    let goal = Position::from_packed(goal_packed);

    let start_idx = PositionIndex::from(start);
    let goal_idx = PositionIndex::from(goal);

    bidirectional_astar_path(
        start_idx,
        goal_idx,
        move |room_name: RoomName| {
            let js_room_name = JsValue::from_str(&room_name.to_string());
            let result = get_cost_matrix.call1(
                &JsValue::NULL,
                &js_room_name,
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
        },
        max_ops as usize,
        max_rooms as usize,
    )
    .map(|positions| {
        let positions: Vec<Position> = positions.into_iter().map(|p| p.into()).collect();
        Path::from(positions)
    })
}

fn bidirectional_astar_path(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_ops: usize,
    max_rooms: usize,
) -> Option<Vec<PositionIndex>> {
    let mut cost_matrices: HashMap<RoomName, CustomCostMatrix> = HashMap::new();
    let mut forward_open = BinaryHeap::new();
    let mut backward_open = BinaryHeap::new();
    let mut forward_closed = HashSet::new();
    let mut backward_closed = HashSet::new();
    let mut forward_came_from = HashMap::new();
    let mut backward_came_from = HashMap::new();
    let mut forward_g_scores = HashMap::new();
    let mut backward_g_scores = HashMap::new();
    let mut ops = 0;

    forward_open.push(State {
        g_score: 0,
        f_score: start.distance_to(&goal) as u32,
        position: start,
    });

    backward_open.push(State {
        g_score: 0,
        f_score: goal.distance_to(&start) as u32,
        position: goal,
    });

    forward_g_scores.insert(start, 0);
    backward_g_scores.insert(goal, 0);

    while !forward_open.is_empty() && !backward_open.is_empty() && ops < max_ops {
        ops += 1;

        // Forward search step
        if let Some(current) = forward_open.pop() {
            let pos = current.position;

            if backward_closed.contains(&pos) {
                return Some(reconstruct_path(
                    pos,
                    &forward_came_from,
                    &backward_came_from,
                ));
            }

            if forward_closed.contains(&pos) {
                continue;
            }

            forward_closed.insert(pos);

            let room = pos.room_name();
            let cost_matrix = match cost_matrices.entry(room) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.get().clone(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    if let Some(matrix) = get_cost_matrix(room) {
                        entry.insert(matrix.clone());
                        matrix
                    } else {
                        continue;
                    }
                }
            };

            let mut current_cost_matrix = &cost_matrix;
            
            // Get all valid neighbors
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
            
            for dir in directions.iter() {
                if let Some(next_pos) = pos.r#move(*dir) {
                    let next_room = next_pos.room_name();
                    if next_room != room {
                        if cost_matrices.len() >= max_rooms {
                            continue;
                        }
                        if !cost_matrices.contains_key(&next_room) {
                            if let Some(cost_matrix) = get_cost_matrix(next_room) {
                                cost_matrices.insert(next_room, cost_matrix);
                            } else {
                                continue;
                            }
                        }
                        current_cost_matrix = cost_matrices.get(&next_room).unwrap();
                    }

                    let cost = current_cost_matrix.get(RoomXY::new(next_pos.x(), next_pos.y()));
                    if cost == 255 {
                        continue;
                    }

                    let tentative_g_score = current.g_score + cost as u32;
                    if tentative_g_score < *forward_g_scores.get(&next_pos).unwrap_or(&u32::MAX) {
                        forward_came_from.insert(next_pos, pos);
                        forward_g_scores.insert(next_pos, tentative_g_score);
                        forward_open.push(State {
                            g_score: tentative_g_score,
                            f_score: tentative_g_score + next_pos.distance_to(&goal) as u32,
                            position: next_pos,
                        });
                    }
                }
            }
        }

        // Backward search step
        if let Some(current) = backward_open.pop() {
            let pos = current.position;

            if forward_closed.contains(&pos) {
                return Some(reconstruct_path(
                    pos,
                    &forward_came_from,
                    &backward_came_from,
                ));
            }

            if backward_closed.contains(&pos) {
                continue;
            }

            backward_closed.insert(pos);

            let room = pos.room_name();
            let cost_matrix = match cost_matrices.entry(room) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.get().clone(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    if let Some(matrix) = get_cost_matrix(room) {
                        entry.insert(matrix.clone());
                        matrix
                    } else {
                        continue;
                    }
                }
            };

            let mut current_cost_matrix = &cost_matrix;
            
            // Get all valid neighbors
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
            
            for dir in directions.iter() {
                if let Some(next_pos) = pos.r#move(*dir) {
                    let next_room = next_pos.room_name();
                    if next_room != room {
                        if cost_matrices.len() >= max_rooms {
                            continue;
                        }
                        if !cost_matrices.contains_key(&next_room) {
                            if let Some(cost_matrix) = get_cost_matrix(next_room) {
                                cost_matrices.insert(next_room, cost_matrix);
                            } else {
                                continue;
                            }
                        }
                        current_cost_matrix = cost_matrices.get(&next_room).unwrap();
                    }

                    let cost = current_cost_matrix.get(RoomXY::new(next_pos.x(), next_pos.y()));
                    if cost == 255 {
                        continue;
                    }

                    let tentative_g_score = current.g_score + cost as u32;
                    if tentative_g_score < *backward_g_scores.get(&next_pos).unwrap_or(&u32::MAX) {
                        backward_came_from.insert(next_pos, pos);
                        backward_g_scores.insert(next_pos, tentative_g_score);
                        backward_open.push(State {
                            g_score: tentative_g_score,
                            f_score: tentative_g_score + next_pos.distance_to(&start) as u32,
                            position: next_pos,
                        });
                    }
                }
            }
        }
    }

    None
}

fn reconstruct_path(
    meeting_point: PositionIndex,
    forward_came_from: &HashMap<PositionIndex, PositionIndex>,
    backward_came_from: &HashMap<PositionIndex, PositionIndex>,
) -> Vec<PositionIndex> {
    let mut path = Vec::new();

    // Reconstruct forward path
    let mut current = meeting_point;
    while let Some(&prev) = forward_came_from.get(&current) {
        path.push(current);
        if prev == current {
            break;
        }
        current = prev;
    }
    path.reverse();

    // Reconstruct backward path
    let mut current = meeting_point;
    while let Some(&next) = backward_came_from.get(&current) {
        if current != meeting_point {
            path.push(current);
        }
        if next == current {
            break;
        }
        current = next;
    }

    path
}
