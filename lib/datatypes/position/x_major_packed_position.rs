use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::TryFrom;
use super::fast_position;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct XMajorPackedPosition {
    // x-major index (x * HEIGHT + y) for room coordinates
    room: u16,
    // x-major index (x * HEIGHT + y) for local coordinates
    local: u16,
}

impl XMajorPackedPosition {
    #[inline]
    pub fn new(room_x: u8, room_y: u8, x: u8, y: u8) -> Self {
        assert!(x < 50, "out of bounds x: {}", x);
        assert!(y < 50, "out of bounds y: {}", y);
        Self {
            room: ((room_x as u16) * 256) + (room_y as u16),
            local: ((x as u16) * 50) + (y as u16)
        }
    }

    #[inline]
    fn decompose_global(&self) -> (u8, u8, u8, u8) {
        let room_x = (self.room / 256) as u8;
        let room_y = (self.room % 256) as u8;
        let x = (self.local / 50) as u8;
        let y = (self.local % 50) as u8;
        (room_x, room_y, x, y)
    }
}

impl fast_position for XMajorPackedPosition {
    #[inline]
    fn new(packed: u32) -> Self {
        Self::from_packed_repr(packed)
    }

    #[inline]
    fn from_position(position: Position) -> Self {
        let packed = position.packed_repr();
        let room_x = ((packed >> 24) & 0xFF) as u8;
        let room_y = ((packed >> 16) & 0xFF) as u8;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        Self::new(room_x, room_y, x, y)
    }

    #[inline]
    fn to_position(&self) -> Position {
        let (room_x, room_y, x, y) = self.decompose_global();
        let room_idx = ((room_x as u16) << 8) | (room_y as u16);
        let packed = ((room_idx as u32) << 16) | ((x as u32) << 8) | (y as u32);
        Position::from_packed(packed)
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        ((self.room as u32) << 16) | (self.local as u32)
    }

    #[inline]
    fn from_packed_repr(packed: u32) -> Self {
        let room = ((packed >> 16) & 0xFFFF) as u16;
        let local = (packed & 0xFFFF) as u16;
        Self { room, local }
    }

    #[inline]
    fn xy(&self) -> RoomXY {
        let x = (self.local / 50) as u8;
        let y = (self.local % 50) as u8;
        // SAFETY: x,y are always valid room coordinates (0-49)
        unsafe { RoomXY::unchecked_new(x, y) }
    }

    #[inline]
    fn x_major_room(&self) -> u16 {
        self.room
    }

    #[inline]
    fn x_major_local(&self) -> u16 {
        self.local
    }

    #[inline]
    fn x_major_global(&self) -> u32 {
        ((self.room as u32) << 16) | (self.local as u32)
    }

    #[inline]
    fn y_major_room(&self) -> u16 {
        let room_x = (self.room / 256) as u8;
        let room_y = (self.room % 256) as u8;
        ((room_y as u16) * 256) + (room_x as u16)
    }

    #[inline]
    fn y_major_local(&self) -> u16 {
        let x = (self.local / 50) as u8;
        let y = (self.local % 50) as u8;
        ((y as u16) * 50) + (x as u16)
    }

    #[inline]
    fn y_major_global(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        ((room_y as u32) << 24) |
        ((room_x as u32) << 16) |
        ((y as u32) << 8) |
        (x as u32)
    }

    #[inline]
    fn z_order_room(&self) -> u16 {
        let (room_x, room_y, _, _) = self.decompose_global();
        let mut z = 0u16;
        for i in 0..8 {
            z |= ((room_x & (1 << i)) as u16) << i | ((room_y & (1 << i)) as u16) << (i + 1);
        }
        z
    }

    #[inline]
    fn z_order_local(&self) -> u16 {
        let (_, _, x, y) = self.decompose_global();
        let mut z = 0u16;
        for i in 0..8 {
            z |= ((x & (1 << i)) as u16) << i | ((y & (1 << i)) as u16) << (i + 1);
        }
        z
    }

