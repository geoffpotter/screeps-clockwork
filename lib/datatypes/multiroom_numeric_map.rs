use screeps::{Position, RoomName};
use std::collections::HashMap;
use num_traits::Bounded;
use std::ops::{Add, Sub};
use wasm_bindgen::prelude::*;

use super::{
    numeric_map::{NumericMap, JsNumericMap},
    position_index::PositionIndex,
    room_index::RoomIndex,
    local_index::LocalIndex,
};

/// Maps numeric values across multiple rooms, storing a NumericMap for each room
#[derive(Debug, Clone)]
pub struct MultiroomNumericMap<T>
where
    T: Copy + Bounded + Add<Output = T> + Sub<Output = T> + PartialEq
{
    pub maps: HashMap<RoomIndex, NumericMap<T>>,
    pub current_room: Option<RoomIndex>,
    pub current_room_map: Option<*mut NumericMap<T>>,
}

impl<T> MultiroomNumericMap<T>
where
    T: Copy + Bounded + Add<Output = T> + Sub<Output = T> + PartialEq
{
    /// Creates a new empty multiroom numeric map
    pub fn new() -> Self {
        MultiroomNumericMap {
            maps: HashMap::new(),
            current_room: None,
            current_room_map: None,
        }
    }

    /// Gets the value at a given position
    pub fn get(&self, pos: PositionIndex) -> T {
        let room = pos.room();
        if Some(room) == self.current_room {
            // Use cached map reference if position is in current room
            unsafe {
                self.current_room_map
                    .map(|ptr| (*ptr).get(pos.local()))
                    .unwrap_or_else(T::max_value)
            }
        } else {
            self.maps
                .get(&room)
                .map(|map| map.get(pos.local()))
                .unwrap_or_else(T::max_value)
        }
    }

    /// Sets the value at a given position
    pub fn set(&mut self, pos: PositionIndex, value: T) {
        let room = pos.room();
        if Some(room) == self.current_room {
            // Use cached map reference if position is in current room
            if let Some(ptr) = self.current_room_map {
                unsafe {
                    (*ptr).set(pos.local(), value);
                }
                return;
            }
        }
        
        let map = self.maps.entry(room).or_insert_with(NumericMap::new);
        map.set(pos.local(), value);
        
        // Update cache
        self.current_room = Some(room);
        self.current_room_map = Some(map as *mut NumericMap<T>);
    }

    /// Returns whether the map contains data for a given room
    pub fn contains_room(&self, room: RoomIndex) -> bool {
        self.maps.contains_key(&room)
    }

    /// Gets a reference to the NumericMap for a given room, if it exists
    pub fn get_room_map(&self, room: RoomIndex) -> Option<&NumericMap<T>> {
        self.maps.get(&room)
    }

    /// Gets a mutable reference to the NumericMap for a given room, creating it if it doesn't exist
    pub fn get_or_create_room_map(&mut self, room: RoomIndex) -> &mut NumericMap<T> {
        self.maps.entry(room).or_insert_with(NumericMap::new)
    }

    /// Gets the list of rooms in the map
    pub fn rooms(&self) -> Vec<RoomIndex> {
        self.maps.keys().copied().collect()
    }

    /// Calculate approximate memory usage of this map
    pub fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        // Each room map has ROOM_AREA elements of type T
        total += self.maps.len() * std::mem::size_of::<NumericMap<T>>();
        total
    }
}

/// JavaScript interface for MultiroomNumericMap<usize>
#[wasm_bindgen]
pub struct JsMultiroomNumericMap {
    pub(crate) internal: MultiroomNumericMap<usize>,
}

#[wasm_bindgen]
impl JsMultiroomNumericMap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            internal: MultiroomNumericMap::new(),
        }
    }

    #[wasm_bindgen(js_name = get)]
    pub fn js_get(&self, packed_pos: u32) -> usize {
        let pos = Position::from_packed(packed_pos);
        let pos_index = PositionIndex::from(pos);
        self.internal.get(pos_index)
    }

    #[wasm_bindgen(js_name = set)]
    pub fn js_set(&mut self, packed_pos: u32, value: usize) {
        let pos = Position::from_packed(packed_pos);
        let pos_index = PositionIndex::from(pos);
        self.internal.set(pos_index, value);
    }

    #[wasm_bindgen(js_name = get_rooms)]
    pub fn js_get_rooms(&self) -> Vec<u16> {
        self.internal.maps.keys().map(|r| r.index() as u16).collect()
    }

    #[wasm_bindgen(js_name = get_room)]
    pub fn js_get_room(&self, room_index: u16) -> Option<JsNumericMap> {
        let room = RoomIndex::from_index(room_index.into());
        self.internal.maps.get(&room).map(|map| JsNumericMap {
            internal: map.clone(),
        })
    }
}

