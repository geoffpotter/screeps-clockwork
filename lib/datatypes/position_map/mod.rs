mod array_4d_map;
mod bit_packed_map;
mod cached_multiroom_map;
mod cached_room_array_map;
mod cached_run_length_map;
mod cached_sparse_block_map;
mod chunked_z_order_map;
mod dense_hash_map;
mod flat_array_map;
mod global_array_map;
mod hash_grid_map;
mod hierarchical_grid_map;
mod prefix_tree_map;
mod quadtree_map;
mod rle_z_order_map;
mod room_array_map;
mod run_length_delta_map;
mod simple_hash_map;
mod sparse_block_map;
mod vector_array_map;
mod z_order_global_map;
mod y_major_packed_map;
mod benchmark;

// New implementations
mod decomposed_array_4d_map;
mod chunked_global_y_major_map;
mod chunked_global_map;
mod y_major_2d_map;

pub use array_4d_map::Array4DMap;
pub use bit_packed_map::BitPackedMap;
pub use cached_multiroom_map::CachedMultiroomMap;
pub use cached_room_array_map::CachedRoomArrayMap;
pub use cached_run_length_map::CachedRunLengthMap;
pub use cached_sparse_block_map::CachedSparseBlockMap;
pub use chunked_z_order_map::ChunkedZOrderMap;
pub use dense_hash_map::DenseHashMap;
pub use flat_array_map::FlatArrayMap;
pub use global_array_map::GlobalArrayMap;
pub use hash_grid_map::HashGridMap;
pub use hierarchical_grid_map::HierarchicalGridMap;
pub use prefix_tree_map::PrefixTreeMap;
pub use quadtree_map::QuadtreeMap;
pub use rle_z_order_map::RleZOrderMap;
pub use room_array_map::RoomArrayMap;
pub use run_length_delta_map::RunLengthDeltaMap;
pub use simple_hash_map::SimpleHashMap;
pub use sparse_block_map::SparseBlockMap;
pub use vector_array_map::VectorArrayMap;
pub use z_order_global_map::ZOrderGlobalMap;
pub use y_major_packed_map::YMajorPackedMap;

// New implementations
pub use decomposed_array_4d_map::DecomposedArray4DMap;
pub use chunked_global_y_major_map::ChunkedGlobalYMajorMap;
pub use chunked_global_map::ChunkedGlobalMap;
pub use y_major_2d_map::YMajor2DMap;

use std::convert::TryFrom;
use screeps::{Position, RoomName, RoomCoordinate};
use crate::datatypes::{fast_position, position::{DecomposedPosition, GlobalPosition, GlobalYMajorPosition, YMajorPackedPosition}};

use super::MultiroomDistanceMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionOptions {
    pub position: Position,
    pub y_major_packed_position: YMajorPackedPosition,
    pub global_point: GlobalPoint,
    pub global_y_major_packed_position: GlobalYMajorPosition,
    pub global_position: GlobalPosition,
    pub decomposed_position: DecomposedPosition,
}

pub trait MapTrait {
    fn new() -> Self;
    fn set(&mut self, options: PositionOptions, value: usize);
    fn get(&mut self, options: PositionOptions) -> usize;
    fn memory_usage(&self) -> usize;
}


impl MapTrait for ZOrderGlobalMap {
    fn new() -> Self { Self::new() }
    fn set(&mut self, options: PositionOptions, value: usize) { 
        self.set(options.global_point, value) 
    }
    fn get(&mut self, options: PositionOptions) -> usize { 
        ZOrderGlobalMap::get(self, options.global_point)
    }
    fn memory_usage(&self) -> usize { self.memory_usage() }
}



impl MapTrait for MultiroomDistanceMap {
    fn new() -> Self { Self::new() }
    fn set(&mut self, options: PositionOptions, value: usize) {
        self.set(options.position, value);
    }
    fn get(&mut self, options: PositionOptions) -> usize {
        MultiroomDistanceMap::get(self, options.position)
    }
    fn memory_usage(&self) -> usize {
        self.memory_usage()
    }
} 


