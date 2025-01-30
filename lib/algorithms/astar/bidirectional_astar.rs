use crate::datatypes::{ClockworkCostMatrix, CustomCostMatrix, Path, PositionIndex, IndexedRoomDataCache, RoomIndex, MultiroomDistanceMap};
use crate::algorithms::map::{corresponding_room_edge, next_directions};
use screeps::{CircleStyle, Direction, LineStyle, Position, RoomName, RoomVisual, RoomXY};
use crate::log;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

const DEBUG_LOGGING: bool = false;
const DEBUG_VISUALIZE: bool = false;

const ALL_DIRECTIONS: [Direction; 8] = [
    Direction::Top,
    Direction::TopRight,
    Direction::Right,
    Direction::BottomRight,
    Direction::Bottom,
    Direction::BottomLeft,
    Direction::Left,
    Direction::TopLeft,
];

#[derive(Copy, Clone)]
struct State {
    // The cost to reach the current position
    g_score: u32,
    // The current position
    position: PositionIndex,
    // The index of the position's room in the room data cache
    room_key: usize,
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
    if DEBUG_LOGGING {
        log(&format!("Starting bidirectional A* from {:?} to {:?}", start, goal));
    }

    let mut cached_room_data = IndexedRoomDataCache::new(max_rooms, get_cost_matrix);
    
    // Vec-based open lists for forward and backward search
    let mut forward_open: Vec<Vec<State>> = vec![Default::default()];
    let mut backward_open: Vec<Vec<State>> = vec![Default::default()];
    let mut forward_min_idx = 0;
    let mut backward_min_idx = 0;
    
    let mut forward_closed = HashSet::new();
    let mut backward_closed = HashSet::new();
    let mut forward_came_from = HashMap::new();
    let mut backward_came_from = HashMap::new();
    let mut forward_distance_map = MultiroomDistanceMap::new();
    let mut backward_distance_map = MultiroomDistanceMap::new();
    let mut ops = 0;

    // Track best meeting point
    let mut best_meeting_point = None;
    let mut best_total_cost = u32::MAX;

    // Add a constant for the early exit threshold
    const EARLY_EXIT_MULTIPLIER: f32 = 1.0;

    // Initialize forward search
    if let Some(start_room_key) = cached_room_data.get_room_key(start.room()) {
        let h_score = start.distance_to(&goal) as u32;
        forward_open[0].push(State {
            g_score: 0,
            position: start,
            room_key: start_room_key,
        });
        forward_distance_map.set(start.into(), 0);
    }

    // Initialize backward search
    if let Some(goal_room_key) = cached_room_data.get_room_key(goal.room()) {
        let h_score = goal.distance_to(&start) as u32;
        backward_open[0].push(State {
            g_score: 0,
            position: goal,
            room_key: goal_room_key,
        });
        backward_distance_map.set(goal.into(), 0);
    }

