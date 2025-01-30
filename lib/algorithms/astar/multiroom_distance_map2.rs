use crate::algorithms::astar::metrics::PathfindingMetrics;
use crate::algorithms::path::to_multiroom_distance_map_origin::path_to_multiroom_distance_map_origin;
use crate::datatypes::{
    CustomCostMatrix, JsMultiroomNumericMap, LocalIndex, MultiroomNumericMap, 
    PositionIndex, RoomIndex, IndexedRoomDataCache, Path
};
use crate::log;
use crate::utils::set_panic_hook;
use lazy_static::lazy_static;
use screeps::Direction;
use screeps::Position;
use screeps::RoomName;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::throw_val;

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    f_score: usize,
    g_score: usize,
    position: PositionIndex,
    open_direction: Option<Direction>,
    room_key: usize,
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

fn heuristic(position: PositionIndex, goals: &[PositionIndex]) -> usize {
    goals.iter()
        .map(|g| position.distance_to(g))
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

/// Creates a distance map for the given start positions, using A* to optimize the search and
/// find the shortest path to the given destinations.
pub fn astar_multiroom_distance_map2(
    start: Vec<PositionIndex>,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_tiles: usize,
    max_tile_distance: usize,
    goals: Vec<PositionIndex>,
) -> MultiroomNumericMap<usize> {
    set_panic_hook();
    let mut frontier = BinaryHeap::new();
    let mut visited = 0;
    let mut multiroom_distance_map = MultiroomNumericMap::new();
    let mut cached_room_data = IndexedRoomDataCache::new(8, get_cost_matrix);
    let mut metrics = PathfindingMetrics::new();

    let start_room = start[0].room();
    let start_room_key = match cached_room_data.get_room_key(start_room) {
        Some(key) => key,
        None => return multiroom_distance_map,
    };

    // Initialize with start positions
    for position in start {
        frontier.push(State {
            f_score: 0,
            g_score: 0,
            position,
            open_direction: None,
            room_key: start_room_key,
        });
        multiroom_distance_map.set(position, 0);
        visited += 1;
    }

    while let Some(State {
        f_score: _,
        g_score,
        position,
        open_direction,
        room_key: current_room_key,
    }) = frontier.pop() {
        metrics.nodes_visited += 1;

        if g_score >= max_tile_distance {
            continue;
        }

        for direction in next_directions(open_direction) {
            let Some(neighbor_pos) = position.r#move(*direction) else { continue; };
            metrics.neighbor_checks += 1;

            let neighbor_room_key = if neighbor_pos.room() == position.room() {
                current_room_key
            } else {
                match cached_room_data.get_room_key(neighbor_pos.room()) {
                    Some(key) => key,
                    None => continue,
                }
            };

            let terrain_cost = if let Some(cost_matrix) = &cached_room_data[neighbor_room_key].cost_matrix {
                let cost = cost_matrix.get_local(neighbor_pos.local());
                if cost >= 255 {
                    continue;
                }
                cost
            } else {
                continue;
            };

            let next_cost = g_score.saturating_add(terrain_cost as usize);

            if multiroom_distance_map.get(neighbor_pos) <= next_cost {
                continue;
            }

            let h_score = heuristic(neighbor_pos, &goals);
            let f_score = next_cost.saturating_add(h_score);
            frontier.push(State {
                f_score,
                g_score: next_cost,
                position: neighbor_pos,
                open_direction: Some(*direction),
                room_key: neighbor_room_key,
            });
            multiroom_distance_map.set(neighbor_pos, next_cost);
            visited += 1;

            if goals.contains(&neighbor_pos) || visited >= max_tiles {
                return multiroom_distance_map;
            }
        }
    }

    multiroom_distance_map
}

#[wasm_bindgen]
pub fn js_astar_multiroom_distance_map2(
    start_packed: Vec<u32>,
    get_cost_matrix: &js_sys::Function,
    max_tiles: usize,
    max_tile_distance: usize,
    destinations: Vec<u32>,
) -> JsMultiroomNumericMap {
    let start_positions = start_packed
        .iter()
        .map(|pos| PositionIndex::from(Position::from_packed(*pos)))
        .collect();
    let result = astar_multiroom_distance_map2(
        start_positions,
        |room| {
            let result = get_cost_matrix.call1(
                &JsValue::null(),
                &JsValue::from_f64(room.packed_repr() as f64),
            );

            let value = match result {
                Ok(value) => value,
                Err(e) => throw_val(e),
            };

            if value.is_undefined() {
                None
            } else {
                match CustomCostMatrix::try_from(value) {
                    Ok(matrix) => Some(matrix),
                    Err(e) => throw_val(JsValue::from_str(&format!("Invalid CustomCostMatrix: {:?}", e))),
                }
            }
        },
        max_tiles,
        max_tile_distance,
        destinations
            .iter()
            .map(|pos| PositionIndex::from(Position::from_packed(*pos)))
            .collect(),
    );
    JsMultiroomNumericMap { internal: result }
}

#[wasm_bindgen]
pub fn js_astar_multiroom_path2(
    start_packed: Vec<u32>,
    get_cost_matrix: &js_sys::Function,
    max_tiles: usize,
    max_tile_distance: usize,
    destinations: Vec<u32>,
) -> Path {
    let start_positions = start_packed
        .iter()
        .map(|pos| PositionIndex::from(Position::from_packed(*pos)))
        .collect();
    let result = astar_multiroom_distance_map2(
        start_positions,
        |room| {
            let result = get_cost_matrix.call1(
                &JsValue::null(),
                &JsValue::from_f64(room.packed_repr() as f64),
            );

            let value = match result {
                Ok(value) => value,
                Err(e) => throw_val(e),
            };

            if value.is_undefined() {
                None
            } else {
                match CustomCostMatrix::try_from(value) {
                    Ok(matrix) => Some(matrix),
                    Err(e) => throw_val(JsValue::from_str(&format!("Invalid CustomCostMatrix: {:?}", e))),
                }
            }
        },
        max_tiles,
        max_tile_distance,
        destinations
            .iter()
            .map(|pos| PositionIndex::from(Position::from_packed(*pos)))
            .collect(),
    );
    if let Ok(path) = path_to_multiroom_distance_map_origin(Position::from_packed(destinations[0]), &result) {
        path
    } else {
        Path::new()
    }
}
