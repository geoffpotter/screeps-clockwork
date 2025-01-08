mod collections;
mod goal;
mod pathfinder;
mod room;
mod types;

pub use room::RoomInfo;
pub use goal::{Goal, PathfindingOptions};
pub use pathfinder::PathFinder;
use screeps::{game, Position};
pub use types::*;
use wasm_bindgen::{prelude::wasm_bindgen, throw_str};

use crate::log;

use screeps::{CircleStyle, Direction, LineStyle, RoomCoordinate, RoomName, RoomVisual, TextAlign, TextStyle, game::cpu};
use crate::{datatypes::{ClockworkCostMatrix, OptionalCache}, utils::PROFILER};
use super::{astar::cost_cache::{self, CostCache}, map::corresponding_room_edge};
use std::{borrow::Borrow, sync::Arc};

pub fn jump(
    current_position: Position,
    first_position: Position,
    direction: Direction,
    jump_cost: Cost,
    goals: &[Position],
    // cost_cache: &mut CostCache
) -> Option<Position> {
    let profiling_enabled = false;
    let profiler = &PROFILER;

    let cost_cache = CostCache::get_instance();
    let next_pos = current_position.checked_add_direction(direction).ok()?;
    let next_cost = cost_cache.look(WorldPosition::from(next_pos));
    if next_cost >= 255 {
        return None;
    }

    // Quick checks first
    if goals.contains(&next_pos) {
        return Some(next_pos);
    }

    if jump_cost != next_cost {
        return Some(next_pos);
    }

    // Room transition checks
    if profiling_enabled {
        profiler.start_call("jump::room_transition");
    }


    if current_position.x() == RoomCoordinate::new(0).unwrap() {
        if direction == Direction::Left || direction == Direction::TopLeft || direction == Direction::BottomLeft {
            if direction == Direction::Left {
                let is_first_position = next_pos.is_equal_to(first_position);
                if profiling_enabled {
                    profiler.end_call("jump::room_transition");
                }
                if is_first_position {
                    return Some(next_pos);
                } else {
                    return Some(current_position);
                }
            } else {
                if profiling_enabled {
                    profiler.end_call("jump::room_transition"); 
                }
                return None;
            }
        }
    }

    if current_position.x() == RoomCoordinate::new(49).unwrap() {
        if direction == Direction::Right || direction == Direction::TopRight || direction == Direction::BottomRight {
            if direction == Direction::Right {
                let is_first_position = next_pos.is_equal_to(first_position);
                if profiling_enabled {
                    profiler.end_call("jump::room_transition");
                }
                if is_first_position {
                    return Some(next_pos);
                } else {
                    return Some(current_position);
                }
            } else {
                if profiling_enabled {
                    profiler.end_call("jump::room_transition");
                }
                return None;
            }
        }
    }

    if current_position.y() == RoomCoordinate::new(0).unwrap() {
        if direction == Direction::Top || direction == Direction::TopLeft || direction == Direction::TopRight {
            if direction == Direction::Top {
                let is_first_position = next_pos.is_equal_to(first_position);
                if profiling_enabled {
                    profiler.end_call("jump::room_transition");
                }
                if is_first_position {
                    return Some(next_pos);
                } else {
                    return Some(current_position);
                }
            } else {
                if profiling_enabled {
                    profiler.end_call("jump::room_transition");
                }
                return None;
            }
        }
    }

    if current_position.y() == RoomCoordinate::new(49).unwrap() {
        if direction == Direction::Bottom || direction == Direction::BottomLeft || direction == Direction::BottomRight {
            if direction == Direction::Bottom {
                let is_first_position = next_pos.is_equal_to(first_position);
                if profiling_enabled {
                    profiler.end_call("jump::room_transition");
                }
                if is_first_position {
                    return Some(next_pos);
                } else {
                    return Some(current_position);
                }
            } else {
                if profiling_enabled {
                    profiler.end_call("jump::room_transition");
                }
                return None;
            }
        }
    }

    if profiling_enabled {
        profiler.end_call("jump::room_transition");
    }

    if profiling_enabled {
        profiler.start_call("jump::neighbor_checks");
    }
    // Diagonal movement
    if direction.is_diagonal() {
        let back_and_right = current_position
            .checked_add_direction(direction.multi_rot(3))
            // .map(corresponding_room_edge)
            .map(WorldPosition::from)
            .ok()?;
        let back_and_left = current_position
            .checked_add_direction(direction.multi_rot(-3))
            // .map(corresponding_room_edge)
            .map(WorldPosition::from)
            .ok()?;

        // Check for forced neighbors
        if 
            //back_and_left.room_name() != current_position.room_name() || 
            cost_cache.look(back_and_left) > jump_cost ||
            //back_and_right.room_name() != current_position.room_name() ||
            cost_cache.look(back_and_right) > jump_cost
        {
            if profiling_enabled {
                profiler.end_call("jump::neighbor_checks");
            }
            return Some(next_pos);
        }

        let dir_up_and_left = direction.multi_rot(1);
        let dir_up_and_right = direction.multi_rot(-1);
        
        if profiling_enabled {
            profiler.start_call("jump::diagonal_recursive");
        }
        let jump_up_and_left = jump(next_pos, current_position, dir_up_and_left, jump_cost, goals);
        let jump_up_and_right = jump(next_pos, current_position, dir_up_and_right, jump_cost, goals);
        if profiling_enabled {
            profiler.end_call("jump::diagonal_recursive");
        }

        if (jump_up_and_left.is_some() && !jump_up_and_left.unwrap().is_equal_to(current_position) && !jump_up_and_left.unwrap().is_equal_to(next_pos)) 
            || (jump_up_and_right.is_some() && !jump_up_and_right.unwrap().is_equal_to(current_position) && !jump_up_and_right.unwrap().is_equal_to(next_pos))
        {
            if profiling_enabled {
                profiler.end_call("jump::neighbor_checks");
            }
            return Some(next_pos);
        }
    } else {
        // Cardinal movement - check for forced neighbors
        let left = current_position
            .checked_add_direction(direction.multi_rot(-2))
            .map(corresponding_room_edge)
            .map(WorldPosition::from)
            .ok()?;
        let right = current_position
            .checked_add_direction(direction.multi_rot(2))
            .map(corresponding_room_edge)
            .map(WorldPosition::from)
            .ok()?;

        let left_cost = cost_cache.look(left); 
        let right_cost = cost_cache.look(right);

        let left_and_up = current_position
            .checked_add_direction(direction.multi_rot(-1))
            .map(corresponding_room_edge)
            .map(WorldPosition::from)
            .ok()?;
        let right_and_up = current_position
            .checked_add_direction(direction.multi_rot(1))
            .map(corresponding_room_edge)
            .map(WorldPosition::from)
            .ok()?;
        let left_and_up_cost = cost_cache.look(left_and_up);
        let right_and_up_cost = cost_cache.look(right_and_up);

        // special check for first position
        if next_pos.is_equal_to(first_position) {
            let left_and_back = current_position
                .checked_add_direction(direction.multi_rot(-3))
                .map(corresponding_room_edge)
                .map(WorldPosition::from)
                .ok()?;
            let right_and_back = current_position
                .checked_add_direction(direction.multi_rot(3))
                .map(corresponding_room_edge)
                .map(WorldPosition::from)
                .ok()?;
            let left_and_back_cost = cost_cache.look(left_and_back);
            let right_and_back_cost = cost_cache.look(right_and_back);
            if left_cost < 255 && (left_and_back_cost >= left_cost) {
                return Some(first_position);
            }
            if right_cost < 255 && (right_and_back_cost > right_cost) {
                return Some(first_position);
            }
        }

        if (left_and_up_cost < 255 && !(left_and_up_cost > left_cost)) ||
           (right_and_up_cost < 255 && !(right_and_up_cost >= right_cost))
        {
            if profiling_enabled {
                profiler.end_call("jump::neighbor_checks");
            }
            return Some(current_position);
        }
    }
    if profiling_enabled {
        profiler.end_call("jump::neighbor_checks");
        profiler.start_call("jump::recursive");
    }

    let ret = jump(next_pos, first_position, direction, jump_cost, goals);
    if profiling_enabled {
        profiler.end_call("jump::recursive");
    }
    ret
}



