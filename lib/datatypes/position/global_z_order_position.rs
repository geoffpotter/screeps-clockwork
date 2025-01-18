use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::{TryInto, TryFrom};
use super::fast_position;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct GlobalZOrderPosition {
    packed: u32,
}

impl GlobalZOrderPosition {
    #[inline]
    pub fn new(room_x: u8, room_y: u8, x: u8, y: u8) -> Self {
        // Convert room coordinates to global coordinates (already unsigned)
        let global_x = ((room_x as u16) * 50) + x as u16;
        let global_y = ((room_y as u16) * 50) + y as u16;
        
        // Calculate z-order value by interleaving bits
        let mut z = 0u32;
        for i in 0..16 {
            let x_bit = (global_x >> i) & 1;
            let y_bit = (global_y >> i) & 1;
            z |= ((x_bit as u32) << (2 * i)) | ((y_bit as u32) << (2 * i + 1));
        }
        
        Self { packed: z }
    }

    #[inline]
    fn decompose_global(&self) -> (u8, u8, u8, u8) {
        let mut x = 0u16;
        let mut y = 0u16;
        let mut t = self.packed;

        // Extract x,y from z-order value by deinterleaving bits
        for i in 0..16 {
            x |= ((t & (1 << (2 * i))) >> i) as u16;
            y |= ((t & (1 << (2 * i + 1))) >> (i + 1)) as u16;
        }

        // Convert global coordinates back to room + local using division and modulo
        let room_x = (x / 50) as u8;
        let room_y = (y / 50) as u8;
        let local_x = (x % 50) as u8;
        let local_y = (y % 50) as u8;

        (room_x, room_y, local_x, local_y)
    }
}

impl fast_position for GlobalZOrderPosition {
    #[inline]
    fn new(packed: u32) -> Self {
        Self::from_packed_repr(packed)
    }

    #[inline]
    fn from_position(position: Position) -> Self {
        let packed = position.packed_repr();
        let room_idx = (packed >> 16) & 0xFFFF;
        let room_x = (room_idx & 0xFF) as u8;
        let room_y = ((room_idx >> 8) & 0xFF) as u8;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        Self::new(room_x, room_y, x, y)
    }

    #[inline]
    fn to_position(&self) -> Position {
        let (room_x, room_y, x, y) = self.decompose_global();
        let room_idx = ((room_x as u16) | 
                      (((room_y as u16) << 8)));
        let packed = ((room_idx as u32) << 16) | ((x as u32) << 8) | (y as u32);
        Position::from_packed(packed)
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        let room_idx = ((room_x as u16) | 
                      (((room_y as u16) << 8)));
        ((room_idx as u32) << 16) | ((x as u32) << 8) | (y as u32)
    }

    #[inline]
    fn from_packed_repr(packed: u32) -> Self {
        Self::from_position(Position::from_packed(packed))
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
        ((room_x as u16) | 
        (((room_y as u16) << 8)))
    }

    #[inline]
    fn x_major_local(&self) -> u16 {
        let (_, _, x, y) = self.decompose_global();
        ((x as u16) << 8) | (y as u16)
    }

    #[inline]
    fn x_major_global(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        ((room_x as u32) << 24) |
        ((room_y as u32) << 16) |
        ((x as u32) << 8) |
        (y as u32)
    }

    #[inline]
    fn y_major_room(&self) -> u16 {
        let (room_x, room_y, _, _) = self.decompose_global();
        ((room_y as u16) << 8) | 
        ((room_x as u16))
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
        self.packed
    }

    #[inline]
    fn hilbert_room(&self) -> u16 {
        let (room_x, room_y, _, _) = self.decompose_global();
        let mut x = room_x as u16;
        let mut y = room_y as u16;
        let mut d = 0u16;
        let mut rx: u16;
        let mut ry: u16;
        let mut s: u16 = 128; // Start with largest quadrant
        
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
        let mut s: u16 = 64; // Start with largest quadrant for 50x50 room
        
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
        let mut global_x = ((self.packed >> 16) & 0xFFFF) as u16;
        let mut global_y = (self.packed & 0xFFFF) as u16;
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
        // Convert from offset coordinates back to signed for room name
        let signed_x = (room_x as i16).wrapping_sub(128);
        let signed_y = (room_y as i16).wrapping_sub(128);
        let h = if signed_x < 0 { 'W' } else { 'E' };
        let v = if signed_y < 0 { 'N' } else { 'S' };
        let x = if signed_x < 0 { (-signed_x) as u32 } else { signed_x as u32 };
        let y = if signed_y < 0 { (-signed_y) as u32 } else { signed_y as u32 };
        RoomName::new(&format!("{}{}{}{}", h, x, v, y)).unwrap()
    }