impl Default for JsMultiroomNumericMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use screeps::RoomCoordinate;

    #[test]
    fn test_basic_operations() {
        let mut map: MultiroomNumericMap<usize> = MultiroomNumericMap::new();
        let room = RoomIndex::new(128, 128); // E0N0
        let pos = PositionIndex::new(
            room,
            LocalIndex::new(25, 25),
        );

        // Test default value
        assert_eq!(map.get(pos), usize::MAX);

        // Test setting and getting values
        map.set(pos, 42);
        assert_eq!(map.get(pos), 42);
    }

    #[test]
    fn test_room_operations() {
        let mut map: MultiroomNumericMap<usize> = MultiroomNumericMap::new();
        let room = RoomIndex::new(128, 128); // E0N0
        let pos = PositionIndex::new(
            room,
            LocalIndex::new(25, 25),
        );

        map.set(pos, 42);

        assert!(map.contains_room(room));
        assert_eq!(map.rooms(), vec![room]);

        let room_map = map.get_room_map(room).unwrap();
        assert_eq!(room_map.get(pos.local()), 42);
    }

    #[test]
    fn test_multiple_rooms() {
        let mut map: MultiroomNumericMap<usize> = MultiroomNumericMap::new();
        let rooms = [
            RoomIndex::new(128, 128), // E0N0
            RoomIndex::new(129, 128), // E1N0
            RoomIndex::new(128, 129), // E0N1
        ];
        
        // Test setting values in different rooms
        for (i, &room) in rooms.iter().enumerate() {
            let pos = PositionIndex::new(
                room,
                LocalIndex::new(25, 25),
            );
            map.set(pos, i);
        }

        // Verify values and room existence
        for (i, &room) in rooms.iter().enumerate() {
            let pos = PositionIndex::new(
                room,
                LocalIndex::new(25, 25),
            );
            assert_eq!(map.get(pos), i);
            assert!(map.contains_room(room));
        }

        // Test non-existent room
        let non_existent = RoomIndex::new(127, 128); // W1N0
        let pos = PositionIndex::new(
            non_existent,
            LocalIndex::new(25, 25),
        );
        assert_eq!(map.get(pos), usize::MAX);
        assert!(!map.contains_room(non_existent));

        // Test room listing
        let mut room_list = map.rooms();
        room_list.sort_by_key(|r| r.index());
        let mut expected = rooms.to_vec();
        expected.sort_by_key(|r| r.index());
        assert_eq!(room_list, expected);
    }

    #[test]
    fn test_edge_coordinates() {
        let mut map: MultiroomNumericMap<usize> = MultiroomNumericMap::new();
        let room = RoomIndex::new(128, 128); // E0N0
        
        // Test corners
        let corners = [
            LocalIndex::new(0, 0),
            LocalIndex::new(0, 49),
            LocalIndex::new(49, 0),
            LocalIndex::new(49, 49),
        ];

        for (i, corner) in corners.iter().enumerate() {
            let pos = PositionIndex::new(room, *corner);
            map.set(pos, i);
            assert_eq!(map.get(pos), i);
        }
    }

    #[test]
    fn test_memory_usage() {
        let mut map: MultiroomNumericMap<usize> = MultiroomNumericMap::new();
        
        // Empty map
        let initial_size = map.memory_usage();
        assert!(initial_size > 0);
        
        // Add a room
        let room = RoomIndex::new(128, 128); // E0N0
        let pos = PositionIndex::new(
            room,
            LocalIndex::new(0, 0),
        );
        map.set(pos, 42);
        
        // Size should increase by at least one NumericMap
        assert!(map.memory_usage() > initial_size);
        assert_eq!(
            map.memory_usage(),
            initial_size + std::mem::size_of::<NumericMap<usize>>()
        );
    }
}


pub struct MultiroomNumericMapUsize(MultiroomNumericMap<usize>);
