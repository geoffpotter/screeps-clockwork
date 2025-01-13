use std::collections::HashMap;
use super::{GlobalPoint, MapTrait};
use screeps::Position;

const WORLD_SIZE: i32 = 16384; // 2^14, smaller but still plenty big
const ARRAY_SIZE: usize = (WORLD_SIZE as usize) * (WORLD_SIZE as usize);
const OFFSET: i32 = WORLD_SIZE / 2;

pub struct GlobalArrayMap {
    // Single flat array covering the entire world
    // Indexed by (x + offset) + (y + offset) * WORLD_SIZE
    values: Box<[usize]>,
}

impl GlobalArrayMap {
    fn get_index(point: GlobalPoint) -> Option<usize> {
        let x = point.x + OFFSET;
        let y = point.y + OFFSET;
        
        if x < 0 || x >= WORLD_SIZE || y < 0 || y >= WORLD_SIZE {
            return None;
        }
        
        Some((x + y * WORLD_SIZE) as usize)
    }
}

impl MapTrait for GlobalArrayMap {
    fn new() -> Self {
        Self {
            values: vec![usize::MAX; ARRAY_SIZE].into_boxed_slice(),
        }
    }

    fn set(&mut self, wpos: GlobalPoint, _pos: Position, value: usize) {
        if let Some(index) = Self::get_index(wpos) {
            self.values[index] = value;
        }
    }

    fn get(&mut self, wpos: GlobalPoint, _pos: Position) -> usize {
        Self::get_index(wpos)
            .map(|index| self.values[index])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        // Size of array of usize
        std::mem::size_of::<usize>() * ARRAY_SIZE
    }
} 