    #[inline]
    fn neighbor_in_dir(&self, dir: Direction) -> Self {
        // Extract x,y from z-order value by deinterleaving bits
        let mut global_x = 0u16;
        let mut global_y = 0u16;
        let mut t = self.packed;

        for i in 0..16 {
            global_x |= ((t & (1 << (2 * i))) >> i) as u16;
            global_y |= ((t & (1 << (2 * i + 1))) >> (i + 1)) as u16;
        }

        let local_x = (global_x % 50) as u8;
        let local_y = (global_y % 50) as u8;
        
        let (final_x, final_y) = match dir {
            Direction::Top => {
                if local_y == 0 {
                    // Moving up across room boundary
                    (global_x, global_y.wrapping_sub(1))
                } else {
                    (global_x, global_y.wrapping_sub(1))
                }
            }
            Direction::TopRight => {
                if local_y == 0 && local_x == 49 {
                    // Moving up and right across room boundary
                    (global_x.wrapping_add(1), global_y.wrapping_sub(1))
                } else if local_y == 0 {
                    (global_x.wrapping_add(1), global_y.wrapping_sub(1))
                } else if local_x == 49 {
                    (global_x.wrapping_add(1), global_y.wrapping_sub(1))
                } else {
                    (global_x.wrapping_add(1), global_y.wrapping_sub(1))
                }
            }
            Direction::Right => {
                if local_x == 49 {
                    // Moving right across room boundary
                    (global_x.wrapping_add(1), global_y)
                } else {
                    (global_x.wrapping_add(1), global_y)
                }
            }
            Direction::BottomRight => {
                if local_y == 49 && local_x == 49 {
                    // Moving down and right across room boundary
                    (global_x.wrapping_add(1), global_y.wrapping_add(1))
                } else if local_y == 49 {
                    (global_x.wrapping_add(1), global_y.wrapping_add(1))
                } else if local_x == 49 {
                    (global_x.wrapping_add(1), global_y.wrapping_add(1))
                } else {
                    (global_x.wrapping_add(1), global_y.wrapping_add(1))
                }
            }
            Direction::Bottom => {
                if local_y == 49 {
                    // Moving down across room boundary
                    (global_x, global_y.wrapping_add(1))
                } else {
                    (global_x, global_y.wrapping_add(1))
                }
            }
            Direction::BottomLeft => {
                if local_y == 49 && local_x == 0 {
                    // Moving down and left across room boundary
                    (global_x.wrapping_sub(1), global_y.wrapping_add(1))
                } else if local_y == 49 {
                    (global_x.wrapping_sub(1), global_y.wrapping_add(1))
                } else if local_x == 0 {
                    (global_x.wrapping_sub(1), global_y.wrapping_add(1))
                } else {
                    (global_x.wrapping_sub(1), global_y.wrapping_add(1))
                }
            }
            Direction::Left => {
                if local_x == 0 {
                    // Moving left across room boundary
                    (global_x.wrapping_sub(1), global_y)
                } else {
                    (global_x.wrapping_sub(1), global_y)
                }
            }
            Direction::TopLeft => {
                if local_y == 0 && local_x == 0 {
                    // Moving up and left across room boundary
                    (global_x.wrapping_sub(1), global_y.wrapping_sub(1))
                } else if local_y == 0 {
                    (global_x.wrapping_sub(1), global_y.wrapping_sub(1))
                } else if local_x == 0 {
                    (global_x.wrapping_sub(1), global_y.wrapping_sub(1))
                } else {
                    (global_x.wrapping_sub(1), global_y.wrapping_sub(1))
                }
            }
        };

        // Pack back into z-order format
        let mut z = 0u32;
        for i in 0..16 {
            let x_bit = (final_x >> i) & 1;
            let y_bit = (final_y >> i) & 1;
            z |= ((x_bit as u32) << (2 * i)) | ((y_bit as u32) << (2 * i + 1));
        }
        Self { packed: z }
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
        let fast = GlobalZOrderPosition::from_position(original);
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_xy() {
        let pos = GlobalZOrderPosition::from_position(
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