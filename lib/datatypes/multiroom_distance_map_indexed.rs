
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use crate::datatypes::PositionIndex;

use super::{distance_map_indexed::DistanceMapIndexed, MultiroomDistanceMap, RoomIndex};

/// Maps distance values across multiple rooms, storing a DistanceMap for each room
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct MultiroomDistanceMapIndexed {
    #[wasm_bindgen(skip)]
    pub maps: HashMap<RoomIndex, DistanceMapIndexed>,
}

impl MultiroomDistanceMapIndexed {
    /// Creates a new empty multiroom distance map
    pub fn new() -> Self {
        MultiroomDistanceMapIndexed {
            maps: HashMap::new(),
        }
    }

    /// Gets the distance value at a given position
    pub fn get(&self, pos: PositionIndex) -> usize {
        self.maps
            .get(&pos.room())
            .map(|map| map[pos.local()])
            .unwrap_or(usize::MAX)
    }

    /// Sets the distance value at a given position
    pub fn set(&mut self, pos: PositionIndex, value: usize) {
        let map = self.maps.entry(pos.room()).or_insert_with(DistanceMapIndexed::new);
        map[pos.local()] = value;
    }

    /// Returns whether the map contains data for a given room
    pub fn contains_room(&self, room_index: RoomIndex) -> bool {
        self.maps.contains_key(&room_index)
    }

    /// Gets a reference to the DistanceMap for a given room, if it exists
    pub fn get_room_map(&self, room_index: RoomIndex) -> Option<&DistanceMapIndexed> {
        self.maps.get(&room_index)
    }

    /// Gets a mutable reference to the DistanceMap for a given room, creating it if it doesn't exist
    pub fn get_or_create_room_map(&mut self, room_index: RoomIndex) -> &mut DistanceMapIndexed {
        self.maps.entry(room_index).or_insert_with(DistanceMapIndexed::new)
    }

    /// Gets the list of rooms in the map
    pub fn rooms(&self) -> Vec<RoomIndex> {
        self.maps.keys().cloned().collect()
    }

    pub fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        total += self.maps.len() * std::mem::size_of::<DistanceMapIndexed>();
        total
    }
}

#[wasm_bindgen]
impl MultiroomDistanceMapIndexed {
    /// Creates a new empty multiroom distance map (JavaScript constructor)
    #[wasm_bindgen(constructor)]
    pub fn js_new() -> Self {
        Self::new()
    }

    /// Gets the distance value at a given position
    #[wasm_bindgen(js_name = get)]
    pub fn js_get(&self, packed_pos: u32) -> usize {
        let pos_index = PositionIndex::from(packed_pos);
        self.get(pos_index)
    }

    /// Sets the distance value at a given position
    #[wasm_bindgen(js_name = set)]
    pub fn js_set(&mut self, packed_pos: u32, value: usize) {
        let pos_index = PositionIndex::from(packed_pos);
        self.set(pos_index, value);
    }

    /// Gets the list of rooms in the map
    #[wasm_bindgen(js_name = get_rooms)]
    pub fn js_get_rooms(&self) -> Vec<u16> {
        self.rooms().iter().map(|r| r.index() as u16).collect()
    }

    /// Gets the DistanceMap for a given room
    #[wasm_bindgen(js_name = get_room)]
    pub fn js_get_room(&self, room_name: u16) -> Option<DistanceMapIndexed> {
        let room_index = RoomIndex::from(room_name);
        self.maps.get(&room_index).cloned()
    }
}

impl Default for MultiroomDistanceMapIndexed {
    fn default() -> Self {
        Self::new()
    }
}