    #[inline]
    fn z_order_global(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        let mut z = 0u32;
        for i in 0..8 {
            z |= ((room_x & (1 << i)) as u32) << (4*i) |
                 ((room_y & (1 << i)) as u32) << (4*i + 1) |
                 ((x & (1 << i)) as u32) << (4*i + 2) |
                 ((y & (1 << i)) as u32) << (4*i + 3);
        }
        z
    }

    #[inline]
    fn hilbert_room(&self) -> u16 {
        let (room_x, room_y, _, _) = self.decompose_global();
        let mut x = room_x as u16;
        let mut y = room_y as u16;
        let mut d = 0u16;
        let mut rx: u16;
        let mut ry: u16;
        let mut s: u16 = 128;
        
        while s > 0 {
            rx = if (x & s) > 0 { 1 } else { 0 };
            ry = if (y & s) > 0 { 1 } else { 0 };
            d = d.wrapping_add(s.wrapping_mul(s).wrapping_mul((3u16.wrapping_mul(rx)) ^ ry));
            if ry == 0 {
                if rx == 1 {
                    x = s.wrapping_sub(1).wrapping_sub(x);
                    y = s.wrapping_sub(1).wrapping_sub(y);
                }
                std::mem::swap(&mut x, &mut y);
            }
            s = s.wrapping_div(2);
        }
        d
    }

    #[inline]
    fn hilbert_local(&self) -> u16 {
        let (_, _, x, y) = self.decompose_global();
        let mut pos_x = x as u16;
        let mut pos_y = y as u16;
        let mut d = 0u16;
        let mut rx: u16;
        let mut ry: u16;
        let mut s: u16 = 64;
        
        while s > 0 {
            rx = if (pos_x & s) > 0 { 1 } else { 0 };
            ry = if (pos_y & s) > 0 { 1 } else { 0 };
            d = d.wrapping_add(s.wrapping_mul(s).wrapping_mul((3u16.wrapping_mul(rx)) ^ ry));
            if ry == 0 {
                if rx == 1 {
                    pos_x = s.wrapping_sub(1).wrapping_sub(pos_x);
                    pos_y = s.wrapping_sub(1).wrapping_sub(pos_y);
                }
                std::mem::swap(&mut pos_x, &mut pos_y);
            }
            s = s.wrapping_div(2);
        }
        d
    }

    #[inline]
    fn hilbert_global(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        let mut global_x = ((room_x as u16) << 8) | (x as u16);
        let mut global_y = ((room_y as u16) << 8) | (y as u16);
        let mut d = 0u32;
        let mut rx: u16;
        let mut ry: u16;
        let mut s: u16 = 32768;
        
        while s > 0 {
            rx = if (global_x & s) > 0 { 1 } else { 0 };
            ry = if (global_y & s) > 0 { 1 } else { 0 };
            d = d.wrapping_add((s as u32).wrapping_mul(s as u32).wrapping_mul((3u32.wrapping_mul(rx as u32)) ^ (ry as u32)));
            if ry == 0 {
                if rx == 1 {
                    global_x = s.wrapping_sub(1).wrapping_sub(global_x);
                    global_y = s.wrapping_sub(1).wrapping_sub(global_y);
                }
                std::mem::swap(&mut global_x, &mut global_y);
            }
            s = s.wrapping_div(2);
        }
        d
    }

    #[inline]
    fn decomposed(&self) -> (u8, u8, u8, u8) {
        self.decompose_global()
    }

