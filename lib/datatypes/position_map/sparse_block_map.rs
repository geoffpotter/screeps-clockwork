use std::collections::HashMap;
use super::{GlobalPoint, MapTrait};
use screeps::Position;

const BLOCK_SIZE: usize = 8;  // 8x8 blocks

pub struct SparseBlockMap {
    // Map from room coordinates to room data
    rooms: HashMap<(i32, i32), RoomBlocks>,
}

struct RoomBlocks {
    // Sparse storage of blocks - only allocated when used
    blocks: HashMap<(usize, usize), Block>,
}

enum Block {
    // For blocks with few values
    Sparse(HashMap<(u8, u8), usize>),
    // For blocks with many values
    Dense([[usize; BLOCK_SIZE]; BLOCK_SIZE]),
}

impl Block {
    fn new_sparse() -> Self {
        Block::Sparse(HashMap::new())
    }

    fn new_dense() -> Self {
        Block::Dense([[usize::MAX; BLOCK_SIZE]; BLOCK_SIZE])
    }

    fn get(&self, x: u8, y: u8) -> usize {
        match self {
            Block::Sparse(map) => map.get(&(x, y)).copied().unwrap_or(usize::MAX),
            Block::Dense(array) => array[y as usize][x as usize],
        }
    }

    fn set(&mut self, x: u8, y: u8, value: usize) {
        match self {
            Block::Sparse(map) => {
                map.insert((x, y), value);
                // Convert to dense if too many values
                if map.len() > (BLOCK_SIZE * BLOCK_SIZE) / 2 {
                    let mut dense = [[usize::MAX; BLOCK_SIZE]; BLOCK_SIZE];
                    for ((x, y), v) in map.iter() {
                        dense[*y as usize][*x as usize] = *v;
                    }
                    *self = Block::Dense(dense);
                }
            }
            Block::Dense(array) => {
                array[y as usize][x as usize] = value;
            }
        }
    }

    fn memory_usage(&self) -> usize {
        match self {
            Block::Sparse(map) => {
                std::mem::size_of::<HashMap<(u8, u8), usize>>() +
                map.capacity() * std::mem::size_of::<((u8, u8), usize)>()
            }
            Block::Dense(_) => {
                std::mem::size_of::<[[usize; BLOCK_SIZE]; BLOCK_SIZE]>()
            }
        }
    }
}

impl Default for RoomBlocks {
    fn default() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }
}

impl SparseBlockMap {
    fn get_indices(pos: Position) -> ((i32, i32), (usize, usize), (u8, u8)) {
        let packed = pos.packed_repr();
        let room_x = (packed >> 24) as i32;
        let room_y = ((packed >> 16) & 0xFF) as i32;
        let local_x = ((packed >> 8) & 0xFF) as usize;
        let local_y = (packed & 0xFF) as usize;
        
        let block_x = local_x / BLOCK_SIZE;
        let block_y = local_y / BLOCK_SIZE;
        
        let pos_x = (local_x % BLOCK_SIZE) as u8;
        let pos_y = (local_y % BLOCK_SIZE) as u8;
        
        ((room_x, room_y), (block_x, block_y), (pos_x, pos_y))
    }
}

impl MapTrait for SparseBlockMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        let ((room_x, room_y), (block_x, block_y), (pos_x, pos_y)) = Self::get_indices(pos);
        
        let room = self.rooms.entry((room_x, room_y)).or_insert_with(RoomBlocks::default);
        let block = room.blocks.entry((block_x, block_y)).or_insert_with(Block::new_sparse);
        
        block.set(pos_x, pos_y, value);
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        let ((room_x, room_y), (block_x, block_y), (pos_x, pos_y)) = Self::get_indices(pos);
        
        self.rooms.get(&(room_x, room_y))
            .and_then(|room| room.blocks.get(&(block_x, block_y)))
            .map(|block| block.get(pos_x, pos_y))
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = 0;
        
        // Size of room hashmap
        total += std::mem::size_of::<HashMap<(i32, i32), RoomBlocks>>();
        
        // Size of each room's blocks
        for room in self.rooms.values() {
            total += std::mem::size_of::<RoomBlocks>();
            total += room.blocks.capacity() * std::mem::size_of::<((usize, usize), Block)>();
            
            // Size of each block
            for block in room.blocks.values() {
                total += block.memory_usage();
            }
        }
        
        total
    }
} 