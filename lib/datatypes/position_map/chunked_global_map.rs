use std::{collections::HashMap, array};
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::{GlobalPosition, fast_position};

const CHUNK_SIZE: usize = 64; // Power of 2 for efficient division
const CHUNK_SHIFT: usize = 6; // log2(CHUNK_SIZE)
const CHUNK_MASK: usize = CHUNK_SIZE - 1;
const CACHE_SIZE: usize = 4; // Small cache for recently accessed chunks

pub struct ChunkedGlobalMap {
    chunks: HashMap<(i32, i32), Box<[[usize; CHUNK_SIZE]; CHUNK_SIZE]>>,
    // Cache stores chunk coordinates for recently accessed chunks
    cache_keys: [(i32, i32); CACHE_SIZE],
    cache_index: usize,
}

impl ChunkedGlobalMap {
    #[inline]
    fn get_chunk_coords(pos: &GlobalPosition) -> (i32, i32, usize, usize) {
        // Extract global x,y directly from packed representation
        let global_x = pos.x as i32;
        let global_y = pos.y as i32;
        
        // Calculate chunk coordinates using bit shifts
        let chunk_x = global_x >> CHUNK_SHIFT;
        let chunk_y = global_y >> CHUNK_SHIFT;
        
        // Calculate local coordinates using mask
        let local_x = (global_x & CHUNK_MASK as i32) as usize;
        let local_y = (global_y & CHUNK_MASK as i32) as usize;
        
        (chunk_x, chunk_y, local_x, local_y)
    }

    #[inline]
    fn get_chunk(&self, chunk_x: i32, chunk_y: i32) -> Option<&Box<[[usize; CHUNK_SIZE]; CHUNK_SIZE]>> {
        // Check cache first
        for &(cx, cy) in &self.cache_keys {
            if cx == chunk_x && cy == chunk_y {
                return self.chunks.get(&(chunk_x, chunk_y));
            }
        }
        self.chunks.get(&(chunk_x, chunk_y))
    }

    #[inline]
    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_y: i32) -> &mut Box<[[usize; CHUNK_SIZE]; CHUNK_SIZE]> {
        // Update cache before modifying
        self.cache_keys[self.cache_index] = (chunk_x, chunk_y);
        self.cache_index = (self.cache_index + 1) % CACHE_SIZE;
        
        // Get or create the chunk
        self.chunks.entry((chunk_x, chunk_y))
            .or_insert_with(|| Box::new([[usize::MAX; CHUNK_SIZE]; CHUNK_SIZE]))
    }
}

impl MapTrait for ChunkedGlobalMap {
    fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            cache_keys: [(0, 0); CACHE_SIZE],
            cache_index: 0,
        }
    }

    #[inline]
    fn set(&mut self, options: PositionOptions, value: usize) {
        let (chunk_x, chunk_y, local_x, local_y) = Self::get_chunk_coords(&options.global_position);
        let chunk = self.get_chunk_mut(chunk_x, chunk_y);
        chunk[local_y][local_x] = value;
    }

    #[inline]
    fn get(&mut self, options: PositionOptions) -> usize {
        let (chunk_x, chunk_y, local_x, local_y) = Self::get_chunk_coords(&options.global_position);
        self.get_chunk(chunk_x, chunk_y)
            .map(|chunk| chunk[local_y][local_x])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = 0;
        
        // HashMap overhead
        total += std::mem::size_of::<HashMap<(i32, i32), Box<[[usize; CHUNK_SIZE]; CHUNK_SIZE]>>>();
        
        // Size of each chunk
        let chunk_size = std::mem::size_of::<[[usize; CHUNK_SIZE]; CHUNK_SIZE]>();
        total += self.chunks.len() * chunk_size;
        
        // Cache overhead
        total += std::mem::size_of::<[(i32, i32); CACHE_SIZE]>();
        
        total
    }
} 