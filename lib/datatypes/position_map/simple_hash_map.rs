use std::collections::HashMap;
use super::{GlobalPoint, MapTrait};
use screeps::{Position, RoomName};

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

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        let room_name = pos.room_name();
        let local_x = pos.x().u8() as usize;
        let local_y = pos.y().u8() as usize;
        let index = local_y * ROOM_SIZE + local_x;
        
        let room = self.rooms.entry(room_name)
            .or_insert_with(|| Box::new([usize::MAX; ROOM_AREA]));
        room[index] = value;
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        let room_name = pos.room_name();
        let local_x = pos.x().u8() as usize;
        let local_y = pos.y().u8() as usize;
        let index = local_y * ROOM_SIZE + local_x;
        
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