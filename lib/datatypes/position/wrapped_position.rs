use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::TryFrom;
use super::fast_position;
use super::packed_position::PackedPosition;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct WrappedPosition(pub PackedPosition);

impl WrappedPosition {
    #[inline]
    pub fn new(room_x: u8, room_y: u8, x: u8, y: u8) -> Self {
        Self(PackedPosition::new(room_x, room_y, x, y))
    }
}

impl fast_position for WrappedPosition {
    #[inline]
    fn new(packed: u32) -> Self {
        Self(PackedPosition::from_packed_repr(packed))
    }

    #[inline]
    fn from_position(position: Position) -> Self {
        Self(PackedPosition::from_position(position))
    }

    #[inline]
    fn to_position(&self) -> Position {
        self.0.to_position()
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        self.0.packed_repr()
    }

    #[inline]
    fn from_packed_repr(packed: u32) -> Self {
        Self(PackedPosition::from_packed_repr(packed))
    }

    #[inline]
    fn xy(&self) -> RoomXY {
        self.0.xy()
    }

    #[inline]
    fn x_major_room(&self) -> u16 {
        self.0.x_major_room()
    }

    #[inline]
    fn x_major_local(&self) -> u16 {
        self.0.x_major_local()
    }

    #[inline]
    fn x_major_global(&self) -> u32 {
        self.0.x_major_global()
    }

    #[inline]
    fn y_major_room(&self) -> u16 {
        self.0.y_major_room()
    }

    #[inline]
    fn y_major_local(&self) -> u16 {
        self.0.y_major_local()
    }

    #[inline]
    fn y_major_global(&self) -> u32 {
        self.0.y_major_global()
    }

    #[inline]
    fn z_order_room(&self) -> u16 {
        self.0.z_order_room()
    }

    #[inline]
    fn z_order_local(&self) -> u16 {
        self.0.z_order_local()
    }

    #[inline]
    fn z_order_global(&self) -> u32 {
        self.0.z_order_global()
    }

    #[inline]
    fn hilbert_room(&self) -> u16 {
        self.0.hilbert_room()
    }

    #[inline]
    fn hilbert_local(&self) -> u16 {
        self.0.hilbert_local()
    }

    #[inline]
    fn hilbert_global(&self) -> u32 {
        self.0.hilbert_global()
    }

    #[inline]
    fn decomposed(&self) -> (u8, u8, u8, u8) {
        self.0.decomposed()
    }

    #[inline]
    fn room_name(&self) -> RoomName {
        let (room_x, room_y, _, _) = self.decomposed();
        // Convert from offset coordinates to signed for room name
        let signed_x = (room_x as i16).wrapping_sub(128);
        let signed_y = (room_y as i16).wrapping_sub(128);
        let h = if signed_x < 0 { 'W' } else { 'E' };
        let v = if signed_y < 0 { 'N' } else { 'S' };
        let x = signed_x.unsigned_abs() as u32;
        let y = signed_y.unsigned_abs() as u32;
        RoomName::new(&format!("{}{}{}{}", h, x, v, y)).unwrap()
    }

    #[inline]
    fn neighbor_in_dir(&self, dir: Direction) -> Self {
        Self(self.0.neighbor_in_dir(dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use screeps::Position;

    #[test]
    fn test_roundtrip() {
        let original = Position::new(
            RoomCoordinate::try_from(25_u8).unwrap(),
            RoomCoordinate::try_from(25_u8).unwrap(),
            "E0N0".parse().unwrap()
        );
        let fast = WrappedPosition::from_position(original);
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_xy() {
        let pos = WrappedPosition::from_position(
            Position::new(
                RoomCoordinate::try_from(25_u8).unwrap(),
                RoomCoordinate::try_from(30_u8).unwrap(),
                "E0N0".parse().unwrap()
            )
        );
        let xy = pos.xy();
        assert_eq!(u8::from(xy.x), 25);
        assert_eq!(u8::from(xy.y), 30);
    }
} 