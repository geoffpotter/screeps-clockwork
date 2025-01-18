use std::{collections::HashMap, option};
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::{fast_position, position::DecomposedPosition};

const ROOM_SIZE: usize = 50;
const ROOM_RADIUS: usize = 128; // Support ±128 rooms in each direction
const ROOM_ARRAY_SIZE: usize = ROOM_RADIUS * 2;

pub struct DecomposedArray4DMap {
    // 4D vector with dimensions [room_x][room_y][local_x][local_y]
    // Vectors are created on demand when values are set
    values: Vec<Option<Vec<Option<Vec<Option<Vec<usize>>>>>>>,
}

impl DecomposedArray4DMap {

    fn ensure_path(&mut self, room_x: usize, room_y: usize, x: usize, y: usize) -> &mut usize {
        // Ensure room_x vector exists
        if self.values.len() <= room_x {
            self.values.resize_with(room_x + 1, || None);
        }

        // Ensure room_y vector exists
        if self.values[room_x].is_none() {
            self.values[room_x] = Some(Vec::new());
        }
        let room_y_vec = self.values[room_x].as_mut().unwrap();
        
        if room_y_vec.len() <= room_y {
            room_y_vec.resize_with(room_y + 1, || None);
        }

        // Ensure local_x vector exists
        if room_y_vec[room_y].is_none() {
            room_y_vec[room_y] = Some(Vec::new());
        }
        let local_x_vec = room_y_vec[room_y].as_mut().unwrap();
        
        if local_x_vec.len() <= x {
            local_x_vec.resize_with(x + 1, || None);
        }

        // Ensure local_y vector exists
        if local_x_vec[x].is_none() {
            local_x_vec[x] = Some(Vec::with_capacity(ROOM_SIZE));
            let local_y_vec = local_x_vec[x].as_mut().unwrap();
            local_y_vec.resize(ROOM_SIZE, usize::MAX);
        }
        let local_y_vec = local_x_vec[x].as_mut().unwrap();
        
        if local_y_vec.len() <= y {
            local_y_vec.resize(y + 1, usize::MAX);
        }

        &mut local_y_vec[y]
    }
}

impl MapTrait for DecomposedArray4DMap {
    fn new() -> Self {
        Self {
            values: Vec::new(),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let pos = options.decomposed_position.decomposed();
        *self.ensure_path(pos.0 as usize, pos.1 as usize, pos.2 as usize, pos.3 as usize) = value;
        
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let (room_x, room_y, x, y) = options.decomposed_position.decomposed();
        self.values
            .get(room_x as usize)
            .and_then(|v| v.as_ref())
            .and_then(|v| v.get(room_y as usize))
            .and_then(|v| v.as_ref())
            .and_then(|v| v.get(x as usize))
            .and_then(|v| v.as_ref())
            .and_then(|v| v.get(y as usize))
            .copied()
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = 0;
        
        // Base vector size
        total += std::mem::size_of::<Vec<Option<Vec<Option<Vec<Option<Vec<usize>>>>>>>>();
        
        // Count actual allocated elements
        for room_x in self.values.iter().flatten() {
            total += std::mem::size_of::<Vec<Option<Vec<Option<Vec<usize>>>>>>();
            
            for room_y in room_x.iter().flatten() {
                total += std::mem::size_of::<Vec<Option<Vec<usize>>>>();
                
                for local_x in room_y.iter().flatten() {
                    total += std::mem::size_of::<Vec<usize>>();
                    total += local_x.capacity() * std::mem::size_of::<usize>();
                }
            }
        }
        
        total
    }
} 