    while !forward_open.is_empty() && !backward_open.is_empty() && ops < max_ops {
        ops += 1;

        // Forward search step
        while forward_min_idx < forward_open.len() && forward_open[forward_min_idx].is_empty() {
            forward_min_idx += 1;
        }
        
        // Backward search step
        while backward_min_idx < backward_open.len() && backward_open[backward_min_idx].is_empty() {
            backward_min_idx += 1;
        }

        // Check if either search is exhausted
        if forward_min_idx >= forward_open.len() || backward_min_idx >= backward_open.len() {
            if DEBUG_LOGGING {
                log(&format!("Search exhausted - forward_min_idx: {}, backward_min_idx: {}", 
                    forward_min_idx, backward_min_idx));
            }
            break;
        }

        // Early exit check - if we have a meeting point and both current minimum f-scores exceed it
        if let Some(meeting_point) = best_meeting_point {
            let min_forward_f = forward_min_idx as u32;
            let min_backward_f = backward_min_idx as u32;
            
            // If both minimum f-scores are greater than our best path, we can exit
            if min_forward_f >= best_total_cost && min_backward_f >= best_total_cost {
                if DEBUG_LOGGING {
                    log(&format!("Early exit - best cost: {}, min forward f: {}, min backward f: {}", 
                        best_total_cost, min_forward_f, min_backward_f));
                }
                break;
            }
        }

        // Alternate between forward and backward search
        if ops % 2 == 0 {
            if let Some(current) = forward_open[forward_min_idx].pop() {
                let pos = current.position;
                if DEBUG_VISUALIZE {
                    let viz = RoomVisual::new(Some(pos.room_name()));
                    viz.circle(
                        pos.x().u8() as f32,
                        pos.y().u8() as f32,
                        Some(CircleStyle::default().fill("red").opacity(0.5).radius(0.3)),
                    );
                }
                if DEBUG_LOGGING {
                    log(&format!("Processing forward position {:?} with g_score {}", pos, current.g_score));
                }

                if forward_closed.contains(&pos) {
                    if DEBUG_LOGGING {
                        log(&format!("Position {:?} already in forward closed set", pos));
                    }
                    continue;
                }

                // Early exit if this path is already worse than our best
                let f_score = current.g_score + pos.distance_to(&goal) as u32;
                if best_meeting_point.is_some() && f_score >= best_total_cost {
                    continue;
                }

                // Check if this is a better meeting point
                let backward_cost = backward_distance_map.get(pos.into());
                if backward_cost != usize::MAX {
                    let total_cost = current.g_score + backward_cost as u32;
                    if total_cost < best_total_cost {
                        best_meeting_point = Some(pos);
                        best_total_cost = total_cost;
                        
                        // Since we found a better path, we can skip any positions with higher f-scores
                        forward_open.truncate((total_cost + 1) as usize);
                        backward_open.truncate((total_cost + 1) as usize);
                    }
                }

                forward_closed.insert(pos);
                let current_room_name = cached_room_data[current.room_key].room_index;

                // Process all possible neighbors
                for direction in ALL_DIRECTIONS.iter() {
                    if let Some(next_pos) = pos.r#move(*direction) {
                        if forward_closed.contains(&next_pos) {
                            continue;
                        }

                        // Skip positions that are too far from the optimal path
                        let manhattan_to_goal = next_pos.distance_to(&goal);
                        if manhattan_to_goal as u32 + current.g_score > (best_total_cost as f32 * EARLY_EXIT_MULTIPLIER) as u32 {
                            if DEBUG_LOGGING {
                                log(&format!("Skipping forward position {:?} - too far from goal", next_pos));
                            }
                            continue;
                        }

                        if DEBUG_VISUALIZE {
                            let viz = RoomVisual::new(Some(next_pos.room_name()));
                            viz.circle(
                                next_pos.x().u8() as f32,
                                next_pos.y().u8() as f32,
                                Some(CircleStyle::default().fill("red").opacity(0.2).radius(0.15)),
                            );
                            viz.line(
                                (pos.local().x() as f32, pos.local().y() as f32),
                                (next_pos.local().x() as f32, next_pos.local().y() as f32),
                                Some(LineStyle::default().color("red").opacity(0.2).width(0.1)),
                            );
                        }

                        let next_room_name = next_pos.room();
                        let room_key = if next_room_name == current_room_name {
                            current.room_key
                        } else {
                            match cached_room_data.get_room_key(next_room_name) {
                                Some(key) => key,
                                None => continue,
                            }
                        };

                        let cost = match &cached_room_data[room_key].cost_matrix {
                            Some(matrix) => {
                                let cost = matrix.get(RoomXY::new(next_pos.x(), next_pos.y()));
                                if cost == 255 {
                                    if DEBUG_LOGGING {
                                        log(&format!("Position {:?} is blocked (cost 255)", next_pos));
                                    }
                                    continue;
                                }
                                cost
                            }
                            None => continue,
                        };

                        let next_g_score = current.g_score + cost as u32;
                        let current_best = forward_distance_map.get(next_pos.into());
                        
                        if next_g_score >= current_best as u32 {
                            continue;
                        }

                        let h_score = next_pos.distance_to(&goal) as u32;
                        let f_score = next_g_score + h_score;

                        if DEBUG_LOGGING {
                            log(&format!("Position {:?} - g: {}, h: {}, f: {}", 
                                next_pos, next_g_score, h_score, f_score));
                        }

                        forward_open.resize(forward_open.len().max(f_score as usize + 1), Default::default());
                        forward_distance_map.set(next_pos.into(), next_g_score as usize);
                        forward_came_from.insert(next_pos, pos);
                        forward_open[f_score as usize].push(State {
                            g_score: next_g_score,
                            position: next_pos,
                            room_key,
                        });

                        forward_min_idx = forward_min_idx.min(f_score as usize);
                    }
                }
            }
        } else {
            // Backward search step
            if let Some(current) = backward_open[backward_min_idx].pop() {
                let pos = current.position;
                if DEBUG_VISUALIZE {
                    let viz = RoomVisual::new(Some(pos.room_name()));
                    viz.circle(
                        pos.x().u8() as f32,
                        pos.y().u8() as f32,
                        Some(CircleStyle::default().fill("blue").opacity(0.5).radius(0.3)),
                    );
                }
                if DEBUG_LOGGING {
                    log(&format!("Processing backward position {:?} with g_score {}", pos, current.g_score));
                }

                if backward_closed.contains(&pos) {
                    if DEBUG_LOGGING {
                        log(&format!("Position {:?} already in backward closed set", pos));
                    }
                    continue;
                }

                // Early exit if this path is already worse than our best
                let f_score = current.g_score + pos.distance_to(&start) as u32;
                if best_meeting_point.is_some() && f_score >= best_total_cost {
                    continue;
                }

                // Check if this is a better meeting point
                let forward_cost = forward_distance_map.get(pos.into());
                if forward_cost != usize::MAX {
                    let total_cost = current.g_score + forward_cost as u32;
                    if total_cost < best_total_cost {
                        best_meeting_point = Some(pos);
                        best_total_cost = total_cost;
                        
                        // Since we found a better path, we can skip any positions with higher f-scores
                        forward_open.truncate((total_cost + 1) as usize);
                        backward_open.truncate((total_cost + 1) as usize);
                    }
                }

                backward_closed.insert(pos);
                let current_room_name = cached_room_data[current.room_key].room_index;

                // Process all possible neighbors
                for direction in ALL_DIRECTIONS.iter() {
                    if let Some(next_pos) = pos.r#move(*direction) {
                        if backward_closed.contains(&next_pos) {
                            continue;
                        }

                        // Skip positions that are too far from the optimal path
                        let manhattan_to_start = next_pos.distance_to(&start);
                        if manhattan_to_start as u32 + current.g_score > (best_total_cost as f32 * EARLY_EXIT_MULTIPLIER) as u32 {
                            if DEBUG_LOGGING {
                                log(&format!("Skipping backward position {:?} - too far from start", next_pos));
                            }
                            continue;
                        }

                        if DEBUG_VISUALIZE {
                            let viz = RoomVisual::new(Some(next_pos.room_name()));
                            viz.circle(
                                next_pos.x().u8() as f32,
                                next_pos.y().u8() as f32,
                                Some(CircleStyle::default().fill("blue").opacity(0.2).radius(0.15)),
                            );
                            viz.line(
                                (pos.local().x() as f32, pos.local().y() as f32),
                                (next_pos.local().x() as f32, next_pos.local().y() as f32),
                                Some(LineStyle::default().color("blue").opacity(0.2).width(0.1)),
                            );
                        }

                        let next_room_name = next_pos.room();
                        let room_key = if next_room_name == current_room_name {
                            current.room_key
                        } else {
                            match cached_room_data.get_room_key(next_room_name) {
                                Some(key) => key,
                                None => continue,
                            }
                        };

                        let cost = match &cached_room_data[room_key].cost_matrix {
                            Some(matrix) => {
                                let cost = matrix.get(RoomXY::new(next_pos.x(), next_pos.y()));
                                if cost == 255 {
                                    if DEBUG_LOGGING {
                                        log(&format!("Position {:?} is blocked (cost 255)", next_pos));
                                    }
                                    continue;
                                }
                                cost
                            }
                            None => continue,
                        };

                        let next_g_score = current.g_score + cost as u32;
                        let current_best = backward_distance_map.get(next_pos.into());
                        
                        if next_g_score >= current_best as u32 {
                            continue;
                        }

                        let h_score = next_pos.distance_to(&start) as u32;
                        let f_score = next_g_score + h_score;

                        if DEBUG_LOGGING {
                            log(&format!("Position {:?} - g: {}, h: {}, f: {}", 
                                next_pos, next_g_score, h_score, f_score));
                        }

                        backward_open.resize(backward_open.len().max(f_score as usize + 1), Default::default());
                        backward_distance_map.set(next_pos.into(), next_g_score as usize);
                        backward_came_from.insert(next_pos, pos);
                        backward_open[f_score as usize].push(State {
                            g_score: next_g_score,
                            position: next_pos,
                            room_key,
                        });

                        backward_min_idx = backward_min_idx.min(f_score as usize);
                    }
                }
            }
        }
    }

    if DEBUG_LOGGING {
        if let Some(meeting_point) = best_meeting_point {
            log(&format!("Found optimal meeting point at {:?} with total cost {}", meeting_point, best_total_cost));
        } else {
            log(&format!("Path not found after {} operations", ops));
        }
    }

    // // After finding the path, visualize it if we found one
    // if DEBUG_VISUALIZE {
    //     if let Some(meeting_point) = best_meeting_point {
    //         // Visualize the forward path
    //         let mut current = meeting_point;
    //         while let Some(&prev) = forward_came_from.get(&current) {
    //             let viz = RoomVisual::new(Some(current.room_name()));
    //             viz.line(
    //                 (current.local().x() as f32, current.local().y() as f32),
    //                 (prev.local().x() as f32, prev.local().y() as f32),
    //                 Some(LineStyle::default().color("yellow").opacity(0.8).width(0.15)),
    //             );
    //             if prev == current {
    //                 break;
    //             }
    //             current = prev;
    //         }

    //         // Visualize the backward path
    //         let mut current = meeting_point;
    //         while let Some(&next) = backward_came_from.get(&current) {
    //             let viz = RoomVisual::new(Some(current.room_name()));
    //             viz.line(
    //                 (current.local().x() as f32, current.local().y() as f32),
    //                 (next.local().x() as f32, next.local().y() as f32),
    //                 Some(LineStyle::default().color("yellow").opacity(0.8).width(0.15)),
    //             );
    //             if next == current {
    //                 break;
    //             }
    //             current = next;
    //         }
    //     }
    // }

    best_meeting_point.map(|meeting_point| reconstruct_path(
        meeting_point,
        &forward_came_from,
        &backward_came_from,
    ))
}

