pub mod decomposed_position;
pub mod global_position;
pub mod global_y_major_position;
pub mod y_major_packed_position;

pub use decomposed_position::DecomposedPosition;
pub use global_position::GlobalPosition;
pub use global_y_major_position::GlobalYMajorPosition;
pub use y_major_packed_position::YMajorPackedPosition;

pub trait fast_position {
    fn new(packed: u32) -> Self;
    fn from_position(position: screeps::Position) -> Self;
    fn to_position(&self) -> screeps::Position;
    fn packed_repr(&self) -> u32;
    fn from_packed_repr(packed: u32) -> Self;
    fn xy(&self) -> screeps::RoomXY;
    fn x_major_room(&self) -> u16;
    fn x_major_local(&self) -> u16;
    fn x_major_global(&self) -> u32;
    fn y_major_room(&self) -> u16;
    fn y_major_local(&self) -> u16;
    fn y_major_global(&self) -> u32;
    fn z_order_room(&self) -> u16;
    fn z_order_local(&self) -> u16;
    fn z_order_global(&self) -> u32;
    fn hilbert_room(&self) -> u16;
    fn hilbert_local(&self) -> u16;
    fn hilbert_global(&self) -> u32;
    fn decomposed(&self) -> (u8, u8, u8, u8);
    fn room_name(&self) -> screeps::RoomName;
    fn neighbor_in_dir(&self, dir: screeps::Direction) -> Self;
}