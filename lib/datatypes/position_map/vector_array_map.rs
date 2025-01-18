use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;
use crate::datatypes::position::fast_position;

pub struct VectorArrayMap {
    rooms: HashMap<u16, Vec<usize>>,
}

impl MapTrait for VectorArrayMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let room_key = options.y_major_packed_position.y_major_room();
        let index = options.y_major_packed_position.y_major_local() as usize;

        let room = self.rooms.entry(room_key)
            .or_insert_with(|| {
                let mut room = Vec::with_capacity(2500);
                room.resize(2500, usize::MAX);
                room
            });
        room[index] = value;
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let room_key = options.y_major_packed_position.y_major_room();
        let index = options.y_major_packed_position.y_major_local() as usize;

        self.rooms.get(&room_key)
            .map(|room| room[index])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        // Size of HashMap
        let map_size = std::mem::size_of::<HashMap<u16, Vec<usize>>>() + 
            self.rooms.len() * (std::mem::size_of::<u16>() + std::mem::size_of::<Vec<usize>>());
        
        map_size
    }
} 