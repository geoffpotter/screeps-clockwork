use screeps::constants::extra::{ROOM_AREA, ROOM_SIZE};
use wasm_bindgen::prelude::*;
use super::local_index::LocalIndex;

/// Maps a value of type T onto individual room tile positions.
#[derive(Debug, Clone)]
pub struct GenericMap<T>([Option<T>; ROOM_AREA]);

impl<T: Copy> GenericMap<T> {
    /// Creates a new generic map with all values set to None.
    #[inline]
    pub fn new() -> Self where T: Default {
        Self([None; ROOM_AREA])
    }

    /// Gets a reference to the value at the given position
    #[inline]
    pub fn get(&self, local: LocalIndex) -> Option<&T> {
        self.0[local.index() as usize].as_ref()
    }

    /// Gets a mutable reference to the value at the given position
    #[inline]
    pub fn get_mut(&mut self, local: LocalIndex) -> Option<&mut T> {
        self.0[local.index() as usize].as_mut()
    }

    /// Sets the value at the given position
    #[inline]
    pub fn set(&mut self, local: LocalIndex, value: T) {
        self.0[local.index() as usize] = Some(value);
    }

    /// Clears the value at the given position
    #[inline]
    pub fn clear(&mut self, local: LocalIndex) {
        self.0[local.index() as usize] = None;
    }

    /// Returns an iterator over non-None values and their positions
    pub fn iter(&self) -> impl Iterator<Item = (LocalIndex, &T)> {
        self.0.iter().enumerate().filter_map(|(i, val)| {
            val.as_ref().map(|v| {
                let x = (i / ROOM_SIZE as usize) as u8;
                let y = (i % ROOM_SIZE as usize) as u8;
                (LocalIndex::new(x, y), v)
            })
        })
    }
}

impl<T: Copy + Default> Default for GenericMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

// JavaScript interface
#[wasm_bindgen]
pub struct JsGenericMap {
    values: Vec<Option<JsValue>>
}

#[wasm_bindgen]
impl JsGenericMap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            values: vec![None; ROOM_AREA]
        }
    }

    #[wasm_bindgen(js_name = get)]
    pub fn js_get(&self, x: u8, y: u8) -> JsValue {
        let local = LocalIndex::new(x, y);
        self.values[local.index() as usize]
            .clone()
            .unwrap_or(JsValue::UNDEFINED)
    }

    #[wasm_bindgen(js_name = set)]
    pub fn js_set(&mut self, x: u8, y: u8, value: JsValue) {
        let local = LocalIndex::new(x, y);
        self.values[local.index() as usize] = Some(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut map: GenericMap<i32> = GenericMap::new();
        let local = LocalIndex::new(25, 25);
        
        // Test default value
        assert_eq!(map.get(local), None);
        
        // Test setting and getting values
        map.set(local, 42);
        assert_eq!(map.get(local), Some(&42));
        
        // Test mutable reference
        if let Some(value) = map.get_mut(local) {
            *value = 24;
        }
        assert_eq!(map.get(local), Some(&24));

        map.clear(local);
        assert_eq!(map.get(local), None);
    }

    #[test]
    fn test_iteration() {
        let mut map: GenericMap<i32> = GenericMap::new();
        let local = LocalIndex::new(0, 0);
        map.set(local, 42);
        
        let mut found = false;
        for (pos, &value) in map.iter() {
            if pos == local {
                assert_eq!(value, 42);
                found = true;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_custom_type() {
        #[derive(Clone, Debug, PartialEq, Default, Copy)]
        struct TestStruct {
            value: i32,
        }
        
        let mut map: GenericMap<TestStruct> = GenericMap::new();
        let local = LocalIndex::new(25, 25);
        
        let test_value = TestStruct { value: 42 };
        map.set(local, test_value);
        
        assert_eq!(map.get(local), Some(&test_value));
    }
}
