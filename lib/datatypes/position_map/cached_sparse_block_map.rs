use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const ROOM_SIZE: usize = 50;
const BLOCK_SIZE: usize = 10;
const BLOCKS_PER_SIDE: usize = ROOM_SIZE / BLOCK_SIZE;
const BLOCK_AREA: usize = BLOCK_SIZE * BLOCK_SIZE;

#[derive(Clone)]
struct Block {
    values: Box<[usize; BLOCK_AREA]>,
}

impl Block {
    fn new() -> Self {
        Self {
            values: Box::new([usize::MAX; BLOCK_AREA]),
        }
    }
}

pub struct CachedSparseBlockMap {
    rooms: HashMap<(i32, i32), HashMap<(usize, usize), Block>>,
    cached_room_coords: Option<(i32, i32)>,
    cached_room: Option<*mut HashMap<(usize, usize), Block>>,
    cached_block_coords: Option<(usize, usize)>,
    cached_block: Option<*mut Block>,
}

impl CachedSparseBlockMap {
    fn get_indices(pos: Position) -> (i32, i32, usize, usize, usize) {
        let packed = pos.packed_repr();
        let room_x = ((packed >> 24) as i8) as i32;
        let room_y = (((packed >> 16) & 0xFF) as i8) as i32;
        let local_x = ((packed >> 8) & 0xFF) as usize;
        let local_y = (packed & 0xFF) as usize;
        let block_x = local_x / BLOCK_SIZE;
        let block_y = local_y / BLOCK_SIZE;
        let block_local_x = local_x % BLOCK_SIZE;
        let block_local_y = local_y % BLOCK_SIZE;
        let block_index = block_local_y * BLOCK_SIZE + block_local_x;
        (room_x, room_y, block_x, block_y, block_index)
    }

    fn update_cache(&mut self, room_x: i32, room_y: i32, block_x: usize, block_y: usize) {
        if self.cached_room_coords != Some((room_x, room_y)) {
            self.cached_room_coords = Some((room_x, room_y));
            self.cached_room = self.rooms.get_mut(&(room_x, room_y))
                .map(|room| room as *mut _);
            self.cached_block_coords = None;
            self.cached_block = None;
        }
        
        if let Some(cached_room) = self.cached_room {
            if self.cached_block_coords != Some((block_x, block_y)) {
                self.cached_block_coords = Some((block_x, block_y));
                unsafe {
                    self.cached_block = (*cached_room).get_mut(&(block_x, block_y))
                        .map(|block| block as *mut _);
                }
            }
        }
    }
}

impl MapTrait for CachedSparseBlockMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            cached_room_coords: None,
            cached_room: None,
            cached_block_coords: None,
            cached_block: None,
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let (room_x, room_y, block_x, block_y, block_index) = Self::get_indices(options.position);
        self.update_cache(room_x, room_y, block_x, block_y);
        
        if let Some(cached_block) = self.cached_block {
            unsafe {
                (*cached_block).values[block_index] = value;
            }
        } else {
            let room = self.rooms.entry((room_x, room_y)).or_insert_with(HashMap::new);
            let block = room.entry((block_x, block_y)).or_insert_with(Block::new);
            block.values[block_index] = value;
            
            // Update cache after insertion
            self.cached_room = Some(room as *mut _);
            self.cached_block = Some(room.get_mut(&(block_x, block_y)).unwrap() as *mut _);
            self.cached_room_coords = Some((room_x, room_y));
            self.cached_block_coords = Some((block_x, block_y));
        }
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let (room_x, room_y, block_x, block_y, block_index) = Self::get_indices(options.position);
        self.update_cache(room_x, room_y, block_x, block_y);
        
        if let Some(cached_block) = self.cached_block {
            unsafe {
                (*cached_block).values[block_index]
            }
        } else {
            usize::MAX
        }
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += self.rooms.len() * std::mem::size_of::<((i32, i32), HashMap<(usize, usize), Block>)>();
        
        // Size of blocks in each room
        for room in self.rooms.values() {
            total += room.len() * (std::mem::size_of::<(usize, usize)>() + std::mem::size_of::<Block>());
        }
        
        total
    }
}

// Safety: The raw pointers in cached_room and cached_block are always valid when used
unsafe impl Send for CachedSparseBlockMap {}
unsafe impl Sync for CachedSparseBlockMap {} 