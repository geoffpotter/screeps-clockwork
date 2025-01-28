use std::convert::TryFrom;

use crate::algorithms::astar::metrics::PathfindingMetrics;
use crate::algorithms::jps::jump;
use crate::algorithms::jps::WorldPosition;
use crate::algorithms::jps::OBSTACLE;
use crate::datatypes::ClockworkCostMatrix;
use crate::datatypes::MultiroomDistanceMap;
use crate::log;
use crate::utils::{set_panic_hook, PROFILER};
use crate::algorithms::astar::cost_cache::CostCache;
use lazy_static::lazy_static;
use screeps::CircleStyle;
use screeps::Direction;
use screeps::LineStyle;
use screeps::Position;
use screeps::RoomName;
use screeps::RoomVisual;
use screeps::TextAlign;
use screeps::TextStyle;
use wasm_bindgen::prelude::*;
use wasm_bindgen::throw_val;

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    g_score: usize,
    position: Position,
    open_direction: Option<Direction>,
}

// impl Ord for State {
//     fn cmp(&self, other: &Self) -> Ordering {
//         // if time() % 2 == 0 {
//         //     // old:
//             // other.f_score.cmp(&self.f_score)
//         // } else {
//             // new:
//             let f_score_cmp = other.f_score.cmp(&self.f_score);
//             if f_score_cmp == Ordering::Equal {
//                 // self.g_score.cmp(&other.g_score).reverse()
//                 self.g_score.cmp(&other.g_score)
//             } else {
//                 f_score_cmp
//             }
//         // }
//     }
// }

// impl PartialOrd for State {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }

