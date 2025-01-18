use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const CELL_SIZE: i32 = 50;

pub struct HashGridMap {
    cells: HashMap<(i32, i32), Box<[[usize; CELL_SIZE as usize]; CELL_SIZE as usize]>>,
}

impl MapTrait for HashGridMap {
    fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let cell_x = options.global_point.x / CELL_SIZE;
        let cell_y = options.global_point.y / CELL_SIZE;
        let local_x = ((options.global_point.x % CELL_SIZE) + CELL_SIZE) % CELL_SIZE;
        let local_y = ((options.global_point.y % CELL_SIZE) + CELL_SIZE) % CELL_SIZE;
        
        let cell = self.cells.entry((cell_x, cell_y))
            .or_insert_with(|| Box::new([[usize::MAX; CELL_SIZE as usize]; CELL_SIZE as usize]));
        
        cell[local_y as usize][local_x as usize] = value;
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let cell_x = options.global_point.x / CELL_SIZE;
        let cell_y = options.global_point.y / CELL_SIZE;
        let local_x = ((options.global_point.x % CELL_SIZE) + CELL_SIZE) % CELL_SIZE;
        let local_y = ((options.global_point.y % CELL_SIZE) + CELL_SIZE) % CELL_SIZE;
        
        self.cells.get(&(cell_x, cell_y))
            .map(|cell| cell[local_y as usize][local_x as usize])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of HashMap overhead
        total += std::mem::size_of::<HashMap<(i32, i32), Box<[[usize; CELL_SIZE as usize]; CELL_SIZE as usize]>>>();
        
        // Size of each cell's array
        total += self.cells.len() * (CELL_SIZE * CELL_SIZE) as usize * std::mem::size_of::<usize>();
        
        total
    }
} 