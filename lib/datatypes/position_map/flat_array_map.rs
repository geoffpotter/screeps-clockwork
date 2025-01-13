use std::collections::HashMap;
use super::{GlobalPoint, MapTrait};
use screeps::{Position, RoomName};

const ROOM_SIZE: usize = 50;
const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;

pub struct FlatArrayMap {
    // Map from room name to flat array
    rooms: HashMap<RoomName, Box<[usize; ROOM_AREA]>>,
}

impl FlatArrayMap {
    fn get_index(x: u8, y: u8) -> usize {
        // Y-major indexing
        (y as usize) * ROOM_SIZE + (x as usize)
    }
}

impl MapTrait for FlatArrayMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        let room_name = pos.room_name();
        let x = pos.x().u8();
        let y = pos.y().u8();
        let index = Self::get_index(x, y);
        
        let room = self.rooms.entry(room_name)
            .or_insert_with(|| Box::new([usize::MAX; ROOM_AREA]));
        room[index] = value;
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        let room_name = pos.room_name();
        let x = pos.x().u8();
        let y = pos.y().u8();
        let index = Self::get_index(x, y);
        
        self.rooms.get(&room_name)
            .map(|room| room[index])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += std::mem::size_of::<HashMap<RoomName, Box<[usize; ROOM_AREA]>>>();
        
        // Size of arrays in each room
        total += self.rooms.len() * ROOM_AREA * std::mem::size_of::<usize>();
        
        total
    }
} 