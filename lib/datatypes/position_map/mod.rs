mod chunked_z_order_map;
mod rle_z_order_map;
mod z_order_global_map;
mod benchmark;
mod quadtree_map;
mod hierarchical_grid_map;
mod hash_grid_map;
mod run_length_delta_map;
mod sparse_block_map;
mod prefix_tree_map;
mod global_array_map;
mod room_array_map;
mod array_4d_map;
mod flat_array_map;
mod dense_hash_map;
mod bit_packed_map;
mod cached_multiroom_map;
mod cached_room_array_map;
mod cached_sparse_block_map;
mod cached_run_length_map;
mod simple_hash_map;
mod vector_array_map;

use std::convert::TryFrom;
pub use chunked_z_order_map::ChunkedZOrderMap;
pub use rle_z_order_map::RleZOrderMap;
use screeps::{Position, RoomCoordinate};
pub use z_order_global_map::ZOrderGlobalMap;
pub use benchmark::BenchmarkStats;
pub use quadtree_map::QuadtreeMap;
pub use hierarchical_grid_map::HierarchicalGridMap;
pub use hash_grid_map::HashGridMap;
pub use run_length_delta_map::RunLengthDeltaMap;
pub use sparse_block_map::SparseBlockMap;
pub use prefix_tree_map::PrefixTreeMap;
pub use global_array_map::GlobalArrayMap;
pub use room_array_map::RoomArrayMap;
pub use array_4d_map::Array4DMap;
pub use flat_array_map::FlatArrayMap;
pub use dense_hash_map::DenseHashMap;
pub use bit_packed_map::BitPackedMap;
pub use cached_multiroom_map::CachedMultiroomMap;
pub use cached_room_array_map::CachedRoomArrayMap;
pub use cached_sparse_block_map::CachedSparseBlockMap;
pub use cached_run_length_map::CachedRunLengthMap;
pub use simple_hash_map::SimpleHashMap;
pub use vector_array_map::VectorArrayMap;

use super::MultiroomDistanceMap;


/// A point in global coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalPoint {
    pub x: i32,
    pub y: i32,
}

pub trait MapTrait {
    fn new() -> Self;
    fn set(&mut self, wpos: GlobalPoint, pos: Position, value: usize);
    fn get(&mut self, wpos: GlobalPoint, pos: Position) -> usize;
    fn memory_usage(&self) -> usize;
}




impl MapTrait for ZOrderGlobalMap {
    fn new() -> Self { Self::new() }
    fn set(&mut self, wpos: GlobalPoint, _pos: Position, value: usize) { 
        self.set(wpos, value) 
    }
    fn get(&mut self, wpos: GlobalPoint, _pos: Position) -> usize { 
        ZOrderGlobalMap::get(self, wpos)
    }
    fn memory_usage(&self) -> usize { self.memory_usage() }
}



impl MapTrait for MultiroomDistanceMap {
    fn new() -> Self { Self::new() }
    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        self.set(pos, value);
    }
    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        MultiroomDistanceMap::get(self, pos)
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
    
    map.set(wpos, pos, 1);
    let value = map.get(wpos, pos);
    assert_eq!(value, 1, "Should be able to set then get a value");
    
    // Test point not set - in room W1N1 at local coordinates (1,1)
    let missing_room = "W1N1".parse().unwrap();
    let missing_x = RoomCoordinate::try_from(1).unwrap();
    let missing_y = RoomCoordinate::try_from(1).unwrap();
    let missing_pos = Position::new(missing_x, missing_y, missing_room);
    let missing_point = GlobalPoint { x: 51, y: 51 }; // In the next room over
    
    assert_eq!(map.get(missing_point, missing_pos), usize::MAX, "Missing point should return usize::MAX");
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
}
