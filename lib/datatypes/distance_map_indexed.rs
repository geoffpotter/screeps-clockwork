// Original under MIT license from: https://github.com/einargs/rust-screeps-code/blob/main/src/rooms/tile_slice.rs
use std::ops::{Index, IndexMut};
use wasm_bindgen::prelude::*;

use screeps::{constants::extra::{ROOM_AREA, ROOM_SIZE}, xy_to_linear_index, RoomXY};

use super::LocalIndex;

/// Maps a distance value onto individual room tile positions.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct DistanceMapIndexed([usize; ROOM_AREA]);

impl DistanceMapIndexed {
    /// Creates a new distance map with all values defaulted to `usize::MAX`.
    #[inline]
    pub fn new() -> DistanceMapIndexed {
        DistanceMapIndexed([usize::MAX; ROOM_AREA])
    }

    /// Converts the distance map into a vector of distances.
    pub fn to_vec(&self) -> Vec<usize> {
        self.0.to_vec()
    }
}

impl Default for DistanceMapIndexed {
    /// Creates a new distance map with all values defaulted to `usize::MAX`.
    fn default() -> DistanceMapIndexed {
        DistanceMapIndexed([usize::MAX; ROOM_AREA])
    }
}

impl Index<usize> for DistanceMapIndexed {
    type Output = usize;
    /// Gets the distance value at a given index.
    fn index(&self, index: usize) -> &usize {
        &self.0[index]
    }
}

/// Allows indexing by raw linear index
impl IndexMut<usize> for DistanceMapIndexed {
    fn index_mut(&mut self, index: usize) -> &mut usize {
        &mut self.0[index]
    }
}

/// Allows indexing by RoomXY directly
impl Index<RoomXY> for DistanceMapIndexed {
    type Output = usize;
    fn index(&self, index: RoomXY) -> &usize {
        &self.0[xy_to_linear_index(index)]
    }
}

/// Allows indexing by RoomXY to get a mutable copy of the associated data
impl IndexMut<RoomXY> for DistanceMapIndexed {
    fn index_mut(&mut self, index: RoomXY) -> &mut usize {
        &mut self.0[xy_to_linear_index(index)]
    }
}

/// Allows indexing by RoomXY references
impl Index<&RoomXY> for DistanceMapIndexed {
    type Output = usize;
    fn index(&self, index: &RoomXY) -> &usize {
        &self.0[xy_to_linear_index(*index)]
    }
}

/// Allows indexing by RoomXY references to get a mutable copy of the associated data
impl IndexMut<&RoomXY> for DistanceMapIndexed {
    fn index_mut(&mut self, index: &RoomXY) -> &mut usize {
        &mut self.0[xy_to_linear_index(*index)]
    }
}

/// Allows indexing by local_index
impl Index<LocalIndex> for DistanceMapIndexed {
    type Output = usize;
    fn index(&self, index: LocalIndex) -> &usize {
        &self.0[index.index()]
    }
}

/// Allows indexing by local_index to get a mutable copy of the associated data
impl IndexMut<LocalIndex> for DistanceMapIndexed {
    fn index_mut(&mut self, index: LocalIndex) -> &mut usize {
        &mut self.0[index.index()]
    }
}

/// Allows indexing by a reference to a local_index
impl Index<&LocalIndex> for DistanceMapIndexed {
    type Output = usize;
    fn index(&self, index: &LocalIndex) -> &usize {
        &self.0[index.index()]
    }
}

/// Allows indexing by a reference to a local_index to get a mutable copy of the associated data
impl IndexMut<&LocalIndex> for DistanceMapIndexed {
    fn index_mut(&mut self, index: &LocalIndex) -> &mut usize {
        &mut self.0[index.index()]
    }
}

/// Iterator that yields (LocalIndex, &T) pairs
pub struct DistanceMapEnumerate<'a> {
    tile_map: &'a DistanceMapIndexed,
    current_index: LocalIndex,
}

impl<'a> Iterator for DistanceMapEnumerate<'a> {
    type Item = (LocalIndex, &'a usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index.index() >= ROOM_AREA {
            return None;
        }

        let local = self.current_index;
        let value = &self.tile_map.0[self.current_index];

        let next_pos = LocalIndex::new(self.current_index.x() + 1, self.current_index.y());
        self.current_index = next_pos;
        Some((local, value))
    }
}

impl DistanceMapIndexed {
    /// Returns an iterator that yields (RoomXY, &T) pairs
    pub fn enumerate(&self) -> DistanceMapEnumerate {
        DistanceMapEnumerate {
            tile_map: self,
            current_index: LocalIndex::new(0, 0),
        }
    }
}

#[wasm_bindgen]
impl DistanceMapIndexed {
    /// Converts the distance map into a flat array of distances.
    #[wasm_bindgen(js_name = toArray)]
    pub fn to_array(&self) -> Vec<usize> {
        self.0.to_vec()
    }

    /// Gets the distance value at a given position.
    #[wasm_bindgen(js_name = get)]
    pub fn js_get(&self, x: u8, y: u8) -> usize {
        let local = LocalIndex::new(x, y);
        self.0[local.index()]
    }

    /// Sets the distance value at a given position.
    #[wasm_bindgen(js_name = set)]
    pub fn js_set(&mut self, x: u8, y: u8, value: usize) {
        let local = LocalIndex::new(x, y);
        self.0[local.index()] = value;
    }
}
