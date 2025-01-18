use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::TryFrom;
use super::fast_position;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct DecomposedPosition {
    room_x: u8,  // Stored with +128 offset from signed coordinate
    room_y: u8,  // Stored with +128 offset from signed coordinate
    x: u8,
    y: u8,
}

impl DecomposedPosition {
    #[inline]
    pub fn new(room_x: u8, room_y: u8, x: u8, y: u8) -> Self {
        assert!(x < 50, "out of bounds x: {}", x);
        assert!(y < 50, "out of bounds y: {}", y);
        Self { room_x, room_y, x, y }
    }

    #[inline]
    fn from_room_idx(room_idx: u16) -> (u8, u8) {
        let room_x = (room_idx & 0xFF) as u8;
        let room_y = ((room_idx >> 8) & 0xFF) as u8;
        (room_x, room_y)
    }

    #[inline]
    fn to_room_idx(room_x: u8, room_y: u8) -> u16 {
        ((room_x as u16) << 8) | (room_y as u16)
    }

    #[inline]
    fn signed_to_offset(val: i8) -> u8 {
        (val as u8).wrapping_add(128)
    }

    #[inline]
    fn offset_to_signed(val: u8) -> i8 {
        (val as i8).wrapping_sub(-128_i8)
    }
}

impl fast_position for DecomposedPosition {
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
        Position::from_packed(((self.room_x as u32) << 24) |
                            ((self.room_y as u32) << 16) |
                            ((self.x as u32) << 8) |
                            (self.y as u32))
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        ((self.room_x as u32) << 24) |
        ((self.room_y as u32) << 16) |
        ((self.x as u32) << 8) |
        (self.y as u32)
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
        // SAFETY: x,y are always valid room coordinates (0-49)
        unsafe { RoomXY::unchecked_new(self.x, self.y) }
    }

    #[inline]
    fn x_major_room(&self) -> u16 {
        ((self.room_x as u16) << 8) | (self.room_y as u16)
    }

    #[inline]
    fn x_major_local(&self) -> u16 {
        ((self.x as u16) << 8) | (self.y as u16)
    }

    #[inline]
    fn x_major_global(&self) -> u32 {
        ((self.room_x as u32) << 24) |
        ((self.room_y as u32) << 16) |
        ((self.x as u32) << 8) |
        (self.y as u32)
    }

    #[inline]
    fn y_major_room(&self) -> u16 {
        ((self.room_y as u16) << 8) | (self.room_x as u16)
    }

    #[inline]
    fn y_major_local(&self) -> u16 {
        ((self.y as u16) << 8) | (self.x as u16)
    }

    #[inline]
    fn y_major_global(&self) -> u32 {
        ((self.room_y as u32) << 24) |
        ((self.room_x as u32) << 16) |
        ((self.y as u32) << 8) |
        (self.x as u32)
    }

    #[inline]
    fn z_order_room(&self) -> u16 {
        let mut z = 0u16;
        for i in 0..8 {
            z |= ((self.room_x & (1 << i)) as u16) << i | ((self.room_y & (1 << i)) as u16) << (i + 1);
        }
        z
    }

    #[inline]
    fn z_order_local(&self) -> u16 {
        let mut z = 0u16;
        for i in 0..8 {
            z |= ((self.x & (1 << i)) as u16) << i | ((self.y & (1 << i)) as u16) << (i + 1);
        }
        z
    }

    #[inline]
    fn z_order_global(&self) -> u32 {
        let mut z = 0u32;
        for i in 0..8 {
            z |= ((self.room_x & (1 << i)) as u32) << (4*i) |
                 ((self.room_y & (1 << i)) as u32) << (4*i + 1) |
                 ((self.x & (1 << i)) as u32) << (4*i + 2) |
                 ((self.y & (1 << i)) as u32) << (4*i + 3);
        }
        z
    }

    #[inline]
    fn hilbert_room(&self) -> u16 {
        let mut x = self.room_x as u16;
        let mut y = self.room_y as u16;
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
        let mut pos_x = self.x as u16;
        let mut pos_y = self.y as u16;
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
        let mut pos_x = ((self.room_x as u16) << 8) | (self.x as u16);
        let mut pos_y = ((self.room_y as u16) << 8) | (self.y as u16);
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
        (self.room_x, self.room_y, self.x, self.y)
    }

    #[inline]
    fn room_name(&self) -> RoomName {
        // Convert from offset coordinates to signed for room name
        let signed_x = (self.room_x as i8).wrapping_sub(-128_i8);
        let signed_y = (self.room_y as i8).wrapping_sub(-128_i8);
        let h = if signed_x < 0 { 'W' } else { 'E' };
        let v = if signed_y < 0 { 'N' } else { 'S' };
        let x = signed_x.unsigned_abs() as u32;
        let y = signed_y.unsigned_abs() as u32;
        RoomName::new(&format!("{}{}{}{}", h, x, v, y)).unwrap()
    }

    #[inline]
    fn neighbor_in_dir(&self, dir: Direction) -> Self {
        let (final_x, final_y, new_room_x, new_room_y) = match dir {
            Direction::Top => {
                if self.y == 0 {
                    (self.x, 49, self.room_x, self.room_y.wrapping_sub(1))
                } else {
                    (self.x, self.y.wrapping_sub(1), self.room_x, self.room_y)
                }
            }
            Direction::TopRight => {
                if self.y == 0 && self.x == 49 {
                    (0, 49, self.room_x.wrapping_add(1), self.room_y.wrapping_sub(1))
                } else if self.y == 0 {
                    (self.x.wrapping_add(1), 49, self.room_x, self.room_y.wrapping_sub(1))
                } else if self.x == 49 {
                    (0, self.y.wrapping_sub(1), self.room_x.wrapping_add(1), self.room_y)
                } else {
                    (self.x.wrapping_add(1), self.y.wrapping_sub(1), self.room_x, self.room_y)
                }
            }
            Direction::Right => {
                if self.x == 49 {
                    (0, self.y, self.room_x.wrapping_add(1), self.room_y)
                } else {
                    (self.x.wrapping_add(1), self.y, self.room_x, self.room_y)
                }
            }
            Direction::BottomRight => {
                if self.y == 49 && self.x == 49 {
                    (0, 0, self.room_x.wrapping_add(1), self.room_y.wrapping_add(1))
                } else if self.y == 49 {
                    (self.x.wrapping_add(1), 0, self.room_x, self.room_y.wrapping_add(1))
                } else if self.x == 49 {
                    (0, self.y.wrapping_add(1), self.room_x.wrapping_add(1), self.room_y)
                } else {
                    (self.x.wrapping_add(1), self.y.wrapping_add(1), self.room_x, self.room_y)
                }
            }
            Direction::Bottom => {
                if self.y == 49 {
                    (self.x, 0, self.room_x, self.room_y.wrapping_add(1))
                } else {
                    (self.x, self.y.wrapping_add(1), self.room_x, self.room_y)
                }
            }
            Direction::BottomLeft => {
                if self.y == 49 && self.x == 0 {
                    (49, 0, self.room_x.wrapping_sub(1), self.room_y.wrapping_add(1))
                } else if self.y == 49 {
                    (self.x.wrapping_sub(1), 0, self.room_x, self.room_y.wrapping_add(1))
                } else if self.x == 0 {
                    (49, self.y.wrapping_add(1), self.room_x.wrapping_sub(1), self.room_y)
                } else {
                    (self.x.wrapping_sub(1), self.y.wrapping_add(1), self.room_x, self.room_y)
                }
            }
            Direction::Left => {
                if self.x == 0 {
                    (49, self.y, self.room_x.wrapping_sub(1), self.room_y)
                } else {
                    (self.x.wrapping_sub(1), self.y, self.room_x, self.room_y)
                }
            }
            Direction::TopLeft => {
                if self.y == 0 && self.x == 0 {
                    (49, 49, self.room_x.wrapping_sub(1), self.room_y.wrapping_sub(1))
                } else if self.y == 0 {
                    (self.x.wrapping_sub(1), 49, self.room_x, self.room_y.wrapping_sub(1))
                } else if self.x == 0 {
                    (49, self.y.wrapping_sub(1), self.room_x.wrapping_sub(1), self.room_y)
                } else {
                    (self.x.wrapping_sub(1), self.y.wrapping_sub(1), self.room_x, self.room_y)
                }
            }
        };

        Self {
            room_x: new_room_x,
            room_y: new_room_y,
            x: final_x,
            y: final_y,
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
        let fast = DecomposedPosition::from_position(original);
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_xy() {
        let pos = DecomposedPosition::from_position(
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

    #[test]
    fn test_room_idx_roundtrip() {
        let room_x = 128u8;
        let room_y = 128u8;
        let room_idx = DecomposedPosition::to_room_idx(room_x, room_y);
        let (rx, ry) = DecomposedPosition::from_room_idx(room_idx);
        assert_eq!(rx, room_x);
        assert_eq!(ry, room_y);
    }
} 