fn reconstruct_path(
    meeting_point: PositionIndex,
    forward_came_from: &HashMap<PositionIndex, PositionIndex>,
    backward_came_from: &HashMap<PositionIndex, PositionIndex>,
) -> Vec<PositionIndex> {
    if DEBUG_LOGGING {
        log(&format!("Reconstructing path from meeting point {:?}", meeting_point));
    }
    let mut path = Vec::new();
    let mut visited = HashSet::new();

    // Reconstruct forward path
    let mut current = meeting_point;
    visited.insert(current);
    while let Some(&prev) = forward_came_from.get(&current) {
        if DEBUG_LOGGING {
            log(&format!("Forward path node: {:?}", current));
        }
        path.push(current);
        if prev == current || visited.contains(&prev) {
            if DEBUG_LOGGING {
                log(&format!("Cycle detected in forward path at {:?}", current));
            }
            break;
        }
        visited.insert(prev);
        current = prev;
    }
    path.push(current);
    path.reverse();

    // Reconstruct backward path
    let mut current = meeting_point;
    while let Some(&next) = backward_came_from.get(&current) {
        if DEBUG_LOGGING {
            log(&format!("Backward path node: {:?}", current));
        }
        if next == current || visited.contains(&next) {
            if DEBUG_LOGGING {
                log(&format!("Cycle detected in backward path at {:?}", current));
            }
            break;
        }
        visited.insert(next);
        current = next;
        path.push(current);
    }

    if DEBUG_LOGGING {
        log(&format!("Final path length: {}", path.len()));
    }
    path
}
