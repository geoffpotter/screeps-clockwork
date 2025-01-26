use std::convert::TryFrom;
use std::ops::{Index, IndexMut};

use screeps::{linear_index_to_xy, xy_to_linear_index, LocalCostMatrix, RoomCoordinate, RoomXY, ROOM_AREA};
use wasm_bindgen::__rt::WasmRefCell;
use wasm_bindgen::prelude::*;

use super::local_index::LocalIndex;

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct CustomCostMatrix {
    bits: [u8; ROOM_AREA],
}

#[wasm_bindgen]
impl CustomCostMatrix {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { bits: [0; ROOM_AREA] }
    }
}

impl CustomCostMatrix {
    pub fn new_with_value(value: u8) -> Self {
        Self { bits: [value; ROOM_AREA] }
    }

    pub fn get(&self, pos: RoomXY) -> u8 {
        self.bits[xy_to_linear_index(pos)]
    }

    pub fn set(&mut self, pos: RoomXY, value: u8) {
        self.bits[xy_to_linear_index(pos)] = value;
    }

    pub fn get_local(&self, pos: LocalIndex) -> u8 {
        self.bits[pos.index()]
    }

    pub fn set_local(&mut self, pos: LocalIndex, value: u8) {
        self.bits[pos.index()] = value;
    }
}

impl Default for CustomCostMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<LocalIndex> for CustomCostMatrix {
    type Output = u8;

    fn index(&self, pos: LocalIndex) -> &Self::Output {
        &self.bits[pos.index() as usize]
    }
}

impl IndexMut<LocalIndex> for CustomCostMatrix {
    fn index_mut(&mut self, pos: LocalIndex) -> &mut Self::Output {
        &mut self.bits[pos.index()]
    }
}

// this has to be one way, LocalCostMatrix doesn't let you set the bits
impl From<LocalCostMatrix> for CustomCostMatrix {
    fn from(value: LocalCostMatrix) -> Self {
        Self { bits: *value.get_bits() }
    }
}


#[wasm_bindgen(inline_js = "
    export function customcostmatrix_get_pointer(value) {
        if (!value || 
            typeof value !== 'object' || 
            !('__wbg_ptr' in value) ||
            (value.constructor.name !== 'CustomCostMatrix' && value.constructor.name !== 'ClockworkCostMatrix')) {
            return 0;
        }
        return value.__wbg_ptr;
    }
")]
extern "C" {
    fn customcostmatrix_get_pointer(value: JsValue) -> u32;
}
impl TryFrom<JsValue> for CustomCostMatrix {
    type Error = &'static str;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        let ptr = customcostmatrix_get_pointer(value);
        if ptr == 0 {
            return Err("Invalid customcostmatrix_get_pointer reference");
        }
        let me = ptr as *mut WasmRefCell<CustomCostMatrix>;
        wasm_bindgen::__rt::assert_not_null(me);
        let me = unsafe { &*me };
        Ok(me.borrow().clone())
    }
}




/// A wrapper around the `LocalCostMatrix` type from the Screeps API.
/// Instances can be passed between WASM and JS as a pointer, using the
/// methods to get and set values, rather than copying the entire matrix.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct ClockworkCostMatrix {
    internal: LocalCostMatrix,
}

#[wasm_bindgen]
impl ClockworkCostMatrix {
    /// Creates a new cost matrix within the WASM module. Optionally, a default value
    /// can be provided to initialize all cells in the matrix to that value.
    #[wasm_bindgen(constructor)]
    pub fn new(default: Option<u8>) -> ClockworkCostMatrix {
        match default {
            Some(default) => ClockworkCostMatrix {
                internal: LocalCostMatrix::new_with_value(default),
            },
            None => ClockworkCostMatrix {
                internal: LocalCostMatrix::new(),
            },
        }
    }