fn heuristic(position: Position, goal: &[Position]) -> usize {
    goal.iter()
        .map(|g| position.get_range_to(*g))
        .min()
        .unwrap_or(0) as usize
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

/// Creates a distance map for the given start positions, using A* with jump-point search
/// to optimize the search and find the shortest path to the given destinations.
pub fn jps_multiroom_distance_map(
    start: Vec<Position>,
    get_cost_matrix: impl Fn(RoomName) -> Option<ClockworkCostMatrix>,
    max_ops: usize,
    goals: Vec<Position>,
) -> MultiroomDistanceMap {
    set_panic_hook();
    let profiler = &PROFILER;

    // Turn this on to see visuals in-game:
    const ENABLE_VISUALIZATION: bool = true;

    // Whether or not to profile sub-steps
    let profiling_enabled = false;
    if profiling_enabled {
        profiler.start_call("jps_multiroom_distance_map");
    }

    let mut open: Vec<Vec<State>> = vec![Default::default()];
    let mut min_idx = 0;
    let mut multiroom_distance_map = MultiroomDistanceMap::new();
    let cost_cache = CostCache::get_instance();
    let mut metrics = PathfindingMetrics::new();

    let start_room = start[0].room_name();

    // Initialization
    if profiling_enabled {
        profiler.start_call("initialization");
    }
    for position in start {
        if ENABLE_VISUALIZATION {
            // Mark initial positions with a small white circle
            let viz = RoomVisual::new(Some(position.room_name()));
            viz.circle(
                position.x().u8() as f32,
                position.y().u8() as f32,
                Some(CircleStyle::default().radius(0.3).stroke("white").fill("white")),
            );
        }

        open[0].push(State {
            g_score: 0,
            position,
            open_direction: None,
        });
        multiroom_distance_map.set(position, 0);
    }
    if profiling_enabled {
        profiler.end_call("initialization");
    }

    let mut current_room = start_room;
    let mut current_room_distance_map = multiroom_distance_map.get_or_create_room_map(current_room);

    while min_idx < open.len() {
        while let Some(State {
            g_score,
            position,
            open_direction,
        }) = open[min_idx].pop()
        {
            if profiling_enabled {
                profiler.start_call("Close Node");
            }

            

            // Once we pop from frontier, this node is "closed":
            if ENABLE_VISUALIZATION {
                // Draw a black circle to indicate a closed node
                let viz = RoomVisual::new(Some(position.room_name()));
                viz.circle(
                    position.x().u8() as f32,
                    position.y().u8() as f32,
                    Some(CircleStyle::default().fill("#000000").opacity(0.5).radius(0.3)),
                );
            }

            metrics.nodes_visited += 1;

            if metrics.nodes_visited >= max_ops {
                if profiling_enabled {
                    profiler.end_call("Close Node");
                }
                unsafe {
                    log(&format!("{:?}", metrics));
                }
                if profiling_enabled {
                    profiler.end_call("jps_multiroom_distance_map");
                    profiler.print_results();
                }
                return multiroom_distance_map;
            }

            // If we reached any goal, stop
            if goals.iter().any(|g| g == &position) {
                if profiling_enabled {
                    profiler.end_call("Close Node");
                }
                unsafe {
                    log(&format!("{:?}", metrics));
                }
                if profiling_enabled {
                    profiler.end_call("jps_multiroom_distance_map");
                    profiler.print_results();
                }
                return multiroom_distance_map;
            }

            // We try all useful directions from here
            for direction in next_directions(open_direction) {
                if profiling_enabled {
                    profiler.start_call("direction_processing");
                }

                metrics.neighbor_checks += 1;
                let first_step = match position.checked_add_direction(*direction) {
                    Ok(pos) => pos,
                    Err(_) => {
                        if profiling_enabled {
                            profiler.end_call("direction_processing");
                        }
                        log(&format!("failed direction_processing: {:?}", direction));
                        continue;
                    }
                };

                if first_step.room_name() != current_room {
                    current_room = first_step.room_name();
                    current_room_distance_map = multiroom_distance_map.get_or_create_room_map(current_room);
                }

                let first_step_cost = cost_cache.look(WorldPosition::from(first_step));
                if first_step_cost == OBSTACLE 
                    || current_room_distance_map[first_step.xy()] <= g_score.saturating_add(first_step_cost as usize) 
                    {
                    // Impassable
                    if profiling_enabled {
                        profiler.end_call("direction_processing");
                    }
                    continue;
                }

                if profiling_enabled {
                    profiler.start_call("jump");
                }
                if let Some(neighbor) =
                    jump(position, first_step, *direction, first_step_cost, goals.as_slice())
                {
                    // Check if the jumped-to position is a wall
                    if cost_cache.look(WorldPosition::from(neighbor)) == OBSTACLE {
                        if profiling_enabled {
                            profiler.end_call("jump");
                            profiler.end_call("direction_processing");
                        }
                        continue;
                    }

                    if profiling_enabled {
                        profiler.end_call("jump");
                        profiler.start_call("jump handling");
                    }
                    // If jump returns same position, skip
                    if neighbor.is_equal_to(position) {
                        if profiling_enabled {
                            profiler.end_call("jump handling");
                            profiler.end_call("direction_processing");
                        }
                        continue;
                    }

                    metrics.jump_attempts += 1;

                    // Visualize the jump line if you want
                    if ENABLE_VISUALIZATION {
                        let viz = RoomVisual::new(Some(position.room_name()));
                        viz.line(
                            (position.x().u8() as f32, position.y().u8() as f32),
                            (neighbor.x().u8() as f32, neighbor.y().u8() as f32),
                            Some(LineStyle::default().color("#ff0000").width(0.05)),
                        );
                        let terrain_cost = cost_cache.look(WorldPosition::from(neighbor));
                        viz.text(
                            neighbor.x().u8() as f32,
                            neighbor.y().u8() as f32,
                            format!("{}", terrain_cost),
                            Some(TextStyle::default().font(0.175).align(TextAlign::Left).background_padding(0.1)),
                        );
                        
                    }

                    // Interpolate along the path to neighbor
                    if profiling_enabled {
                        profiler.start_call("path_interpolation");
                    }
                    let mut step = position;
                    let mut jump_cost = g_score;
                    while let Ok(next_step) = step.checked_add_direction(*direction) {
                        if next_step.room_name() != current_room {
                            current_room = next_step.room_name();
                            current_room_distance_map =
                                multiroom_distance_map.get_or_create_room_map(current_room);
                        }
                        if next_step == neighbor {
                            break;
                        }
                        jump_cost = jump_cost.saturating_add(first_step_cost as usize);
                        current_room_distance_map[next_step.xy()] =
                            jump_cost.min(current_room_distance_map[next_step.xy()]);
                        step = next_step;
                    }
                    if profiling_enabled {
                        profiler.end_call("path_interpolation");
                        profiler.start_call("add_to_frontier");
                    }

                    let jump_range = position.get_range_to(neighbor);
                    metrics.max_jump_distance = metrics.max_jump_distance.max(jump_range as usize);
                    let terrain_cost = cost_cache.look(WorldPosition::from(neighbor));
                    if terrain_cost == OBSTACLE {
                        if profiling_enabled {
                            profiler.end_call("add_to_frontier");
                            profiler.end_call("jump handling");
                            profiler.end_call("direction_processing");
                        }
                        continue;
                    }

                    let next_cost = jump_cost.saturating_add(terrain_cost as usize);

                    if current_room_distance_map[neighbor.xy()] <= next_cost {
                        if profiling_enabled {
                            profiler.end_call("add_to_frontier");
                            profiler.end_call("jump handling");
                            profiler.end_call("direction_processing");
                        }
                        if ENABLE_VISUALIZATION {
                            let viz = RoomVisual::new(Some(position.room_name()));
                            viz.circle(
                                neighbor.x().u8() as f32,
                                neighbor.y().u8() as f32,
                                Some(CircleStyle::default().stroke("red").fill("transparent").opacity(0.5).radius(0.4)),
                            );
                        }
                        continue;
                    } else if current_room_distance_map[neighbor.xy()] < usize::MAX {
                        // log(&format!("neighbor: {:?} new cost: {} old cost: {}", neighbor, next_cost, current_room_distance_map[neighbor.xy()]));
                        // remove the old state from frontier
                        // frontier.retain(|state| !state.position.is_equal_to(neighbor));
                        if ENABLE_VISUALIZATION {
                            let viz = RoomVisual::new(Some(position.room_name()));
                            viz.circle(
                                neighbor.x().u8() as f32,
                                neighbor.y().u8() as f32,
                                Some(CircleStyle::default().stroke("green").fill("transparent").opacity(0.5).radius(0.4)),
                            );
                        }
                    }

                    // We push neighbor into the frontier => considered "open"
                    // if profiling_enabled {
                    // }

                    
                    let h_score = heuristic(neighbor, goals.as_slice());
                    let f_score = next_cost.saturating_add(h_score);


                    if ENABLE_VISUALIZATION {
                        let viz = RoomVisual::new(Some(position.room_name()));
                        viz.circle(
                            neighbor.x().u8() as f32,
                            neighbor.y().u8() as f32,
                            Some(CircleStyle::default().stroke("#ffffff").opacity(0.5).radius(0.3)),
                        );
                        // Place G bottom-left, H bottom-right, F top-right
                        let offset = 0.4;
                        viz.text(
                            neighbor.x().u8() as f32 - offset,
                            neighbor.y().u8() as f32 + offset,
                            format!("G:{}", next_cost),
                            Some(TextStyle::default().font(0.175).align(TextAlign::Left).background_padding(0.1)),
                        );
                        // viz.text(
                        //     neighbor.x().u8() as f32 + offset,
                        //     neighbor.y().u8() as f32 + offset,
                        //     format!(" H:{}", h_score),
                        //     Some(TextStyle::default().font(0.175).align(TextAlign::Right).background_padding(0.1)),
                        // );
                        viz.text(
                            neighbor.x().u8() as f32 + offset,
                            neighbor.y().u8() as f32 - offset,
                            format!(" F:{}", f_score),
                            Some(TextStyle::default().font(0.175).align(TextAlign::Right).background_padding(0.1)),
                        );
                    }
                    while open.len() <= f_score {
                        open.push(Default::default());
                    }
                    open[f_score].push(State {
                        g_score: next_cost,
                        position: neighbor,
                        open_direction: position.get_direction_to(neighbor),
                    });
                    current_room_distance_map[neighbor.xy()] = next_cost;
                    if profiling_enabled {
                        profiler.end_call("add_to_frontier");
                        profiler.end_call("jump handling");
                    }
                }
                if profiling_enabled {
                    profiler.end_call("direction_processing");
                }
            }

            if profiling_enabled {
                profiler.end_call("Close Node");
            }
        }
        min_idx += 1;
    }

    // No more frontier...
    log(&format!("{:?}", metrics));

    if profiling_enabled {
        profiler.end_call("jps_multiroom_distance_map");
        profiler.print_results();
    }
    multiroom_distance_map
}

#[wasm_bindgen]
pub fn js_jps_multiroom_distance_map(
    start_packed: Vec<u32>,
    get_cost_matrix: &js_sys::Function,
    max_ops: usize,
    destinations: Vec<u32>,
) -> MultiroomDistanceMap {
    





    let start_positions = start_packed
        .iter()
        .map(|pos| Position::from_packed(*pos))
        .collect();
    jps_multiroom_distance_map(
        start_positions,
        |room| {
            // let start_cpu = cpu::get_used();
            let result = get_cost_matrix.call1(
                &JsValue::null(),
                &JsValue::from_f64(room.packed_repr() as f64),
            );

            let value = match result {
                Ok(value) => value,
                Err(e) => throw_val(e),
            };

            let cost_matrix = if value.is_undefined() {
                None
            } else {
                // let end_cpu = cpu::get_used();
                // log(&format!("rust: CM Cpu time: {:?} {:?}", end_cpu - start_cpu, ClockworkCostMatrix::try_from(value.clone()).is_ok()));
                Some(
                    ClockworkCostMatrix::try_from(value)
                        .ok()
                        .expect_throw("Invalid ClockworkCostMatrix"),
                )
            };
            // let end_cpu = cpu::get_used();
            // log(&format!("rust: CM Cpu time: {:?}", end_cpu - start_cpu));
            cost_matrix
        },
        max_ops,
        destinations
            .iter()
            .map(|pos| Position::from_packed(*pos))
            .collect(),
    )
}
