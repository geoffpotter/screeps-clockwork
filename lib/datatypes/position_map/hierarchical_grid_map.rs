use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const ROOM_SIZE: usize = 50;
const GRID_SIZE: usize = 10;
const GRIDS_PER_SIDE: usize = ROOM_SIZE / GRID_SIZE;

pub struct HierarchicalGridMap {
    rooms: HashMap<RoomName, RoomLayer>,
}

struct RoomLayer {
    grids: Box<[Grid; GRIDS_PER_SIDE * GRIDS_PER_SIDE]>,
}

struct Grid {
    cells: Box<[usize; GRID_SIZE * GRID_SIZE]>,
}

impl Grid {
    fn new() -> Self {
        Self {
            cells: Box::new([usize::MAX; GRID_SIZE * GRID_SIZE]),
        }
    }

    fn get_index(x: u8, y: u8) -> usize {
        // Y-major indexing within grid
        (y as usize) * GRID_SIZE + (x as usize)
    }
}

impl RoomLayer {
    fn new() -> Self {
        Self {
            grids: Box::new(core::array::from_fn(|_| Grid::new())),
        }
    }

    fn get_indices(x: u8, y: u8) -> (usize, u8, u8) {
        let grid_x = x as usize / GRID_SIZE;
        let grid_y = y as usize / GRID_SIZE;
        let local_x = (x as usize % GRID_SIZE) as u8;
        let local_y = (y as usize % GRID_SIZE) as u8;
        
        // Y-major indexing for grid selection
        let grid_index = grid_y * GRIDS_PER_SIDE + grid_x;
        (grid_index, local_x, local_y)
    }
}

impl MapTrait for HierarchicalGridMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let room_name = options.position.room_name();
        let x = options.position.x().u8();
        let y = options.position.y().u8();
        
        let room = self.rooms.entry(room_name).or_insert_with(RoomLayer::new);
        let (grid_idx, local_x, local_y) = RoomLayer::get_indices(x, y);
        let cell_idx = Grid::get_index(local_x, local_y);
        
        room.grids[grid_idx].cells[cell_idx] = value;
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let room_name = options.position.room_name();
        let x = options.position.x().u8();
        let y = options.position.y().u8();
        
        self.rooms.get(&room_name)
            .map(|room| {
                let (grid_idx, local_x, local_y) = RoomLayer::get_indices(x, y);
                let cell_idx = Grid::get_index(local_x, local_y);
                room.grids[grid_idx].cells[cell_idx]
            })
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += std::mem::size_of::<HashMap<RoomName, RoomLayer>>();
        
        // Size of grids and cells in each room
        for room in self.rooms.values() {
            total += std::mem::size_of::<Box<[Grid; GRIDS_PER_SIDE * GRIDS_PER_SIDE]>>();
            for grid in room.grids.iter() {
                total += std::mem::size_of::<Box<[usize; GRID_SIZE * GRID_SIZE]>>();
            }
        }
        
        total
    }
} 