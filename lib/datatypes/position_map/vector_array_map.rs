use std::collections::HashMap;
use screeps::{Position, RoomName};
use crate::datatypes::position_map::{GlobalPoint, MapTrait};

pub struct VectorArrayMap {
    rooms: Vec<Box<[usize; 2500]>>,
    room_indices: HashMap<RoomName, usize>,
    cached_room: Option<(RoomName, usize)>,
}

impl MapTrait for VectorArrayMap {
    fn new() -> Self {
        Self {
            rooms: Vec::new(),
            room_indices: HashMap::new(),
            cached_room: None,
        }
    }

    fn set(&mut self, _point: GlobalPoint, pos: Position, value: usize) {
        let room = pos.room_name();
        let room_idx = if let Some((cached_room, cached_idx)) = self.cached_room {
            if cached_room == room {
                cached_idx
            } else {
                let idx = match self.room_indices.get(&room) {
                    Some(&idx) => idx,
                    None => {
                        // Add new room to vector and store its index
                        let idx = self.rooms.len();
                        self.rooms.push(Box::new([usize::MAX; 2500]));
                        self.room_indices.insert(room, idx);
                        idx
                    }
                };
                self.cached_room = Some((room, idx));
                idx
            }
        } else {
            let idx = match self.room_indices.get(&room) {
                Some(&idx) => idx,
                None => {
                    // Add new room to vector and store its index
                    let idx = self.rooms.len();
                    self.rooms.push(Box::new([usize::MAX; 2500]));
                    self.room_indices.insert(room, idx);
                    idx
                }
            };
            self.cached_room = Some((room, idx));
            idx
        };
        
        let index = pos.y().u8() as usize * 50 + pos.x().u8() as usize;
        self.rooms[room_idx][index] = value;
    }

    fn get(&mut self, _point: GlobalPoint, pos: Position) -> usize {
        let room = pos.room_name();
        let room_idx = if let Some((cached_room, cached_idx)) = self.cached_room {
            if cached_room == room {
                cached_idx
            } else {
                match self.room_indices.get(&room) {
                    Some(&idx) => {
                        self.cached_room = Some((room, idx));
                        idx
                    }
                    None => return usize::MAX,
                }
            }
        } else {
            match self.room_indices.get(&room) {
                Some(&idx) => {
                    self.cached_room = Some((room, idx));
                    idx
                }
                None => return usize::MAX,
            }
        };

        let index = pos.y().u8() as usize * 50 + pos.x().u8() as usize;
        self.rooms[room_idx][index]
    }

    fn memory_usage(&self) -> usize {
        // Size of rooms vector (each room is 2500 * size_of::<usize>())
        let rooms_size = self.rooms.len() * 2500 * std::mem::size_of::<usize>();
        // Size of room indices HashMap (each entry is RoomName + usize)
        let indices_size = self.room_indices.len() * (std::mem::size_of::<RoomName>() + std::mem::size_of::<usize>());
        // Size of cached room (RoomName + usize when Some)
        let cache_size = if self.cached_room.is_some() {
            std::mem::size_of::<RoomName>() + std::mem::size_of::<usize>()
        } else {
            0
        };
        rooms_size + indices_size + cache_size
    }
} 