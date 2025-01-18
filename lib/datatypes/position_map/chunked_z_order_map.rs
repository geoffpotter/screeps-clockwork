use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

/// Global map using Z-order curve split into chunks for memory efficiency
pub struct ChunkedZOrderMap {
    chunks: HashMap<(i32, i32), Box<[usize; Self::CHUNK_SIZE]>>,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl ChunkedZOrderMap {
    const CHUNK_BITS: usize = 6;  // 2^6 = 64 squares per side
    const CHUNK_SIZE: usize = 1 << (Self::CHUNK_BITS * 2);  // Total size is 64*64 = 4096
    const CHUNK_MASK: i32 = (1 << Self::CHUNK_BITS) - 1;  // Mask for coordinates within chunk
    
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            min_x: i32::MAX,
            max_x: i32::MIN,
            min_y: i32::MAX,
            max_y: i32::MIN,
        }
    }

    /// Convert x,y coordinates to z-order curve index within a chunk
    #[inline(always)]
    fn xy_to_z(x: i32, y: i32) -> usize {
        // Ensure coordinates are within chunk bounds
        let x = x & Self::CHUNK_MASK;
        let y = y & Self::CHUNK_MASK;
        
        let mut z = 0;
        let mut x_temp = x as usize;
        let mut y_temp = y as usize;
        
        for i in 0..Self::CHUNK_BITS {
            z |= (x_temp & 1) << (2 * i);
            z |= (y_temp & 1) << (2 * i + 1);
            x_temp >>= 1;
            y_temp >>= 1;
        }
        z
    }

    /// Convert z-order index back to x,y coordinates within a chunk
    #[inline(always)]
    fn z_to_xy(z: usize) -> (i32, i32) {
        let mut x = 0;
        let mut y = 0;
        let mut z_temp = z;
        
        for i in 0..Self::CHUNK_BITS {
            x |= ((z_temp & (1 << (2 * i))) >> i) as i32;
            y |= ((z_temp & (1 << (2 * i + 1))) >> (i + 1)) as i32;
        }
        
        (x, y)
    }

    /// Get the chunk coordinates and local coordinates within the chunk
    #[inline(always)]
    fn get_chunk_coords(x: i32, y: i32) -> ((i32, i32), (i32, i32)) {
        let chunk_x = x >> Self::CHUNK_BITS;
        let chunk_y = y >> Self::CHUNK_BITS;
        let local_x = x & Self::CHUNK_MASK;
        let local_y = y & Self::CHUNK_MASK;
        ((chunk_x, chunk_y), (local_x, local_y))
    }
}

impl MapTrait for ChunkedZOrderMap {
    fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            min_x: i32::MAX,
            max_x: i32::MIN,
            min_y: i32::MAX,
            max_y: i32::MIN,
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let packed = options.position.packed_repr();
        let room_x = ((packed >> 24) as i8) as i32;
        let room_y = (((packed >> 16) & 0xFF) as i8) as i32;
        let local_x = ((packed >> 8) & 0xFF) as i32;
        let local_y = (packed & 0xFF) as i32;

        let global_x = room_x * 50 + local_x;
        let global_y = room_y * 50 + local_y;
        
        let ((chunk_x, chunk_y), (local_x, local_y)) = Self::get_chunk_coords(global_x, global_y);
        
        let chunk = self.chunks.entry((chunk_x, chunk_y))
            .or_insert_with(|| Box::new([usize::MAX; Self::CHUNK_SIZE]));
        
        let z = Self::xy_to_z(local_x, local_y);
        chunk[z] = value;
        
        self.min_x = self.min_x.min(global_x);
        self.max_x = self.max_x.max(global_x);
        self.min_y = self.min_y.min(global_y);
        self.max_y = self.max_y.max(global_y);
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let packed = options.position.packed_repr();
        let room_x = ((packed >> 24) as i8) as i32;
        let room_y = (((packed >> 16) & 0xFF) as i8) as i32;
        let local_x = ((packed >> 8) & 0xFF) as i32;
        let local_y = (packed & 0xFF) as i32;
        
        let global_x = room_x * 50 + local_x;
        let global_y = room_y * 50 + local_y;
        
        let ((chunk_x, chunk_y), (local_x, local_y)) = Self::get_chunk_coords(global_x, global_y);
        
        self.chunks.get(&(chunk_x, chunk_y))
            .map(|chunk| {
                let z = Self::xy_to_z(local_x, local_y);
                chunk[z]
            })
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let chunk_size = std::mem::size_of::<Box<[usize; Self::CHUNK_SIZE]>>();
        std::mem::size_of::<Self>() + self.chunks.len() * chunk_size
    }
} 