thread_local! {
    static PATHFINDER: std::cell::RefCell<PathFinder> = std::cell::RefCell::new(PathFinder::new());
}

#[wasm_bindgen]
pub fn js_pathfinder(origin: u32, goals: Vec<u32>) -> Vec<u32> {
    let start = game::cpu::get_used();
    PATHFINDER.with(|pf| {
        let mut pf = pf.borrow_mut();
        let origin = Position::from_packed(origin);
        let goals = goals
            .into_iter()
            .map(|g| {
                let pos = Position::from_packed(g);
                Goal::new(WorldPosition::from(pos), 0)
            })
            .collect();
        log(&format!("Rust Pathfinder setup: {}", game::cpu::get_used() - start).to_string());
        let start = game::cpu::get_used();
        let options = PathfindingOptions {
            plain_cost: 1,
            swamp_cost: 5,
            max_rooms: 100,
            flee: false,
            max_cost: 1500,
            max_ops: 50000,
            heuristic_weight: 1.0,
        };
        let result = pf.search(WorldPosition::from(origin), goals, options);
        log(&format!("Rust Pathfinder search: {}", game::cpu::get_used() - start).to_string());
        if let Ok(result) = result {
            log(&format!("Rust Pathfinder ops: {}", result.ops).to_string());
            log(&format!("Rust Pathfinder cost: {}", result.cost).to_string());
            log(&format!("Rust Pathfinder length: {}", result.path.len()).to_string());
            log(&format!("Rust Pathfinder incomplete: {}", result.incomplete).to_string());
            return result
                .path
                .into_iter()
                .map(|p| Position::from(p).packed_repr())
                .collect();
        } else if let Err(e) = result {
            throw_str(e);
        }
        vec![]
    })
}