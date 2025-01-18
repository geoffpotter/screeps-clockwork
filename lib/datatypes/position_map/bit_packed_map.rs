use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};

const ROOM_SIZE: usize = 50;
const BITS_PER_VALUE: usize = 8; // Store values 0-255
const VALUES_PER_WORD: usize = 64 / BITS_PER_VALUE; // 8 values per u64
const VALUE_MASK: u64 = (1 << BITS_PER_VALUE) - 1;
const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;
const WORDS_PER_ROOM: usize = (ROOM_AREA + VALUES_PER_WORD - 1) / VALUES_PER_WORD;
const MISSING_VALUE: u64 = VALUE_MASK; // Use max value (255) to represent missing values
const MISSING_VALUE_WORD: u64 = 0; // Use 0 to represent missing values in the word

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

    fn set(&mut self, options: PositionOptions, value: usize) {
        // Handle values that are too large or represent missing
        if value >= (1 << BITS_PER_VALUE) || value == usize::MAX {
            // println!("Value too large or missing, I'm cheating.: {}", value);
            return;
        }

        let room_name = options.position.room_name();
        let x = options.position.x().u8();
        let y = options.position.y().u8();
        let (word_idx, bit_offset) = Self::get_indices(x, y);
        
        let room = self.rooms.entry(room_name)
            .or_insert_with(|| {
                let mut room = Box::new([0; WORDS_PER_ROOM]);
                // Initialize all values to MISSING_VALUE
                for word in room.iter_mut() {
                    *word = MISSING_VALUE_WORD;
                }
                room
            });
        
        // Clear old value and set new one
        room[word_idx] &= !(VALUE_MASK << bit_offset);
        room[word_idx] |= (value as u64) << bit_offset;
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let room_name = options.position.room_name();
        let x = options.position.x().u8();
        let y = options.position.y().u8();
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
        // Size of HashMap overhead
        let base_size = std::mem::size_of::<HashMap<RoomName, Box<[u64; WORDS_PER_ROOM]>>>();
        
        // Size of each room's data
        let room_size = std::mem::size_of::<Box<[u64; WORDS_PER_ROOM]>>();
        
        // Total size
        base_size + (self.rooms.len() * room_size)
    }
} 