    /// Gets the cost of a given position in the cost matrix.
    #[wasm_bindgen(js_name = "get")]
    pub fn js_get(&self, x: u8, y: u8) -> u8 {
        let x = RoomCoordinate::new(x)
            .unwrap_or_else(|_| wasm_bindgen::throw_str(&format!("Invalid x coordinate: {}", x)));
        let y = RoomCoordinate::new(y)
            .unwrap_or_else(|_| wasm_bindgen::throw_str(&format!("Invalid y coordinate: {}", y)));
        self.internal.get(RoomXY::new(x, y))
    }

    /// Sets the cost of a given position in the cost matrix.
    #[wasm_bindgen(js_name = "set")]
    pub fn js_set(&mut self, x: u8, y: u8, value: u8) {
        let x = RoomCoordinate::new(x)
            .unwrap_or_else(|_| wasm_bindgen::throw_str(&format!("Invalid x coordinate: {}", x)));
        let y = RoomCoordinate::new(y)
            .unwrap_or_else(|_| wasm_bindgen::throw_str(&format!("Invalid y coordinate: {}", y)));
        self.internal.set(RoomXY::new(x, y), value);
    }


    #[wasm_bindgen(js_name = "toCustomCostMatrix")]
    pub fn js_to_custom_cost_matrix(&self) -> CustomCostMatrix {
        CustomCostMatrix { bits: *self.internal.get_bits() }
    }
}

impl ClockworkCostMatrix {
    /// Gets the cost of a given position in the cost matrix.
    pub fn get(&self, xy: RoomXY) -> u8 {
        self.internal.get(xy)
    }

    /// Sets the cost of a given position in the cost matrix.
    pub fn set(&mut self, xy: RoomXY, value: u8) {
        self.internal.set(xy, value);
    }
}

impl ClockworkCostMatrix {
    /// Gets the internal `LocalCostMatrix` instance from the wrapper.
    pub fn get_internal(&self) -> &LocalCostMatrix {
        &self.internal
    }

    pub fn get_custom(&self) -> CustomCostMatrix {
        CustomCostMatrix {
            bits: *self.internal.get_bits(),
        }
    }
}

// Add get/set methods for LocalIndex
impl ClockworkCostMatrix {
    /// Gets the cost at the given LocalIndex position
    pub fn get_local(&self, pos: LocalIndex) -> u8 {
        let (x, y) = pos.xy();
        let x = RoomCoordinate::new(x).expect("invalid x coordinate");
        let y = RoomCoordinate::new(y).expect("invalid y coordinate");
        self.internal.get(RoomXY::new(x, y))
    }

    /// Sets the cost at the given LocalIndex position
    pub fn set_local(&mut self, pos: LocalIndex, value: u8) {
        let (x, y) = pos.xy();
        let x = RoomCoordinate::new(x).expect("invalid x coordinate");
        let y = RoomCoordinate::new(y).expect("invalid y coordinate");
        self.internal.set(RoomXY::new(x, y), value);
    }
}

#[wasm_bindgen(inline_js = "
    export function clockworkcostmatrix_get_pointer(value) {
        if (!value || 
            typeof value !== 'object' || 
            !('__wbg_ptr' in value) ||
            value.constructor.name !== 'ClockworkCostMatrix') {
            return 0;
        }
        return value.__wbg_ptr;
    }
")]
extern "C" {
    fn clockworkcostmatrix_get_pointer(value: JsValue) -> u32;
}

impl TryFrom<JsValue> for ClockworkCostMatrix {
    type Error = &'static str;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        let ptr = clockworkcostmatrix_get_pointer(value);
        if ptr == 0 {
            return Err("Invalid ClockworkCostMatrix reference");
        }
        let me = ptr as *mut WasmRefCell<ClockworkCostMatrix>;
        wasm_bindgen::__rt::assert_not_null(me);
        let me = unsafe { &*me };
        Ok(me.borrow().clone())
    }
}

impl From<LocalCostMatrix> for ClockworkCostMatrix {
    fn from(value: LocalCostMatrix) -> Self {
        ClockworkCostMatrix { internal: value }
    }
}

