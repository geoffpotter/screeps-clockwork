use std::collections::HashMap;
use super::{GlobalPoint, MapTrait};
use screeps::{Position, RoomName};

const ROOM_SIZE: usize = 50;
const BITS_PER_VALUE: usize = 8; // Store values 0-255
const VALUES_PER_WORD: usize = 64 / BITS_PER_VALUE; // 8 values per u64
const VALUE_MASK: u64 = (1 << BITS_PER_VALUE) - 1;
const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;
const WORDS_PER_ROOM: usize = (ROOM_AREA + VALUES_PER_WORD - 1) / VALUES_PER_WORD;
const MISSING_VALUE: u64 = VALUE_MASK; // Use max value (255) to represent missing values

pub struct BitPackedMap {
    // Each u64 stores multiple values
    rooms: HashMap<RoomName, Box<[u64; WORDS_PER_ROOM]>>,
}

impl BitPackedMap {
    fn get_indices(x: u8, y: u8) -> (usize, usize) {
        // Y-major indexing
        let pos_index = (y as usize) * ROOM_SIZE + (x as usize);
        let word_index = pos_index / VALUES_PER_WORD;
        let value_offset = (pos_index % VALUES_PER_WORD) * BITS_PER_VALUE;
        (word_index, value_offset)
    }
}

impl MapTrait for BitPackedMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        // Handle values that are too large or represent missing
        if value >= (1 << BITS_PER_VALUE) || value == usize::MAX {
            return;
        }

        let room_name = pos.room_name();
        let x = pos.x().u8();
        let y = pos.y().u8();
        let (word_idx, bit_offset) = Self::get_indices(x, y);
        
        let room = self.rooms.entry(room_name)
            .or_insert_with(|| {
                let mut room = Box::new([0; WORDS_PER_ROOM]);
                // Initialize all values to MISSING_VALUE
                for word in room.iter_mut() {
                    for i in 0..VALUES_PER_WORD {
                        *word |= MISSING_VALUE << (i * BITS_PER_VALUE);
                    }
                }
                room
            });
        
        // Clear old value and set new one
        room[word_idx] &= !(VALUE_MASK << bit_offset);
        room[word_idx] |= (value as u64) << bit_offset;
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        let room_name = pos.room_name();
        let x = pos.x().u8();
        let y = pos.y().u8();
        let (word_idx, bit_offset) = Self::get_indices(x, y);
        
        self.rooms.get(&room_name)
            .map(|room| {
                let value = (room[word_idx] >> bit_offset) & VALUE_MASK;
                if value == MISSING_VALUE {
                    usize::MAX
                } else {
                    value as usize
                }
            })
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += std::mem::size_of::<HashMap<RoomName, Box<[u64; WORDS_PER_ROOM]>>>();
        
        // Size of arrays in each room
        total += self.rooms.len() * WORDS_PER_ROOM * std::mem::size_of::<u64>();
        
        total
    }
} 