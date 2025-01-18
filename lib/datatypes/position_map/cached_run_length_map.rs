use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const ROOM_SIZE: usize = 50;
const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;

#[derive(Clone)]
struct Run {
    start_index: usize,
    length: usize,
    delta: i64,
}

#[derive(Clone)]
struct RoomEncoding {
    base_value: usize,
    runs: Vec<Run>,
    decoded_values: Box<[usize; ROOM_AREA]>,
}

impl RoomEncoding {
    fn new() -> Self {
        Self {
            base_value: usize::MAX,
            runs: Vec::new(),
            decoded_values: Box::new([usize::MAX; ROOM_AREA]),
        }
    }

    fn set_value(&mut self, index: usize, value: usize) {
        if self.decoded_values[index] == usize::MAX {
            self.base_value = value;
            self.decode_values();
        } else {
            let current = self.decoded_values[index] as i64;
            let delta = value as i64 - current;
            
            self.runs.push(Run {
                start_index: index,
                length: 1,
                delta,
            });
            
            self.optimize_runs();
            self.decode_values();
        }
    }

    fn decode_values(&mut self) {
        // Reset all values to base
        for value in self.decoded_values.iter_mut() {
            *value = self.base_value;
        }
        
        // Apply runs
        for run in &self.runs {
            for i in 0..run.length {
                let idx = run.start_index + i;
                let current = self.decoded_values[idx] as i64;
                let new_value = current + run.delta;
                self.decoded_values[idx] = if new_value < 0 || new_value > usize::MAX as i64 {
                    usize::MAX
                } else {
                    new_value as usize
                };
            }
        }
    }

    fn optimize_runs(&mut self) {
        // Merge adjacent runs with same delta
        if self.runs.len() > 1 {
            let mut i = 0;
            while i < self.runs.len() - 1 {
                if self.runs[i].delta == self.runs[i + 1].delta &&
                   self.runs[i].start_index + self.runs[i].length == self.runs[i + 1].start_index {
                    self.runs[i].length += self.runs[i + 1].length;
                    self.runs.remove(i + 1);
                } else {
                    i += 1;
                }
            }
        }
    }
}

pub struct CachedRunLengthMap {
    rooms: HashMap<RoomName, RoomEncoding>,
    cached_room_coords: Option<RoomName>,
    cached_room: Option<*mut RoomEncoding>,
}

impl CachedRunLengthMap {
    fn get_indices(pos: Position) -> (RoomName, usize) {
        let room_name = pos.room_name();
        let local_x = pos.x().u8() as usize;
        let local_y = pos.y().u8() as usize;
        let index = local_y * ROOM_SIZE + local_x;
        (room_name, index)
    }

    fn update_cache(&mut self, room_name: RoomName) {
        if self.cached_room_coords != Some(room_name) {
            self.cached_room_coords = Some(room_name);
            self.cached_room = self.rooms.get_mut(&room_name)
                .map(|room| room as *mut _);
        }
    }
}

impl MapTrait for CachedRunLengthMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            cached_room_coords: None,
            cached_room: None,
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let (room_name, index) = Self::get_indices(options.position);
        self.update_cache(room_name);
        
        if let Some(cached) = self.cached_room {
            unsafe {
                (*cached).set_value(index, value);
            }
        } else {
            let room = self.rooms.entry(room_name).or_insert_with(RoomEncoding::new);
            room.set_value(index, value);
            
            // Update cache after insertion
            self.cached_room = Some(room as *mut _);
            self.cached_room_coords = Some(room_name);
        }
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let (room_name, index) = Self::get_indices(options.position);
        self.update_cache(room_name);
        
        if let Some(cached) = self.cached_room {
            unsafe {
                (*cached).decoded_values[index]
            }
        } else {
            usize::MAX
        }
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += self.rooms.len() * std::mem::size_of::<(RoomName, RoomEncoding)>();
        
        // Size of runs and decoded values in each room
        for room in self.rooms.values() {
            total += room.runs.capacity() * std::mem::size_of::<Run>();
            total += std::mem::size_of::<Box<[usize; ROOM_AREA]>>();
        }
        
        total
    }
}

// Safety: The raw pointer in cached_room is always valid when used
unsafe impl Send for CachedRunLengthMap {}
unsafe impl Sync for CachedRunLengthMap {} 