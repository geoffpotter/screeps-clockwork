use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

pub struct FlatArrayMap {
    rooms: HashMap<RoomName, Box<[usize]>>,
}

impl MapTrait for FlatArrayMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let room_name = options.position.room_name();
        let x = options.position.x().u8() as usize;
        let y = options.position.y().u8() as usize;
        let index = y * 50 + x;

        let room = self.rooms.entry(room_name)
            .or_insert_with(|| {
                let mut room = Vec::with_capacity(2500);
                room.resize(2500, usize::MAX);
                room.into_boxed_slice()
            });
        room[index] = value;
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let room_name = options.position.room_name();
        let x = options.position.x().u8() as usize;
        let y = options.position.y().u8() as usize;
        let index = y * 50 + x;

        self.rooms.get(&room_name)
            .map(|room| room[index])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        // Size of HashMap overhead
        let base_size = std::mem::size_of::<HashMap<RoomName, Box<[usize]>>>();
        
        // Size of each room's array
        let room_size = 2500 * std::mem::size_of::<usize>();
        
        // Total size
        base_size + (self.rooms.len() * room_size)
    }
} 