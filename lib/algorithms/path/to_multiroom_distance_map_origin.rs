use crate::algorithms::map::corresponding_room_edge;
use crate::algorithms::map::neighbors_without_edges;
use crate::datatypes::MultiroomDistanceMap;
use crate::datatypes::MultiroomDistanceMapIndexed;
use crate::datatypes::MultiroomGenericMap;
use crate::datatypes::MultiroomNumericMap;
use crate::datatypes::MultiroomNumericMapUsize;
use crate::datatypes::Path;
use crate::datatypes::PositionIndex;
use crate::log;
use screeps::CircleStyle;
use screeps::Position;
use screeps::RoomVisual;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::throw_str;

// Maximum iterations to prevent infinite loops (50x50 room size)
const MAX_STEPS: usize = 2500;


pub trait MultiroomDistanceMapTrait {
    fn get(&self, pos: Position) -> usize;
}

impl MultiroomDistanceMapTrait for MultiroomDistanceMap {
    fn get(&self, pos: Position) -> usize {
        self.get(pos)
    }
}

impl MultiroomDistanceMapTrait for MultiroomGenericMap<usize> {
    fn get(&self, pos: Position) -> usize {
        if let Some(value) = self.get(PositionIndex::from(pos)) {
            *value
        } else {
            usize::MAX
        }
    }
}

impl MultiroomDistanceMapTrait for MultiroomNumericMap<usize> {
    fn get(&self, pos: Position) -> usize {
        self.get(PositionIndex::from(pos))
    }
}

impl MultiroomDistanceMapTrait for MultiroomDistanceMapIndexed {
    fn get(&self, pos: Position) -> usize {
        self.get(PositionIndex::from(pos))
    }
}

pub fn path_to_multiroom_distance_map_origin(
    start: Position,
    distance_map: &MultiroomDistanceMapTrait,
) -> Result<Path, &'static str> {
    let mut path = Path::new();
    let mut visited = HashSet::new();
    let mut current = start;
    let mut steps = 0;

    while steps < MAX_STEPS {
        // log(&format!("Step {:?}, current: {:?}", steps, current));
        path.add(current);

        let current_distance = distance_map.get(current);
        if current_distance == 0 {
            // log(&format!("Reached origin at {:?}", current));
            // We've reached the origin
            return Ok(path);
        }

        // Find the neighbor with the lowest distance value
        let mut next_pos = None;
        let mut min_distance = usize::MAX;

        for neighbor in neighbors_without_edges(current) {
            let neighbor_distance = distance_map.get(neighbor);

            if neighbor_distance < min_distance {
                min_distance = neighbor_distance;
                next_pos = Some(neighbor);
            }
        }

        // If no valid next position is found, return an error
        if let Some(next) = next_pos {
            if visited.contains(&next) {
                log(&format!("Cycle detected in distance map at {:?}", next));
                // log(&format!("Visited: {:?}", visited));
                return Err("Cycle detected in distance map");
            }

            // // if next is a room edge, jump to the corresponding room edge
            // if next.is_room_edge() {
            //     log(&format!("Jumping to room edge at {:?}", next));
            //     path.add(next);
            // }
            // current = corresponding_room_edge(next);
            current = next;
            visited.insert(current);
        } else {
            log(&format!("No valid path to origin found"));
            return Err("No valid path to origin found");
        }

        steps += 1;
    }
    // log(&format!("Path exceeded maximum length: {:?}, visited: {:?}", path.len(), visited.iter().map(|p| format!("{:?}, {:?}, {:?}", p.x(), p.y(), p.room_name().to_string())).collect::<Vec<String>>()));
    log(&format!("Path exceeded maximum length: {:?}", path.len()));
    Err("Path exceeded maximum length")
}

#[wasm_bindgen]
pub fn js_path_to_multiroom_distance_map_origin(
    start: u32,
    distance_map: &MultiroomDistanceMap,
) -> Path {
    match path_to_multiroom_distance_map_origin(Position::from_packed(start), distance_map) {
        Ok(path) => path,
        Err(e) => throw_str(&format!(
            "Error calculating path to multiroom distance map origin: {}",
            e
        )),
    }
}

#[wasm_bindgen]
pub fn js_path_to_multiroom_distance_map_origin_indexed(
    start: u32,
    distance_map: &MultiroomDistanceMapIndexed,
) -> Path {
    match path_to_multiroom_distance_map_origin(Position::from_packed(start), distance_map) {
        Ok(path) => path,
        Err(e) => throw_str(&format!(
            "Error calculating path to multiroom distance map origin: {}",
            e
        )),
    }
}
