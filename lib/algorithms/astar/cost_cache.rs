use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use screeps::{Position, RoomName, RoomTerrain};
use crate::algorithms::jps::{Cost, MapPosition, RoomIndex, WorldPosition, MAX_ROOMS, OBSTACLE};
use crate::datatypes::{ClockworkCostMatrix, OptionalCache};
use crate::log;
use crate::utils::PROFILER;
use crate::algorithms::jps::RoomInfo;

static mut COST_CACHE: Option<CostCache<'static>> = None;

pub struct CostCache<'a> {

    current_room_cost_matrix: Option<ClockworkCostMatrix>,
    current_matrix_room: Option<RoomName>,
    cost_matrices: OptionalCache<'a, RoomName, ClockworkCostMatrix>,

    reverse_room_table: Vec<RoomIndex>,
    room_table: Vec<RoomInfo>,
    look_table: [Cost; 4],
    blocked_rooms: HashSet<MapPosition>,
}

impl<'a> CostCache<'a> {
    pub fn get_instance() -> &'static mut CostCache<'static> {
        unsafe {
            if COST_CACHE.is_none() {
                COST_CACHE = Some(CostCache::new(|_| None));
            }
            COST_CACHE.as_mut().unwrap()
        }
    }

    pub fn new<F>(get_cost_matrix: F) -> Self 
    where 
        F: Fn(RoomName) -> Option<ClockworkCostMatrix> + 'a
    {
        let cost_matrices = OptionalCache::new(get_cost_matrix);

        Self {
            reverse_room_table: vec![0; 1 << 16], // 2^16 possible room positions,
            room_table: Vec::with_capacity(MAX_ROOMS),
            look_table: [1, OBSTACLE, 5, 0],
            blocked_rooms: HashSet::new(),

            current_room_cost_matrix: None,
            current_matrix_room: None,

            cost_matrices,
        }
    }
    
    /// Calculate the cost of moving to a position
    pub fn look(&mut self, pos: WorldPosition) -> Cost {
        let map_pos = pos.map_position();
        let room_index = match self.reverse_room_table[map_pos.id() as usize] {
            0 => self.room_index_from_pos(map_pos).unwrap(),
            // 0 => OBSTACLE,
            i => i,
        };
        // let room_index = self.room_index_from_pos(map_pos).unwrap();

        let terrain = &self.room_table[(room_index - 1) as usize];
        let cost_matrix_value = terrain.cost_matrix[pos.xx as usize % 50][pos.yy as usize % 50];

        if cost_matrix_value != 0 {
            // log(&format!("Cost matrix value: {:?}", cost_matrix_value));
            if cost_matrix_value == 0xff {
                return OBSTACLE;
            }
            return cost_matrix_value as Cost;
        }

        let terrain_type = terrain.look((pos.xx % 50) as u8, (pos.yy % 50) as u8) as usize;
        // log(&format!("Terrain type: {:?}, cost: {:?}", terrain_type, self.look_table[terrain_type]));
        self.look_table[terrain_type]
    }

    /// Get or create a room index for a map position
    fn room_index_from_pos(&mut self, map_pos: MapPosition) -> Option<RoomIndex> {
        let room_index = self.reverse_room_table[map_pos.id() as usize];
        if room_index != 0 {
            return Some(room_index as RoomIndex);
        }

        // Room not found - try to create new entry
        if self.room_table.len() >= MAX_ROOMS {
            return None;
        }

        if self.blocked_rooms.contains(&map_pos) {
            return None;
        }

        // Load terrain data for this room
        let terrain_data = self.get_terrain(map_pos)?;

        // Create new room info
        let room = RoomInfo::new(terrain_data.to_vec(), None, map_pos);

        self.room_table.push(room);
        let new_index = self.room_table.len() as RoomIndex;
        self.reverse_room_table[map_pos.id() as usize] = new_index;

        Some(new_index)
    }
    pub fn get_terrain(&self, pos: MapPosition) -> Option<Vec<u8>> {
        let room_name = RoomName::from_packed(pos.id());
        let terrain = RoomTerrain::new(room_name)?;
        let buffer = terrain.get_raw_buffer().to_vec();

        // Transform from [y * 50 + x] to [x * 50 + y]
        let mut transformed = vec![0; 2500];
        for y in 0..50 {
            for x in 0..50 {
                transformed[x * 50 + y] = buffer[y * 50 + x];
            }
        }

        // Compact to 2 bits per cell
        let compacted = transformed
            .chunks(4)
            .map(|chunk| chunk[0] | chunk[1] << 2 | chunk[2] << 4 | chunk[3] << 6)
            .collect::<Vec<_>>();

        Some(compacted)
    }
} 