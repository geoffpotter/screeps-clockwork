use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const ROOM_SIZE: usize = 50;

pub struct CachedMultiroomMap {
    rooms: HashMap<RoomName, Box<[usize; ROOM_SIZE * ROOM_SIZE]>>,
    cached_room_coords: Option<RoomName>,
    cached_room: Option<*mut [usize; ROOM_SIZE * ROOM_SIZE]>,
}

impl CachedMultiroomMap {
    fn get_index(x: u8, y: u8) -> usize {
        // Y-major indexing
        (y as usize) * ROOM_SIZE + (x as usize)
    }

    fn update_cache(&mut self, room_name: RoomName) {
        // Only update if it's a different room
        if self.cached_room_coords != Some(room_name) {
            self.cached_room_coords = Some(room_name);
            self.cached_room = self.rooms.get_mut(&room_name)
                .map(|room| room.as_mut() as *mut _);
        }
    }
}

impl MapTrait for CachedMultiroomMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            cached_room_coords: None,
            cached_room: None,
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let room_name = options.position.room_name();
        let x = options.position.x().u8();
        let y = options.position.y().u8();
        let index = Self::get_index(x, y);
        
        // Try to use cached room first
        self.update_cache(room_name);
        
        if let Some(cached) = self.cached_room {
            // Safety: We ensure the pointer is valid in update_cache
            unsafe {
                (*cached)[index] = value;
            }
        } else {
            // Cache miss, create new room and update cache
            let room = self.rooms.entry(room_name)
                .or_insert_with(|| Box::new([usize::MAX; ROOM_SIZE * ROOM_SIZE]));
            room[index] = value;
            
            // Update cache after inserting new room
            self.cached_room_coords = Some(room_name);
            self.cached_room = Some(room.as_mut() as *mut _);
        }
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let room_name = options.position.room_name();
        let x = options.position.x().u8();
        let y = options.position.y().u8();
        let index = Self::get_index(x, y);
        
        // Update cache just like in set
        self.update_cache(room_name);
        
        if let Some(cached) = self.cached_room {
            // Safety: We ensure the pointer is valid in update_cache
            unsafe {
                return (*cached)[index];
            }
        }
        
        // Cache miss, fall back to HashMap
        self.rooms.get(&room_name)
            .map(|room| room[index])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Add size of HashMap's internal allocations (capacity * (key + value + internal node data))
        let hash_map_capacity = self.rooms.capacity();
        total += hash_map_capacity * (std::mem::size_of::<RoomName>() + std::mem::size_of::<Box<[usize; ROOM_SIZE * ROOM_SIZE]>>());
        
        // Add size of each room's array including Box allocation
        total += self.rooms.len() * (
            std::mem::size_of::<[usize; ROOM_SIZE * ROOM_SIZE]>() // The array itself
        );
        
        total
    }
} 