    #[inline]
    fn room_name(&self) -> RoomName {
        let (room_x, room_y, _, _) = self.decompose_global();
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
        // For local: x * 50 + y
        // For room: x * 256 + y
        
        let mut new_room = self.room;
        let mut new_local = self.local;
        
        match dir {
            Direction::Top => {
                if new_local % 50 == 0 { // y == 0
                    new_room = new_room.wrapping_sub(1); // room_y -= 1
                    new_local = new_local + 49; // y = 49
                } else {
                    new_local = new_local - 1; // y -= 1
                }
            }
            Direction::TopRight => {
                if new_local % 50 == 0 { // y == 0
                    new_room = new_room.wrapping_sub(1); // room_y -= 1
                    if new_local >= (49 * 50) { // x == 49
                        new_room = new_room.wrapping_add(256); // room_x += 1
                        new_local = 49; // x = 0, y = 49
                    } else {
                        new_local = new_local + 99; // x += 1, y = 49
                    }
                } else {
                    if new_local >= (49 * 50) { // x == 49
                        new_room = new_room.wrapping_add(256); // room_x += 1
                        new_local = new_local - 2450; // x = 0, y -= 1
                    } else {
                        new_local = new_local + 49; // x += 1, y -= 1
                    }
                }
            }
            Direction::Right => {
                if new_local >= (49 * 50) { // x == 49
                    new_room = new_room.wrapping_add(256); // room_x += 1
                    new_local = new_local - 2450; // x = 0
                } else {
                    new_local = new_local + 50; // x += 1
                }
            }
            Direction::BottomRight => {
                if new_local % 50 == 49 { // y == 49
                    new_room = new_room.wrapping_add(1); // room_y += 1
                    if new_local >= (49 * 50) { // x == 49
                        new_room = new_room.wrapping_add(256); // room_x += 1
                        new_local = 0; // x = 0, y = 0
                    } else {
                        new_local = new_local - 49 + 50; // x += 1, y = 0
                    }
                } else {
                    if new_local >= (49 * 50) { // x == 49
                        new_room = new_room.wrapping_add(256); // room_x += 1
                        new_local = new_local - 2449; // x = 0, y += 1
                    } else {
                        new_local = new_local + 51; // x += 1, y += 1
                    }
                }
            }
            Direction::Bottom => {
                if new_local % 50 == 49 { // y == 49
                    new_room = new_room.wrapping_add(1); // room_y += 1
                    new_local = new_local - 49; // y = 0
                } else {
                    new_local = new_local + 1; // y += 1
                }
            }
            Direction::BottomLeft => {
                if new_local % 50 == 49 { // y == 49
                    new_room = new_room.wrapping_add(1); // room_y += 1
                    if new_local < 50 { // x == 0
                        new_room = new_room.wrapping_sub(256); // room_x -= 1
                        new_local = 49 * 50; // x = 49, y = 0
                    } else {
                        new_local = new_local - 99; // x -= 1, y = 0
                    }
                } else {
                    if new_local < 50 { // x == 0
                        new_room = new_room.wrapping_sub(256); // room_x -= 1
                        new_local = (49 * 50) + new_local + 1; // x = 49, y += 1
                    } else {
                        new_local = new_local - 49; // x -= 1, y += 1
                    }
                }
            }
            Direction::Left => {
                if new_local < 50 { // x == 0
                    new_room = new_room.wrapping_sub(256); // room_x -= 1
                    new_local = (49 * 50) + (new_local % 50); // x = 49
                } else {
                    new_local = new_local - 50; // x -= 1
                }
            }
            Direction::TopLeft => {
                if new_local % 50 == 0 { // y == 0
                    new_room = new_room.wrapping_sub(1); // room_y -= 1
                    if new_local < 50 { // x == 0
                        new_room = new_room.wrapping_sub(256); // room_x -= 1
                        new_local = (49 * 50) + 49; // x = 49, y = 49
                    } else {
                        new_local = new_local - 51; // x -= 1, y = 49
                    }
                } else {
                    if new_local < 50 { // x == 0
                        new_room = new_room.wrapping_sub(256); // room_x -= 1
                        new_local = (49 * 50) + (new_local - 1); // x = 49, y -= 1
                    } else {
                        new_local = new_local - 51; // x -= 1, y -= 1
                    }
                }
            }
        }
        
        Self {
            room: new_room,
            local: new_local
        }
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
        let fast = XMajorPackedPosition::from_position(original);
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_xy() {
        let pos = XMajorPackedPosition::from_position(
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