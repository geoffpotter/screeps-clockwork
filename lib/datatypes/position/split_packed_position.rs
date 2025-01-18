use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::TryFrom;
use super::fast_position;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SplitPosition {
    room_idx: u16,
    x: u8,
    y: u8,
}

impl SplitPosition {
    #[inline]
    pub fn new(room_x: u8, room_y: u8, x: u8, y: u8) -> Self {
        assert!(x < 50, "out of bounds x: {}", x);
        assert!(y < 50, "out of bounds y: {}", y);
        let room_idx = Self::compose_room_idx(room_x, room_y);
        Self { room_idx, x, y }
    }

    #[inline]
    fn decompose_room_idx(room_idx: u16) -> (u8, u8) {
        let room_x = (room_idx & 0xFF) as u8;
        let room_y = ((room_idx >> 8) & 0xFF) as u8;
        (room_x, room_y)
    }

    #[inline]
    fn compose_room_idx(room_x: u8, room_y: u8) -> u16 {
        (room_x as u16) | ((room_y as u16) << 8)
    }
}

impl fast_position for SplitPosition {
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
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        Position::from_packed(((room_x as u32) << 24) |
                            ((room_y as u32) << 16) |
                            ((self.x as u32) << 8) |
                            (self.y as u32))
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        ((room_x as u32) << 24) |
        ((room_y as u32) << 16) |
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
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        (room_x as u16) | ((room_y as u16) << 8)
    }

    #[inline]
    fn x_major_local(&self) -> u16 {
        ((self.x as u16) << 8) | (self.y as u16)
    }

    #[inline]
    fn x_major_global(&self) -> u32 {
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        ((room_x as u32) << 24) |
        ((room_y as u32) << 16) |
        ((self.x as u32) << 8) |
        (self.y as u32)
    }

    #[inline]
    fn y_major_room(&self) -> u16 {
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        ((room_y as u16) << 8) | (room_x as u16)
    }

    #[inline]
    fn y_major_local(&self) -> u16 {
        ((self.y as u16) << 8) | (self.x as u16)
    }

    #[inline]
    fn y_major_global(&self) -> u32 {
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        ((room_y as u32) << 24) |
        ((room_x as u32) << 16) |
        ((self.y as u32) << 8) |
        (self.x as u32)
    }

    #[inline]
    fn z_order_room(&self) -> u16 {
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        let mut z = 0u16;
        for i in 0..8 {
            z |= ((room_x & (1 << i)) as u16) << i | ((room_y & (1 << i)) as u16) << (i + 1);
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
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        let mut z = 0u32;
        for i in 0..8 {
            z |= ((room_x & (1 << i)) as u32) << (4*i) |
                 ((room_y & (1 << i)) as u32) << (4*i + 1) |
                 ((self.x & (1 << i)) as u32) << (4*i + 2) |
                 ((self.y & (1 << i)) as u32) << (4*i + 3);
        }
        z
    }

    #[inline]
    fn hilbert_room(&self) -> u16 {
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
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
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        let mut pos_x = ((room_x as u16) << 8) | (self.x as u16);
        let mut pos_y = ((room_y as u16) << 8) | (self.y as u16);
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
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        (room_x, room_y, self.x, self.y)
    }

    #[inline]
    fn room_name(&self) -> RoomName {
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
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
        let (room_x, room_y) = Self::decompose_room_idx(self.room_idx);
        let x = self.x;
        let y = self.y;
        
        let (final_x, final_y, new_room_x, new_room_y) = match dir {
            Direction::Top => {
                if y == 0 {
                    (x, 49, room_x, room_y.wrapping_sub(1))
                } else {
                    (x, y.wrapping_sub(1), room_x, room_y)
                }
            }
            Direction::TopRight => {
                if y == 0 && x == 49 {
                    (0, 49, room_x.wrapping_add(1), room_y.wrapping_sub(1))
                } else if y == 0 {
                    (x.wrapping_add(1), 49, room_x, room_y.wrapping_sub(1))
                } else if x == 49 {
                    (0, y.wrapping_sub(1), room_x.wrapping_add(1), room_y)
                } else {
                    (x.wrapping_add(1), y.wrapping_sub(1), room_x, room_y)
                }
            }
            Direction::Right => {
                if x == 49 {
                    (0, y, room_x.wrapping_add(1), room_y)
                } else {
                    (x.wrapping_add(1), y, room_x, room_y)
                }
            }
            Direction::BottomRight => {
                if y == 49 && x == 49 {
                    (0, 0, room_x.wrapping_add(1), room_y.wrapping_add(1))
                } else if y == 49 {
                    (x.wrapping_add(1), 0, room_x, room_y.wrapping_add(1))
                } else if x == 49 {
                    (0, y.wrapping_add(1), room_x.wrapping_add(1), room_y)
                } else {
                    (x.wrapping_add(1), y.wrapping_add(1), room_x, room_y)
                }
            }
            Direction::Bottom => {
                if y == 49 {
                    (x, 0, room_x, room_y.wrapping_add(1))
                } else {
                    (x, y.wrapping_add(1), room_x, room_y)
                }
            }
            Direction::BottomLeft => {
                if y == 49 && x == 0 {
                    (49, 0, room_x.wrapping_sub(1), room_y.wrapping_add(1))
                } else if y == 49 {
                    (x.wrapping_sub(1), 0, room_x, room_y.wrapping_add(1))
                } else if x == 0 {
                    (49, y.wrapping_add(1), room_x.wrapping_sub(1), room_y)
                } else {
                    (x.wrapping_sub(1), y.wrapping_add(1), room_x, room_y)
                }
            }
            Direction::Left => {
                if x == 0 {
                    (49, y, room_x.wrapping_sub(1), room_y)
                } else {
                    (x.wrapping_sub(1), y, room_x, room_y)
                }
            }
            Direction::TopLeft => {
                if y == 0 && x == 0 {
                    (49, 49, room_x.wrapping_sub(1), room_y.wrapping_sub(1))
                } else if y == 0 {
                    (x.wrapping_sub(1), 49, room_x, room_y.wrapping_sub(1))
                } else if x == 0 {
                    (49, y.wrapping_sub(1), room_x.wrapping_sub(1), room_y)
                } else {
                    (x.wrapping_sub(1), y.wrapping_sub(1), room_x, room_y)
                }
            }
        };

        Self {
            room_idx: Self::compose_room_idx(new_room_x, new_room_y),
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
        let fast = SplitPosition::from_position(original);
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_xy() {
        let pos = SplitPosition::from_position(
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