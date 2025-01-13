use std::collections::HashMap;
use super::{GlobalPoint, MapTrait};
use screeps::{Position, RoomName};

pub struct RunLengthDeltaMap {
    rooms: HashMap<RoomName, RoomEncoding>,
}

struct RoomEncoding {
    base_value: usize,
    runs: Vec<Run>,
}

struct Run {
    start_index: usize,
    length: usize,
    delta: i32,
}

impl RoomEncoding {
    fn new() -> Self {
        Self {
            base_value: usize::MAX,
            runs: Vec::new(),
        }
    }

    fn get_index(x: u8, y: u8) -> usize {
        // Y-major indexing
        (y as usize) * 50 + (x as usize)
    }

    fn get(&self, index: usize) -> usize {
        if self.base_value == usize::MAX {
            return usize::MAX;
        }

        let mut value = self.base_value;
        for run in &self.runs {
            if index >= run.start_index && index < run.start_index + run.length {
                let delta = run.delta as isize;
                if delta < 0 && value as isize + delta < 0 {
                    return usize::MAX;
                }
                if delta > 0 && value as isize + delta > usize::MAX as isize {
                    return usize::MAX;
                }
                value = (value as isize + delta) as usize;
                break;
            }
        }
        value
    }

    fn set(&mut self, index: usize, value: usize) {
        if self.base_value == usize::MAX {
            self.base_value = value;
            return;
        }

        let delta = value as isize - self.base_value as isize;
        if delta < i32::MIN as isize || delta > i32::MAX as isize {
            // Delta too large, reset room
            self.runs.clear();
            self.base_value = value;
            return;
        }

        // Find or create run for this position
        for run in &mut self.runs {
            if index >= run.start_index && index < run.start_index + run.length {
                run.delta = delta as i32;
                return;
            }
        }

        // Create new run
        self.runs.push(Run {
            start_index: index,
            length: 1,
            delta: delta as i32,
        });

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

impl MapTrait for RunLengthDeltaMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        let room_name = pos.room_name();
        let x = pos.x().u8();
        let y = pos.y().u8();
        let index = RoomEncoding::get_index(x, y);
        
        let room = self.rooms.entry(room_name).or_insert_with(RoomEncoding::new);
        room.set(index, value);
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        let room_name = pos.room_name();
        let x = pos.x().u8();
        let y = pos.y().u8();
        let index = RoomEncoding::get_index(x, y);
        
        self.rooms.get(&room_name)
            .map(|room| room.get(index))
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += std::mem::size_of::<HashMap<RoomName, RoomEncoding>>();
        
        // Size of runs in each room
        for room in self.rooms.values() {
            total += room.runs.capacity() * std::mem::size_of::<Run>();
        }
        
        total
    }
} 