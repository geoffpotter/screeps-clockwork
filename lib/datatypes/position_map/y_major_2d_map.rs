use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::{YMajorPackedPosition, fast_position};

const ROOM_SIZE: usize = 50;
const ROOM_RADIUS: usize = 128; // Support ±128 rooms in each direction
const ROOM_ARRAY_SIZE: usize = ROOM_RADIUS * 2;

pub struct YMajor2DMap {
    // 2D vector with dimensions [room][local]
    // Room index is y-major packed room coordinates
    // Local index is y-major packed local coordinates
    values: Vec<Option<Vec<usize>>>,
}

impl YMajor2DMap {
    fn get_indices(packed: YMajorPackedPosition) -> Option<(usize, usize)> {
        let room_idx = packed.y_major_room() as usize;
        let local_idx = packed.y_major_local() as usize;
        
        // Check bounds
        if room_idx >= ROOM_ARRAY_SIZE * ROOM_ARRAY_SIZE {
            return None;
        }
        
        Some((room_idx, local_idx))
    }

    fn ensure_path(&mut self, room_idx: usize, local_idx: usize) -> &mut usize {
        // Ensure room vector exists
        if self.values.len() <= room_idx {
            self.values.resize_with(room_idx + 1, || None);
        }

        // Ensure local vector exists
        if self.values[room_idx].is_none() {
            let mut local_vec = Vec::with_capacity(ROOM_SIZE * ROOM_SIZE);
            local_vec.resize(ROOM_SIZE * ROOM_SIZE, usize::MAX);
            self.values[room_idx] = Some(local_vec);
        }

        let local_vec = self.values[room_idx].as_mut().unwrap();
        if local_vec.len() <= local_idx {
            local_vec.resize(local_idx + 1, usize::MAX);
        }

        &mut local_vec[local_idx]
    }
}

impl MapTrait for YMajor2DMap {
    fn new() -> Self {
        Self {
            values: Vec::new(),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let (room_idx, local_idx) = (options.y_major_packed_position.y_major_room() as usize, options.y_major_packed_position.y_major_local() as usize);
        *self.ensure_path(room_idx, local_idx) = value;
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let (room_idx, local_idx) = (options.y_major_packed_position.y_major_room() as usize, options.y_major_packed_position.y_major_local() as usize);
        self.values.get(room_idx)
            .and_then(|room| room.as_ref())
            .and_then(|local_vec| local_vec.get(local_idx))
            .copied()
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = 0;
        
        // Base vector size
        total += std::mem::size_of::<Vec<Option<Vec<usize>>>>();
        
        // Count actual allocated elements
        for room_vec in self.values.iter().flatten() {
            total += std::mem::size_of::<Vec<usize>>();
            total += room_vec.capacity() * std::mem::size_of::<usize>();
        }
        
        total
    }
} 