use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::{fast_position, position::y_major_packed_position::YMajorPackedPosition};

const ROOM_SIZE: usize = 50;
const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;

pub struct YMajorPackedMap {
    rooms: HashMap<u16, Box<[usize; ROOM_AREA]>>,
    cached_room_key: Option<u16>,
    cached_room: Option<*mut Box<[usize; ROOM_AREA]>>,
}

impl YMajorPackedMap {
    fn update_cache(&mut self, room_key: u16) {
        if self.cached_room_key != Some(room_key) {
            self.cached_room_key = Some(room_key);
            self.cached_room = self.rooms.get_mut(&room_key)
                .map(|room| room as *mut _);
        }
    }
}

impl MapTrait for YMajorPackedMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            cached_room_key: None,
            cached_room: None,
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let room_key = options.y_major_packed_position.y_major_room();
        let local_key = options.y_major_packed_position.y_major_local() as usize;
        
        self.update_cache(room_key);
        
        if let Some(cached) = self.cached_room {
            unsafe {
                (*cached)[local_key] = value;
            }
        } else {
            let room = self.rooms.entry(room_key)
                .or_insert_with(|| Box::new([usize::MAX; ROOM_AREA]));
            room[local_key] = value;
            
            // Update cache after insertion
            self.cached_room = Some(room as *mut _);
            self.cached_room_key = Some(room_key);
        }
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let room_key = options.y_major_packed_position.y_major_room();
        let local_key = options.y_major_packed_position.y_major_local() as usize;
        
        self.update_cache(room_key);
        
        if let Some(cached) = self.cached_room {
            unsafe {
                (*cached)[local_key]
            }
        } else {
            usize::MAX
        }
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += self.rooms.len() * std::mem::size_of::<(u16, Box<[usize; ROOM_AREA]>)>();
        
        // Size of arrays in each room
        total += self.rooms.len() * std::mem::size_of::<[usize; ROOM_AREA]>();
        
        total
    }
}

// Safety: The raw pointer in cached_room is always valid when used
unsafe impl Send for YMajorPackedMap {}
unsafe impl Sync for YMajorPackedMap {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datatypes::position_map::test_position_map;

    #[test]
    fn test_y_major_packed_map() {
        let mut map = YMajorPackedMap::new();
        test_position_map(&mut map);
    }
} 