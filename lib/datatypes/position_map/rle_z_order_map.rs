use super::{GlobalPoint, MapTrait};
use screeps::Position;

pub struct RleZOrderMap {
    runs: Vec<Run>,
    base_value: usize,
}

struct Run {
    start: usize,
    end: usize,
    delta: i32,
}

impl RleZOrderMap {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            base_value: usize::MAX,
        }
    }

    fn xy_to_z(x: i32, y: i32) -> usize {
        let mut z: usize = 0;
        // Use 32 bits to handle full global coordinate range
        for i in 0..32 {
            z |= (((x >> i) & 1) as usize) << (2 * i);
            z |= (((y >> i) & 1) as usize) << (2 * i + 1);
        }
        z
    }

    fn get_value(&self, z: usize) -> usize {
        if self.base_value == usize::MAX {
            return usize::MAX;
        }
        
        // Check if point is in any run
        let mut found = false;
        let mut value = self.base_value;
        
        for run in &self.runs {
            if z >= run.start && z < run.end {
                found = true;
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
        
        if found {
            value
        } else {
            usize::MAX
        }
    }

    fn set_value(&mut self, z: usize, value: usize) {
        if self.base_value == usize::MAX {
            self.base_value = value;
            // Create initial run for first value
            self.runs.push(Run {
                start: z,
                end: z + 1,
                delta: 0,
            });
            return;
        }

        let delta = value as isize - self.base_value as isize;
        if delta < i32::MIN as isize || delta > i32::MAX as isize {
            // Delta too large, reset map
            self.runs.clear();
            self.base_value = value;
            // Create new run for reset value
            self.runs.push(Run {
                start: z,
                end: z + 1,
                delta: 0,
            });
            return;
        }

        // Find or create run for this position
        for run in &mut self.runs {
            if z >= run.start && z < run.end {
                run.delta = delta as i32;
                return;
            }
        }

        // Create new run
        self.runs.push(Run {
            start: z,
            end: z + 1,
            delta: delta as i32,
        });

        // Merge adjacent runs with same delta
        if self.runs.len() > 1 {
            let mut i = 0;
            while i < self.runs.len() - 1 {
                if self.runs[i].delta == self.runs[i + 1].delta &&
                   self.runs[i].end == self.runs[i + 1].start {
                    self.runs[i].end = self.runs[i + 1].end;
                    self.runs.remove(i + 1);
                } else {
                    i += 1;
                }
            }
        }
    }
}

impl MapTrait for RleZOrderMap {
    fn new() -> Self {
        Self::new()
    }

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        let packed = pos.packed_repr();
        let room_x = ((packed >> 24) as i8) as i32;
        let room_y = (((packed >> 16) & 0xFF) as i8) as i32;
        let local_x = ((packed >> 8) & 0xFF) as i32;
        let local_y = (packed & 0xFF) as i32;
        
        let global_x = room_x * 50 + local_x;
        let global_y = room_y * 50 + local_y;
        
        let z = Self::xy_to_z(global_x, global_y);
        self.set_value(z, value);
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        let packed = pos.packed_repr();
        let room_x = ((packed >> 24) as i8) as i32;
        let room_y = (((packed >> 16) & 0xFF) as i8) as i32;
        let local_x = ((packed >> 8) & 0xFF) as i32;
        let local_y = (packed & 0xFF) as i32;
        
        let global_x = room_x * 50 + local_x;
        let global_y = room_y * 50 + local_y;
        
        let z = Self::xy_to_z(global_x, global_y);
        self.get_value(z)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        total += self.runs.capacity() * std::mem::size_of::<Run>();
        total
    }
}
