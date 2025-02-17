// Original under MIT license from: https://github.com/einargs/rust-screeps-code/blob/main/src/rooms/tile_slice.rs

use screeps::{xy_to_linear_index, RoomCoordinate, RoomXY};
use std::ops::{Index, IndexMut};
use wasm_bindgen::prelude::*;

use screeps::constants::extra::{ROOM_AREA, ROOM_SIZE};

/// Maps a distance value onto individual room tile positions.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct DistanceMap([usize; ROOM_AREA]);

impl DistanceMap {
    /// Creates a new distance map with all values defaulted to `usize::MAX`.
    #[inline]
    pub fn new() -> DistanceMap {
        DistanceMap([usize::MAX; ROOM_AREA])
    }

    /// Converts the distance map into a vector of distances.
    pub fn to_vec(&self) -> Vec<usize> {
        self.0.to_vec()
    }
}

impl Default for DistanceMap {
    /// Creates a new distance map with all values defaulted to `usize::MAX`.
    fn default() -> DistanceMap {
        DistanceMap([usize::MAX; ROOM_AREA])
    }
}

impl Index<usize> for DistanceMap {
    type Output = usize;
    /// Gets the distance value at a given index.
    fn index(&self, index: usize) -> &usize {
        &self.0[index]
    }
}

/// Allows indexing by raw linear index
impl IndexMut<usize> for DistanceMap {
    fn index_mut(&mut self, index: usize) -> &mut usize {
        &mut self.0[index]
    }
}

/// Allows indexing by RoomXY directly
impl Index<RoomXY> for DistanceMap {
    type Output = usize;
    fn index(&self, index: RoomXY) -> &usize {
        &self.0[xy_to_linear_index(index)]
    }
}

/// Allows indexing by RoomXY to get a mutable copy of the associated data
impl IndexMut<RoomXY> for DistanceMap {
    fn index_mut(&mut self, index: RoomXY) -> &mut usize {
        &mut self.0[xy_to_linear_index(index)]
    }
}

/// Allows indexing by RoomXY references
impl Index<&RoomXY> for DistanceMap {
    type Output = usize;
    fn index(&self, index: &RoomXY) -> &usize {
        &self.0[xy_to_linear_index(*index)]
    }
}

/// Allows indexing by RoomXY references to get a mutable copy of the associated data
impl IndexMut<&RoomXY> for DistanceMap {
    fn index_mut(&mut self, index: &RoomXY) -> &mut usize {
        &mut self.0[xy_to_linear_index(*index)]
    }
}

/// Iterator that yields (RoomXY, &T) pairs
pub struct DistanceMapEnumerate<'a> {
    tile_map: &'a DistanceMap,
    current_index: usize,
}

impl<'a> Iterator for DistanceMapEnumerate<'a> {
    type Item = (RoomXY, &'a usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= ROOM_AREA {
            return None;
        }

        let x = RoomCoordinate::new((self.current_index / ROOM_SIZE as usize) as u8).unwrap();
        let y = RoomCoordinate::new((self.current_index % ROOM_SIZE as usize) as u8).unwrap();
        let xy = RoomXY::new(x, y);
        let value = &self.tile_map.0[self.current_index];

        self.current_index += 1;
        Some((xy, value))
    }
}

impl DistanceMap {
    /// Returns an iterator that yields (RoomXY, &T) pairs
    pub fn enumerate(&self) -> DistanceMapEnumerate {
        DistanceMapEnumerate {
            tile_map: self,
            current_index: 0,
        }
    }
}

#[wasm_bindgen]
impl DistanceMap {
    /// Converts the distance map into a flat array of distances.
    #[wasm_bindgen(js_name = toArray)]
    pub fn to_array(&self) -> Vec<usize> {
        self.0.to_vec()
    }

    /// Gets the distance value at a given position.
    #[wasm_bindgen(js_name = get)]
    pub fn js_get(&self, x: u8, y: u8) -> usize {
        let x = RoomCoordinate::new(x)
            .unwrap_or_else(|_| wasm_bindgen::throw_str(&format!("Invalid x coordinate: {}", x)));
        let y = RoomCoordinate::new(y)
            .unwrap_or_else(|_| wasm_bindgen::throw_str(&format!("Invalid y coordinate: {}", y)));
        self.0[xy_to_linear_index(RoomXY::new(x, y))]
    }

    /// Sets the distance value at a given position.
    #[wasm_bindgen(js_name = set)]
    pub fn js_set(&mut self, x: u8, y: u8, value: usize) {
        let x = RoomCoordinate::new(x)
            .unwrap_or_else(|_| wasm_bindgen::throw_str(&format!("Invalid x coordinate: {}", x)));
        let y = RoomCoordinate::new(y)
            .unwrap_or_else(|_| wasm_bindgen::throw_str(&format!("Invalid y coordinate: {}", y)));
        self.0[xy_to_linear_index(RoomXY::new(x, y))] = value;
    }
}
