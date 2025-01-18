use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::{TryInto, TryFrom};
use super::fast_position;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct GlobalPosition {
    // Stores global coordinates:
    // - 16 bits: global_x (room_x * 50 + x)
    // - 16 bits: global_y (room_y * 50 + y)
    pub x: i16,
    pub y: i16,
}

impl GlobalPosition {
    #[inline]
    pub fn new(room_x: u8, room_y: u8, x: u8, y: u8) -> Self {
        // Convert from offset room coordinates to signed using i16 instead of i8
        let room_x_i16 = (room_x as i16) - 128;
        let room_y_i16 = (room_y as i16) - 128;
        
        // Convert room coordinates to global coordinates
        let global_x = room_x_i16 * 50 + x as i16;
        let global_y = room_y_i16 * 50 + y as i16;
        
        Self { x: global_x, y: global_y }
    }

    #[inline]
    fn decompose_global(&self) -> (u8, u8, u8, u8) {
        // Convert global coordinates back to room + local using div_euclid and rem_euclid
        let room_x_i8 = self.x.div_euclid(50) as i8;
        let room_y_i8 = self.y.div_euclid(50) as i8;
        let local_x = self.x.rem_euclid(50) as u8;
        let local_y = self.y.rem_euclid(50) as u8;
        
        // Convert from signed room coordinates to offset using i16
        let room_x = (room_x_i8 as i16 + 128) as u8;
        let room_y = (room_y_i8 as i16 + 128) as u8;
        
        (room_x, room_y, local_x, local_y)
    }
}

