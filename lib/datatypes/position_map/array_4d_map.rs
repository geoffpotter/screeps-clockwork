use super::{GlobalPoint, MapTrait};
use screeps::Position;

const ROOM_SIZE: usize = 50;
const ROOM_RADIUS: usize = 128; // Support ±128 rooms in each direction
const ROOM_ARRAY_SIZE: usize = ROOM_RADIUS * 2;

pub struct Array4DMap {
    // 4D vector with dimensions [room_x][room_y][local_x][local_y]
    // Vectors are created on demand when values are set
    values: Vec<Option<Vec<Option<Vec<Option<Vec<usize>>>>>>>,
}

impl Array4DMap {
    fn get_indices(pos: Position) -> Option<(usize, usize, usize, usize)> {
        let packed = pos.packed_repr();
        // Extract signed room coordinates
        let room_x = ((packed >> 24) as i8) as i32;
        let room_y = (((packed >> 16) & 0xFF) as i8) as i32;
        let local_x = ((packed >> 8) & 0xFF) as usize;
        let local_y = (packed & 0xFF) as usize;
        
        // Convert room coordinates to array indices with offset
        let room_x_idx = (room_x + ROOM_RADIUS as i32) as usize;
        let room_y_idx = (room_y + ROOM_RADIUS as i32) as usize;
        
        // Check bounds
        if room_x_idx >= ROOM_ARRAY_SIZE || room_y_idx >= ROOM_ARRAY_SIZE {
            return None;
        }
        
        Some((room_x_idx, room_y_idx, local_x, local_y))
    }

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

impl MapTrait for Array4DMap {
    fn new() -> Self {
        Self {
            values: Vec::new(),
        }
    }

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        if let Some((room_x, room_y, x, y)) = Self::get_indices(pos) {
            *self.ensure_path(room_x, room_y, x, y) = value;
        }
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        Self::get_indices(pos)
            .and_then(|(room_x, room_y, x, y)| {
                self.values.get(room_x)?.as_ref()?
                    .get(room_y)?.as_ref()?
                    .get(x)?.as_ref()?
                    .get(y).copied()
            })
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