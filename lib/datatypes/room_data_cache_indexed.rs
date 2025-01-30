use crate::datatypes::CustomCostMatrix;
use crate::datatypes::PositionIndex;
use crate::datatypes::RoomIndex;
use screeps::RoomName;
use std::collections::HashMap;
use std::ops::Fn;
use std::ops::Index;
use std::ops::IndexMut;

use super::DistanceMapIndexed;
use super::MultiroomDistanceMapIndexed;
use super::MultiroomNumericMap;
use super::NumericMap;

#[derive(Clone)]
pub struct RoomData {
    pub cost_matrix: Option<CustomCostMatrix>,
    pub distance_map: DistanceMapIndexed,
    pub room_index: RoomIndex,
}

pub struct RoomDataCache<F>
where
    F: Fn(RoomName) -> Option<CustomCostMatrix>,
{
    room_data: Vec<RoomData>,
    room_map: HashMap<RoomIndex, usize>,
    cost_matrix_creator: F,
    rooms_available: usize,
}

impl<F> RoomDataCache<F>
where
    F: Fn(RoomName) -> Option<CustomCostMatrix>,
{
    pub fn new(max_rooms: usize, cost_matrix_creator: F) -> Self {
        Self {
            room_data: vec![],
            room_map: HashMap::new(),
            cost_matrix_creator,
            rooms_available: max_rooms,
        }
    }

    pub fn get_room_key(&mut self, room_index: RoomIndex) -> Option<usize> {
        if let Some(room_key) = self.room_map.get(&room_index) {
            return Some(*room_key);
        }
        if self.rooms_available == 0 {
            return None;
        }
        let room_name = room_index.room_name();
        self.room_data.push(RoomData {
            cost_matrix: (self.cost_matrix_creator)(room_name),
            distance_map: DistanceMapIndexed::new(),
            room_index,
        });
        let key = self.room_data.len() - 1;
        self.room_map.insert(room_index, key);
        if self.room_data[key].cost_matrix.is_some() {
            self.rooms_available -= 1;
        }
        Some(key)
    }
}

impl<F> Index<usize> for RoomDataCache<F>
where
    F: Fn(RoomName) -> Option<CustomCostMatrix>,
{
    type Output = RoomData;

    fn index(&self, index: usize) -> &Self::Output {
        &self.room_data[index]
    }
}

impl<F> IndexMut<usize> for RoomDataCache<F>
where
    F: Fn(RoomName) -> Option<CustomCostMatrix>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.room_data[index]
    }
}

impl<F> From<RoomDataCache<F>> for MultiroomDistanceMapIndexed
where
    F: Fn(RoomName) -> Option<CustomCostMatrix>,
{
    fn from(cached_room_data: RoomDataCache<F>) -> Self {
        let mut maps = HashMap::new();
        for room_data in cached_room_data.room_data {
            let room_name = room_data.room_index;
            maps.insert(room_name, room_data.distance_map);
        }
        MultiroomDistanceMapIndexed {
            maps
        }
    }
}
