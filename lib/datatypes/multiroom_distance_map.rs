use screeps::{Position, RoomName};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use super::distance_map::DistanceMap;

/// Maps distance values across multiple rooms, storing a DistanceMap for each room
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct MultiroomDistanceMap {
    #[wasm_bindgen(skip)]
    pub maps: HashMap<RoomName, DistanceMap>,
}

impl MultiroomDistanceMap {
    /// Creates a new empty multiroom distance map
    pub fn new() -> Self {
        MultiroomDistanceMap {
            maps: HashMap::new(),
        }
    }

    /// Gets the distance value at a given position
    pub fn get(&self, pos: Position) -> usize {
        self.maps
            .get(&pos.room_name())
            .map(|map| map[pos.xy()])
            .unwrap_or(usize::MAX)
    }

    /// Sets the distance value at a given position
    pub fn set(&mut self, pos: Position, value: usize) {
        let room_name = pos.room_name();
        let map = self.maps.entry(room_name).or_insert_with(DistanceMap::new);
        map[pos.xy()] = value;
    }

    /// Returns whether the map contains data for a given room
    pub fn contains_room(&self, room_name: RoomName) -> bool {
        self.maps.contains_key(&room_name)
    }

    /// Gets a reference to the DistanceMap for a given room, if it exists
    pub fn get_room_map(&self, room_name: RoomName) -> Option<&DistanceMap> {
        self.maps.get(&room_name)
    }

    /// Gets a mutable reference to the DistanceMap for a given room, creating it if it doesn't exist
    pub fn get_or_create_room_map(&mut self, room_name: RoomName) -> &mut DistanceMap {
        self.maps.entry(room_name).or_insert_with(DistanceMap::new)
    }

    /// Gets the list of rooms in the map
    pub fn rooms(&self) -> Vec<RoomName> {
        self.maps.keys().cloned().collect()
    }
}

#[wasm_bindgen]
impl MultiroomDistanceMap {
    /// Creates a new empty multiroom distance map (JavaScript constructor)
    #[wasm_bindgen(constructor)]
    pub fn js_new() -> Self {
        Self::new()
    }

    /// Gets the distance value at a given position
    #[wasm_bindgen(js_name = get)]
    pub fn js_get(&self, packed_pos: u32) -> usize {
        let pos = Position::from_packed(packed_pos);
        self.get(pos)
    }

    /// Sets the distance value at a given position
    #[wasm_bindgen(js_name = set)]
    pub fn js_set(&mut self, packed_pos: u32, value: usize) {
        let pos = Position::from_packed(packed_pos);
        self.set(pos, value);
    }

    /// Gets the list of rooms in the map
    #[wasm_bindgen(js_name = get_rooms)]
    pub fn js_get_rooms(&self) -> Vec<u16> {
        self.rooms().iter().map(|r| r.packed_repr()).collect()
    }

    /// Gets the DistanceMap for a given room
    #[wasm_bindgen(js_name = get_room)]
    pub fn js_get_room(&self, room_name: u16) -> Option<DistanceMap> {
        let room_name = RoomName::from_packed(room_name);
        self.maps.get(&room_name).cloned()
    }
}

impl Default for MultiroomDistanceMap {
    fn default() -> Self {
        Self::new()
    }
}
