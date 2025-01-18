use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use rustc_hash::FxHasher;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::{GlobalYMajorPosition, fast_position};

const CHUNK_SIZE: usize = 64; // Power of 2 for efficient division
const CHUNK_MASK: usize = CHUNK_SIZE - 1;
const INITIAL_CHUNKS_CAPACITY: usize = 4; // Adjust based on expected usage

type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

pub struct ChunkedGlobalYMajorMap {
    // Using FxHasher which is faster for integer keys
    chunks: FxHashMap<(i32, i32), Box<[[usize; CHUNK_SIZE]; CHUNK_SIZE]>>,
}

impl ChunkedGlobalYMajorMap {
    fn get_chunk_coords(packed: u32) -> (i32, i32, usize, usize) {
        // Extract global coordinates
        let global_y = ((packed >> 16) & 0xFFFF) as i32;
        let global_x = (packed & 0xFFFF) as i32;
        
        // Calculate chunk coordinates using bit shifts (faster than division)
        // Since CHUNK_SIZE is 64 (2^6), we can shift by 6
        let chunk_x = global_x >> 6;
        let chunk_y = global_y >> 6;
        
        // Calculate local coordinates within chunk using mask
        let local_x = (global_x & CHUNK_MASK as i32) as usize;
        let local_y = (global_y & CHUNK_MASK as i32) as usize;
        
        (chunk_x, chunk_y, local_x, local_y)
    }
}

impl MapTrait for ChunkedGlobalYMajorMap {
    fn new() -> Self {
        Self {
            chunks: FxHashMap::with_capacity_and_hasher(
                INITIAL_CHUNKS_CAPACITY,
                BuildHasherDefault::default()
            ),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let (chunk_x, chunk_y, local_x, local_y) = Self::get_chunk_coords(options.global_y_major_packed_position.y_major_global());
        
        let chunk = self.chunks.entry((chunk_x, chunk_y))
            .or_insert_with(|| {
                let mut chunk = Box::new([[usize::MAX; CHUNK_SIZE]; CHUNK_SIZE]);
                chunk
            });
            
        chunk[local_y][local_x] = value;
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let (chunk_x, chunk_y, local_x, local_y) = Self::get_chunk_coords(options.global_y_major_packed_position.y_major_global());
        
        self.chunks.get(&(chunk_x, chunk_y))
            .map(|chunk| chunk[local_y][local_x])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = 0;
        
        // HashMap overhead
        total += std::mem::size_of::<FxHashMap<(i32, i32), Box<[[usize; CHUNK_SIZE]; CHUNK_SIZE]>>>();
        
        // Size of each chunk
        let chunk_size = std::mem::size_of::<[[usize; CHUNK_SIZE]; CHUNK_SIZE]>();
        total += self.chunks.len() * chunk_size;
        
        // HashMap buckets overhead (approximate)
        total += self.chunks.capacity() * std::mem::size_of::<(i32, i32)>();
        
        total
    }
} 