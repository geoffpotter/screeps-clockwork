use std::collections::HashMap;
use super::{GlobalPoint, MapTrait};
use screeps::Position;

const ROOM_SIZE: usize = 50;

pub struct DenseHashMap {
    // Direct value storage with usize::MAX for empty slots
    rooms: HashMap<(i32, i32), Box<[usize; ROOM_SIZE * ROOM_SIZE]>>,
}

impl DenseHashMap {
    fn get_indices(pos: Position) -> (i32, i32, usize) {
        let packed = pos.packed_repr();
        let room_x = ((packed >> 24) as i8) as i32;
        let room_y = (((packed >> 16) & 0xFF) as i8) as i32;
        let local_x = ((packed >> 8) & 0xFF) as usize;
        let local_y = (packed & 0xFF) as usize;
        
        let index = local_y * ROOM_SIZE + local_x;
        (room_x, room_y, index)
    }
}

impl MapTrait for DenseHashMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        let (room_x, room_y, index) = Self::get_indices(pos);
        let room = self.rooms.entry((room_x, room_y))
            .or_insert_with(|| Box::new([usize::MAX; ROOM_SIZE * ROOM_SIZE]));
        room[index] = value;
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        let (room_x, room_y, index) = Self::get_indices(pos);
        self.rooms.get(&(room_x, room_y))
            .map(|room| room[index])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<HashMap<(i32, i32), Box<[usize; ROOM_SIZE * ROOM_SIZE]>>>();
        
        // Add size of each room's array
        total += self.rooms.len() * ROOM_SIZE * ROOM_SIZE * std::mem::size_of::<usize>();
        
        total
    }
} 