impl fast_position for GlobalPosition {
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
        Position::from_packed(((room_x as u32) << 24) |
                            ((room_y as u32) << 16) |
                            ((x as u32) << 8) |
                            (y as u32))
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        ((room_x as u32) << 24) |
        ((room_y as u32) << 16) |
        ((x as u32) << 8) |
        (y as u32)
    }

    #[inline]
    fn from_packed_repr(packed: u32) -> Self {
        let room_x = ((packed >> 24) & 0xFF) as u8;
        let room_y = ((packed >> 16) & 0xFF) as u8;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        Self::new(room_x, room_y, x, y)
    }

    #[inline]
    fn xy(&self) -> RoomXY {
        let (_, _, x, y) = self.decompose_global();
        // SAFETY: x,y are always valid room coordinates (0-49)
        unsafe { RoomXY::unchecked_new(x, y) }
    }

    #[inline]
    fn x_major_room(&self) -> u16 {
        let (room_x, room_y, _, _) = self.decompose_global();
        (room_x as u16) | ((room_y as u16) << 8)
    }

    #[inline]
    fn x_major_local(&self) -> u16 {
        let (_, _, x, y) = self.decompose_global();
        ((x as u16) << 8) | (y as u16)
    }

    #[inline]
    fn x_major_global(&self) -> u32 {
        self.packed_repr()
    }

    #[inline]
    fn y_major_room(&self) -> u16 {
        let (room_x, room_y, _, _) = self.decompose_global();
        ((room_y as u16) << 8) | (room_x as u16)
    }

    #[inline]
    fn y_major_local(&self) -> u16 {
        let (_, _, x, y) = self.decompose_global();
        ((y as u16) << 8) | (x as u16)
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
        // Interleave bits for room coordinates (8 bits each)
        for i in 0..8 {
            z |= ((room_x & (1 << i)) as u32) << (i * 2) | 
                 ((room_y & (1 << i)) as u32) << (i * 2 + 1);
        }
        // Shift room bits to upper half and add local coordinates
        z = (z << 16) | 
            ((x as u32) << 8) |
            (y as u32);
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
        let mut pos_x = room_x as u16;
        let mut pos_y = room_y as u16;
        let mut d = 0u32;
        let mut rx: u16;
        let mut ry: u16;
        let mut s: u16 = 32768;
        
        while s > 0 {
            rx = if (pos_x & s) > 0 { 1 } else { 0 };
            ry = if (pos_y & s) > 0 { 1 } else { 0 };
            d = d.wrapping_add((s as u32).wrapping_mul(s as u32).wrapping_mul((3u32.wrapping_mul(rx as u32)) ^ (ry as u32)));
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
    fn decomposed(&self) -> (u8, u8, u8, u8) {
        self.decompose_global()
    }

    #[inline]
    fn room_name(&self) -> RoomName {
        let (room_x, room_y, _, _) = self.decompose_global();
        // Convert from offset coordinates to signed using i16
        let signed_x = (room_x as i16) - 128;
        let signed_y = (room_y as i16) - 128;
        let h = if signed_x < 0 { 'W' } else { 'E' };
        let v = if signed_y < 0 { 'N' } else { 'S' };
        let x = signed_x.abs() as u32;
        let y = signed_y.abs() as u32;
        RoomName::new(&format!("{}{}{}{}", h, x, v, y)).unwrap()
    }

    #[inline]
    fn neighbor_in_dir(&self, dir: Direction) -> Self {
        let (final_x, final_y) = match dir {
            Direction::Top => {
                if self.y % 50 == 0 {
                    (self.x, self.y.wrapping_sub(1))
                } else {
                    (self.x, self.y.wrapping_sub(1))
                }
            }
            Direction::TopRight => {
                if self.y % 50 == 0 && self.x % 50 == 49 {
                    (self.x.wrapping_add(1), self.y.wrapping_sub(1))
                } else if self.y % 50 == 0 {
                    (self.x.wrapping_add(1), self.y.wrapping_sub(1))
                } else if self.x % 50 == 49 {
                    (self.x.wrapping_add(1), self.y.wrapping_sub(1))
                } else {
                    (self.x.wrapping_add(1), self.y.wrapping_sub(1))
                }
            }
            Direction::Right => {
                if self.x % 50 == 49 {
                    (self.x.wrapping_add(1), self.y)
                } else {
                    (self.x.wrapping_add(1), self.y)
                }
            }
            Direction::BottomRight => {
                if self.y % 50 == 49 && self.x % 50 == 49 {
                    (self.x.wrapping_add(1), self.y.wrapping_add(1))
                } else if self.y % 50 == 49 {
                    (self.x.wrapping_add(1), self.y.wrapping_add(1))
                } else if self.x % 50 == 49 {
                    (self.x.wrapping_add(1), self.y.wrapping_add(1))
                } else {
                    (self.x.wrapping_add(1), self.y.wrapping_add(1))
                }
            }
            Direction::Bottom => {
                if self.y % 50 == 49 {
                    (self.x, self.y.wrapping_add(1))
                } else {
                    (self.x, self.y.wrapping_add(1))
                }
            }
            Direction::BottomLeft => {
                if self.y % 50 == 49 && self.x % 50 == 0 {
                    (self.x.wrapping_sub(1), self.y.wrapping_add(1))
                } else if self.y % 50 == 49 {
                    (self.x.wrapping_sub(1), self.y.wrapping_add(1))
                } else if self.x % 50 == 0 {
                    (self.x.wrapping_sub(1), self.y.wrapping_add(1))
                } else {
                    (self.x.wrapping_sub(1), self.y.wrapping_add(1))
                }
            }
            Direction::Left => {
                if self.x % 50 == 0 {
                    (self.x.wrapping_sub(1), self.y)
                } else {
                    (self.x.wrapping_sub(1), self.y)
                }
            }
            Direction::TopLeft => {
                if self.y % 50 == 0 && self.x % 50 == 0 {
                    (self.x.wrapping_sub(1), self.y.wrapping_sub(1))
                } else if self.y % 50 == 0 {
                    (self.x.wrapping_sub(1), self.y.wrapping_sub(1))
                } else if self.x % 50 == 0 {
                    (self.x.wrapping_sub(1), self.y.wrapping_sub(1))
                } else {
                    (self.x.wrapping_sub(1), self.y.wrapping_sub(1))
                }
            }
        };

        Self { x: final_x, y: final_y }
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
        let fast = GlobalPosition::from_position(original);
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_xy() {
        let pos = GlobalPosition::from_position(
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