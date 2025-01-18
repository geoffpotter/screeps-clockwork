use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const ROOM_SIZE: usize = 50;
const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;

pub struct SimpleHashMap {
    // Map from room name to room data
    rooms: HashMap<RoomName, Box<[usize; ROOM_AREA]>>,
}

impl MapTrait for SimpleHashMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let room_name = options.position.room_name();
        let x = options.position.x().u8() as usize;
        let y = options.position.y().u8() as usize;
        let index = y * ROOM_SIZE + x;
        
        let room = self.rooms.entry(room_name)
            .or_insert_with(|| Box::new([usize::MAX; ROOM_AREA]));
        room[index] = value;
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let room_name = options.position.room_name();
        let x = options.position.x().u8() as usize;
        let y = options.position.y().u8() as usize;
        let index = y * ROOM_SIZE + x;
        
        self.rooms.get(&room_name)
            .map(|room| room[index])
            .unwrap_or(usize::MAX)
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