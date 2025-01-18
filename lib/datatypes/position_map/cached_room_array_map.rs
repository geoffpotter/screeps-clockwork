use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const ROOM_SIZE: usize = 50;
const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;

pub struct CachedRoomArrayMap {
    rooms: HashMap<RoomName, Box<[usize; ROOM_AREA]>>,
    cached_room_coords: Option<RoomName>,
    cached_room: Option<*mut Box<[usize; ROOM_AREA]>>,
}

impl CachedRoomArrayMap {
    fn get_indices(pos: Position) -> (RoomName, usize) {
        let room_name = pos.room_name();
        let local_x = pos.x().u8() as usize;
        let local_y = pos.y().u8() as usize;
        let index = local_y * ROOM_SIZE + local_x;
        (room_name, index)
    }

    fn update_cache(&mut self, room_name: RoomName) {
        if self.cached_room_coords != Some(room_name) {
            self.cached_room_coords = Some(room_name);
            self.cached_room = self.rooms.get_mut(&room_name)
                .map(|room| room as *mut _);
        }
    }
}

impl MapTrait for CachedRoomArrayMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            cached_room_coords: None,
            cached_room: None,
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let (room_name, index) = Self::get_indices(options.position);
        self.update_cache(room_name);
        
        if let Some(cached) = self.cached_room {
            unsafe {
                (*cached)[index] = value;
            }
        } else {
            let room = self.rooms.entry(room_name)
                .or_insert_with(|| Box::new([usize::MAX; ROOM_AREA]));
            room[index] = value;
            
            // Update cache after insertion
            self.cached_room = Some(room as *mut _);
            self.cached_room_coords = Some(room_name);
        }
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let (room_name, index) = Self::get_indices(options.position);
        self.update_cache(room_name);
        
        if let Some(cached) = self.cached_room {
            unsafe {
                (*cached)[index]
            }
        } else {
            usize::MAX
        }
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += self.rooms.len() * std::mem::size_of::<(RoomName, Box<[usize; ROOM_AREA]>)>();
        
        // Size of arrays in each room
        total += self.rooms.len() * std::mem::size_of::<[usize; ROOM_AREA]>();
        
        total
    }
}

// Safety: The raw pointer in cached_room is always valid when used
unsafe impl Send for CachedRoomArrayMap {}
unsafe impl Sync for CachedRoomArrayMap {} 