fn test_position_map<T: MapTrait>(map: &mut T) {
    // Create a position in room W0N0 at local coordinates (10,10)
    let room_name = "W0N0".parse().unwrap();
    let x = RoomCoordinate::try_from(10).unwrap();
    let y = RoomCoordinate::try_from(10).unwrap();
    let pos = Position::new(x, y, room_name);
    let wpos = GlobalPoint { x: 10, y: 10 };
    let packed_pos = YMajorPackedPosition::from_position(pos);
    let global_y_major_packed_position = GlobalYMajorPosition::from_position(pos);
    let global_position = GlobalPosition::from_position(pos);
    let decomposed_position = DecomposedPosition::from_position(pos);

    let options = PositionOptions {
        position: pos,
        y_major_packed_position: packed_pos,
        global_point: wpos,
        global_y_major_packed_position,
        global_position,
        decomposed_position,
    };
    
    map.set(options.clone(), 1);
    let value = map.get(options);
    assert_eq!(value, 1, "Should be able to set then get a value");
    
    // Test point not set - in room W1N1 at local coordinates (1,1)
    let missing_room = "W1N1".parse().unwrap();
    let missing_x = RoomCoordinate::try_from(1).unwrap();
    let missing_y = RoomCoordinate::try_from(1).unwrap();
    let missing_pos = Position::new(missing_x, missing_y, missing_room);
    let missing_point = GlobalPoint { x: 51, y: 51 }; // In the next room over
    let missing_packed = YMajorPackedPosition::from_position(missing_pos);
    let missing_global_y_major = GlobalYMajorPosition::from_position(missing_pos);
    let missing_global = GlobalPosition::from_position(missing_pos);
    let missing_decomposed = DecomposedPosition::from_position(missing_pos);

    let missing_options = PositionOptions {
        position: missing_pos,
        y_major_packed_position: missing_packed,
        global_point: missing_point,
        global_y_major_packed_position: missing_global_y_major,
        global_position: missing_global,
        decomposed_position: missing_decomposed,
    };
    
    assert_eq!(map.get(missing_options), usize::MAX, "Missing point should return usize::MAX");
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use screeps::RoomCoordinate;

    use super::*;

    #[test]
    fn test_multiroom_distance_map() {

        // let pos = Position::new(RoomCoordinate::try_from(10).unwrap(), RoomCoordinate::try_from(10).unwrap(), "W127N127".parse().unwrap());
        // let packed = pos.packed_repr();
        // let room_x = packed >> 24;
        // let room_y = packed >> 16 & 0xFF;
        // let x = packed >> 8 & 0xFF;
        // let y = packed & 0xFF;
        // println!("pos: {}, room_x: {}, room_y: {}, x: {}, y: {}", pos, room_x, room_y, x, y);


        let mut map = MultiroomDistanceMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_z_order_global_map() {
        let mut map = ZOrderGlobalMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_chunked_z_order_map() {
        let mut map = ChunkedZOrderMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_rle_z_order_map() {
        let mut map = RleZOrderMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_quadtree_map() {
        let mut map = QuadtreeMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_hierarchical_grid_map() {
        let mut map = HierarchicalGridMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_hash_grid_map() {
        let mut map = HashGridMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_run_length_delta_map() {
        let mut map = RunLengthDeltaMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_sparse_block_map() {
        let mut map = SparseBlockMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_prefix_tree_map() {
        let mut map = PrefixTreeMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_global_array_map() {
        let mut map = GlobalArrayMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_room_array_map() {
        let mut map = RoomArrayMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_array_4d_map() {
        let mut map = Array4DMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_flat_array_map() {
        let mut map = FlatArrayMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_dense_hash_map() {
        let mut map = DenseHashMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_bit_packed_map() {
        let mut map = BitPackedMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_cached_multiroom_map() {
        let mut map = CachedMultiroomMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_cached_room_array_map() {
        let mut map = CachedRoomArrayMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_cached_sparse_block_map() {
        let mut map = CachedSparseBlockMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_cached_run_length_map() {
        let mut map = CachedRunLengthMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_simple_hash_map() {
        let mut map = SimpleHashMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_vector_array_map() {
        let mut map = VectorArrayMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_y_major_packed_map() {
        let mut map = YMajorPackedMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_chunked_global_y_major_map() {
        let mut map = ChunkedGlobalYMajorMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_chunked_global_map() {
        let mut map = ChunkedGlobalMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_y_major_2d_map() {
        let mut map = YMajor2DMap::new();
        test_position_map(&mut map);
    }

    #[test]
    fn test_decomposed_array_4d_map() {
        let mut map = DecomposedArray4DMap::new();
        test_position_map(&mut map);
    }


}


