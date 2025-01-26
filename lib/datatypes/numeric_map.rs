use screeps::{xy_to_linear_index, RoomCoordinate, RoomXY, Position};
use num_traits::Bounded;
use std::ops::{Add, Sub};
use wasm_bindgen::prelude::*;

use screeps::constants::extra::{ROOM_AREA, ROOM_SIZE};
use super::local_index::LocalIndex;

/// Maps numeric values onto room tile positions.
/// T must be a numeric type that supports basic arithmetic and has a MAX value.
#[derive(Debug, Clone)]
pub struct NumericMap<T>([T; ROOM_AREA]) 
where 
    T: Copy + Bounded + Add<Output = T> + Sub<Output = T> + PartialEq;

impl<T> NumericMap<T> 
where 
    T: Copy + Bounded + Add<Output = T> + Sub<Output = T> + PartialEq
{
    /// Creates a new numeric map with all values set to T::max_value()
    #[inline]
    pub fn new() -> Self {
        Self([T::max_value(); ROOM_AREA])
    }

    /// Gets the value at the given position
    #[inline]
    pub fn get(&self, local: LocalIndex) -> T {
        self.0[local.index() as usize]
    }

    /// Gets a mutable reference to the value at the given position
    #[inline]
    pub fn get_mut(&mut self, local: LocalIndex) -> &mut T {
        &mut self.0[local.index() as usize]
    }

    /// Sets the value at the given position
    #[inline]
    pub fn set(&mut self, local: LocalIndex, value: T) {
        self.0[local.index() as usize] = value;
    }

    /// Returns an iterator over values and their positions
    pub fn iter(&self) -> impl Iterator<Item = (LocalIndex, T)> + '_ {
        self.0.iter().enumerate().map(|(i, &value)| {
            let x = (i / ROOM_SIZE as usize) as u8;
            let y = (i % ROOM_SIZE as usize) as u8;
            (LocalIndex::new(x, y), value)
        })
    }

    /// Returns an iterator over non-max values and their positions
    pub fn iter_non_max(&self) -> impl Iterator<Item = (LocalIndex, T)> + '_ {
        self.0.iter().enumerate().filter_map(|(i, &value)| {
            if value != T::max_value() {
                let x = (i / ROOM_SIZE as usize) as u8;
                let y = (i % ROOM_SIZE as usize) as u8;
                Some((LocalIndex::new(x, y), value))
            } else {
                None
            }
        })
    }
}

impl<T> Default for NumericMap<T> 
where 
    T: Copy + Bounded + Add<Output = T> + Sub<Output = T> + PartialEq
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut map: NumericMap<usize> = NumericMap::new();
        let local = LocalIndex::new(25, 25);
        
        // Test default value
        assert_eq!(map.get(local), usize::MAX);
        
        // Test setting and getting values
        map.set(local, 42);
        assert_eq!(map.get(local), 42);
        
        // Test mutable reference
        *map.get_mut(local) = 24;
        assert_eq!(map.get(local), 24);
    }

    #[test]
    fn test_edge_coordinates() {
        let mut map: NumericMap<usize> = NumericMap::new();
        
        // Test corners
        let corners = [
            LocalIndex::new(0, 0),
            LocalIndex::new(0, 49),
            LocalIndex::new(49, 0),
            LocalIndex::new(49, 49),
        ];

        for (i, corner) in corners.iter().enumerate() {
            map.set(*corner, i);
            assert_eq!(map.get(*corner), i);
        }
    }

    #[test]
    fn test_iterator_comprehensive() {
        let mut map: NumericMap<usize> = NumericMap::new();
        let positions = [
            LocalIndex::new(0, 0),
            LocalIndex::new(1, 1),
            LocalIndex::new(2, 2),
        ];

        // Set multiple values
        for (i, pos) in positions.iter().enumerate() {
            map.set(*pos, i);
        }

        // Test regular iterator
        let mut count = 0;
        for (pos, value) in map.iter() {
            if positions.contains(&pos) {
                assert_eq!(value, positions.iter().position(|p| p == &pos).unwrap());
                count += 1;
            } else {
                assert_eq!(value, usize::MAX);
            }
        }
        assert_eq!(count, positions.len());

        // Test non_max iterator
        let collected: Vec<_> = map.iter_non_max().collect();
        assert_eq!(collected.len(), positions.len());
        for (pos, value) in collected {
            assert!(positions.contains(&pos));
            assert_eq!(value, positions.iter().position(|p| p == &pos).unwrap());
        }
    }
}

// JavaScript interface
#[wasm_bindgen]
pub struct JsNumericMap {
    pub(crate) internal: NumericMap<usize>,
}

#[wasm_bindgen]
impl JsNumericMap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            internal: NumericMap::new(),
        }
    }

    #[wasm_bindgen(js_name = get)]
    pub fn js_get(&self, x: u8, y: u8) -> usize {
        let local = LocalIndex::new(x, y);
        self.internal.get(local)
    }

    #[wasm_bindgen(js_name = set)]
    pub fn js_set(&mut self, x: u8, y: u8, value: usize) {
        let local = LocalIndex::new(x, y);
        self.internal.set(local, value);
    }
}

impl Default for JsNumericMap {
    fn default() -> Self {
        Self::new()
    }
} 