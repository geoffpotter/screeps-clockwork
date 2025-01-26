use screeps::{Position, RoomName, xy_to_linear_index};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use screeps::constants::extra::ROOM_AREA;
use super::{
    generic_map::GenericMap,
    position_index::PositionIndex,
    room_index::RoomIndex,
    local_index::LocalIndex,
};

/// Maps values of type T across multiple rooms, storing a GenericMap for each room
#[derive(Debug, Clone)]
pub struct MultiroomGenericMap<T: Copy> {
    maps: HashMap<RoomIndex, GenericMap<T>>,
    current_room: Option<RoomIndex>,
    current_room_map: Option<*mut GenericMap<T>>,
}

impl<T: Copy + Default> MultiroomGenericMap<T> {
    /// Creates a new empty multiroom generic map
    pub fn new() -> Self {
        MultiroomGenericMap {
            maps: HashMap::new(),
            current_room: None,
            current_room_map: None,
        }
    }

    /// Gets the value at a given position
    pub fn get(&self, pos: PositionIndex) -> Option<&T> {
        let room = pos.room();
        if Some(room) == self.current_room {
            // Use cached map reference if position is in current room
            unsafe {
                self.current_room_map
                    .map(|ptr| (*ptr).get(pos.local()))
                    .flatten()
            }
        } else {
            self.maps
                .get(&room)
                .and_then(|map| map.get(pos.local()))
        }
    }

    pub fn get_mut(&mut self, pos: PositionIndex) -> Option<&mut T> {
        let room = pos.room();
        if Some(room) == self.current_room {
            unsafe {
                self.current_room_map
                    .map(|ptr| (*ptr).get_mut(pos.local()))
                    .flatten()
            }
        } else {
            let map = self.maps.entry(room).or_insert_with(GenericMap::new);
            self.current_room = Some(room);
            self.current_room_map = Some(map as *mut GenericMap<T>);
            
            unsafe {
                self.current_room_map
                    .map(|ptr| (*ptr).get_mut(pos.local()))
                    .flatten()
            }
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
        
        let map = self.maps.entry(room).or_insert_with(GenericMap::new);
        map.set(pos.local(), value);
        
        // Update cache
        self.current_room = Some(room);
        self.current_room_map = Some(map as *mut GenericMap<T>);
    }

    /// Clears the value at a given position
    pub fn clear(&mut self, pos: PositionIndex) {
        let room = pos.room();
        if Some(room) == self.current_room {
            if let Some(ptr) = self.current_room_map {
                unsafe {
                    (*ptr).clear(pos.local());
                }
                return;
            }
        }
        
        if let Some(map) = self.maps.get_mut(&room) {
            map.clear(pos.local());
            // Update cache
            self.current_room = Some(room);
            self.current_room_map = Some(map as *mut GenericMap<T>);
        }
    }

    /// Returns whether the map contains data for a given room
    pub fn contains_room(&self, room: RoomIndex) -> bool {
        self.maps.contains_key(&room)
    }

    /// Gets a reference to the GenericMap for a given room, if it exists
    pub fn get_room_map(&self, room: RoomIndex) -> Option<&GenericMap<T>> {
        self.maps.get(&room)
    }

    /// Gets a mutable reference to the GenericMap for a given room, creating it if it doesn't exist
    pub fn get_or_create_room_map(&mut self, room: RoomIndex) -> &mut GenericMap<T> {
        self.maps.entry(room).or_insert_with(GenericMap::new)
    }

    /// Gets the list of rooms in the map
    pub fn rooms(&self) -> Vec<RoomIndex> {
        self.maps.keys().copied().collect()
    }

    /// Calculate approximate memory usage of this map
    pub fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        // Each room map has ROOM_AREA elements of type T
        total += self.maps.len() * std::mem::size_of::<GenericMap<T>>();
        total
    }
}

impl<T: Copy + Default> Default for MultiroomGenericMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

// JavaScript interface
#[wasm_bindgen]
pub struct JsMultiroomGenericMap {
    rooms: HashMap<RoomIndex, Vec<Option<JsValue>>>,
}

#[wasm_bindgen]
impl JsMultiroomGenericMap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    #[wasm_bindgen(js_name = get)]
    pub fn js_get(&self, room_x: u8, room_y: u8, x: u8, y: u8) -> JsValue {
        let room = RoomIndex::new(room_x, room_y);
        let local = LocalIndex::new(x, y);
        let pos = PositionIndex::new(room, local);
        self.rooms
            .get(&pos.room())
            .and_then(|room| room[pos.local().index() as usize].clone())
            .unwrap_or(JsValue::UNDEFINED)
    }

    #[wasm_bindgen(js_name = set)]
    pub fn js_set(&mut self, room_x: u8, room_y: u8, x: u8, y: u8, value: JsValue) {
        let room = RoomIndex::new(room_x, room_y);
        let local = LocalIndex::new(x, y);
        let pos = PositionIndex::new(room, local);
        let room_map = self.rooms
            .entry(pos.room())
            .or_insert_with(|| vec![None; ROOM_AREA]);
        room_map[pos.local().index() as usize] = Some(value);
    }

    #[wasm_bindgen(js_name = get_rooms)]
    pub fn js_get_rooms(&self) -> Vec<u16> {
        self.rooms.keys().map(|r| r.index() as u16).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::local_index::LocalIndex;

    #[test]
    fn test_basic_operations() {
        let mut map: MultiroomGenericMap<i32> = MultiroomGenericMap::new();
        let room = RoomIndex::new(128, 128); // E0N0
        let pos = PositionIndex::new(
            room,
            LocalIndex::new(25, 25),
        );

        // Test default value
        assert_eq!(map.get(pos), None);

        // Test setting and getting values
        map.set(pos, 42);
        assert_eq!(map.get(pos), Some(&42));

        map.clear(pos);
        assert_eq!(map.get(pos), None);
    }

    #[test]
    fn test_room_operations() {
        let mut map: MultiroomGenericMap<i32> = MultiroomGenericMap::new();
        let room = RoomIndex::new(128, 128); // E0N0
        let pos = PositionIndex::new(
            room,
            LocalIndex::new(25, 25),
        );

        map.set(pos, 42);

        assert!(map.contains_room(room));
        assert_eq!(map.rooms(), vec![room]);

        let room_map = map.get_room_map(room).unwrap();
        assert_eq!(room_map.get(pos.local()), Some(&42));
    }

    #[test]
    fn test_custom_type() {
        #[derive(Clone, Debug, PartialEq, Default, Copy)]
        struct TestStruct {
            value: i32,
        }

        let mut map: MultiroomGenericMap<TestStruct> = MultiroomGenericMap::new();
        let room = RoomIndex::new(128, 128); // E0N0
        let pos = PositionIndex::new(
            room,
            LocalIndex::new(25, 25),
        );

        let test_value = TestStruct { value: 42 };
        map.set(pos, test_value);

        assert_eq!(map.get(pos), Some(&test_value));
    }
}
