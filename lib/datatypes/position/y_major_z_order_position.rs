use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::{TryInto, TryFrom};
use super::fast_position;
use lazy_static::lazy_static;

lazy_static! {
    // Lookup tables for deinterleaving
    static ref DEINTERLEAVE_X: [u8; 256] = {
        let mut table = [0u8; 256];
        for i in 0..256 {
            let mut x = 0u8;
            for j in 0..8 {
                x |= ((i & (1 << (j * 2))) >> j) as u8;
            }
            table[i] = x;
        }
        table
    };

    static ref DEINTERLEAVE_Y: [u8; 256] = {
        let mut table = [0u8; 256];
        for i in 0..256 {
            let mut y = 0u8;
            for j in 0..8 {
                y |= ((i & (1 << (j * 2 + 1))) >> (j + 1)) as u8;
            }
            table[i] = y;
        }
        table
    };

    // Lookup table for interleaving
    static ref INTERLEAVE: [[u16; 256]; 256] = {
        let mut table = [[0u16; 256]; 256];
        for x in 0..256 {
            for y in 0..256 {
                let mut z = 0u16;
                for i in 0..8 {
                    z |= ((x & (1 << i)) as u16) << i;
                    z |= ((y & (1 << i)) as u16) << (i + 1);
                }
                table[x as usize][y as usize] = z;
            }
        }
        table
    };
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ZOrderPosition {
    // Top 16 bits: room_idx y-major index (y * WIDTH + x)
    // Bottom 16 bits: z-order curve value
    packed: u32,
}

impl ZOrderPosition {
    #[inline]
    pub fn new(x: u8, y: u8, room_idx: u16) -> Self {
        Self {
            packed: ((room_idx as u32) << 16) | (INTERLEAVE[x as usize][y as usize] as u32)
        }
    }

    #[inline]
    fn deinterleave(z: u16) -> (u8, u8) {
        let mut x = 0u8;
        let mut y = 0u8;
        let mut i = 0;
        let mut z = z;

        while z != 0 {
            x |= ((z & 1) as u8) << i;
            z >>= 1;
            y |= ((z & 1) as u8) << i;
            z >>= 1;
            i += 1;
        }
        (x, y)
    }

    #[inline]
    fn interleave(x: u8, y: u8) -> u16 {
        let mut z = 0u16;
        for i in 0..8 {
            z |= (((x >> i) & 1) as u16) << (2 * i);
            z |= (((y >> i) & 1) as u16) << (2 * i + 1);
        }
        z
    }

    #[inline]
    fn decompose_room_idx(room_idx: u16) -> (u8, u8) {
        // For a 256x256 room grid
        let x = (room_idx % 256) as u8;
        let y = (room_idx / 256) as u8;
        (x, y)
    }

    #[inline]
    fn compose_room_idx(room_x: u8, room_y: u8) -> u16 {
        // Convert to y-major index (y * WIDTH + x)
        ((room_y as u16) * 256) + (room_x as u16)
    }
}

impl fast_position for ZOrderPosition {
    #[inline]
    fn new(packed: u32) -> Self {
        Self::from_packed_repr(packed)
    }

    #[inline]
    fn from_position(position: Position) -> Self {
        let packed = position.packed_repr();
        let room_idx = (packed >> 16) & 0xFFFF;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        Self::new(x, y, room_idx as u16)
    }

    #[inline]
    fn to_position(&self) -> Position {
        let room_idx = (self.packed >> 16) & 0xFFFF;
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        let packed = (room_idx << 16) | ((x as u32) << 8) | (y as u32);
        Position::from_packed(packed)
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        let room_idx = (self.packed >> 16) & 0xFFFF;
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        (room_idx << 16) | ((x as u32) << 8) | (y as u32)
    }

    #[inline]
    fn from_packed_repr(packed: u32) -> Self {
        let room_idx = (packed >> 16) & 0xFFFF;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        Self::new(x, y, room_idx as u16)
    }

    #[inline]
    fn xy(&self) -> RoomXY {
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        // SAFETY: x,y are always valid room coordinates
        unsafe { RoomXY::unchecked_new(x, y) }
    }

    #[inline]
    fn neighbor_in_dir(&self, dir: Direction) -> Self {
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        let room_idx = ((self.packed >> 16) & 0xFFFF) as u16;
        
        let (final_x, final_y, final_room_idx) = match dir {
            Direction::Top => {
                if y == 0 {
                    (x, 49, room_idx.wrapping_sub(256))
                } else {
                    (x, y.wrapping_sub(1), room_idx)
                }
            }
            Direction::TopRight => {
                if y == 0 && x == 49 {
                    (0, 49, room_idx.wrapping_sub(255))
                } else if y == 0 {
                    (x.wrapping_add(1), 49, room_idx.wrapping_sub(256))
                } else if x == 49 {
                    (0, y.wrapping_sub(1), room_idx.wrapping_add(1))
                } else {
                    (x.wrapping_add(1), y.wrapping_sub(1), room_idx)
                }
            }
            Direction::Right => {
                if x == 49 {
                    (0, y, room_idx.wrapping_add(1))
                } else {
                    (x.wrapping_add(1), y, room_idx)
                }
            }
            Direction::BottomRight => {
                if y == 49 && x == 49 {
                    (0, 0, room_idx.wrapping_add(257))
                } else if y == 49 {
                    (x.wrapping_add(1), 0, room_idx.wrapping_add(256))
                } else if x == 49 {
                    (0, y.wrapping_add(1), room_idx.wrapping_add(1))
                } else {
                    (x.wrapping_add(1), y.wrapping_add(1), room_idx)
                }
            }
            Direction::Bottom => {
                if y == 49 {
                    (x, 0, room_idx.wrapping_add(256))
                } else {
                    (x, y.wrapping_add(1), room_idx)
                }
            }
            Direction::BottomLeft => {
                if y == 49 && x == 0 {
                    (49, 0, room_idx.wrapping_add(255))
                } else if y == 49 {
                    (x.wrapping_sub(1), 0, room_idx.wrapping_add(256))
                } else if x == 0 {
                    (49, y.wrapping_add(1), room_idx.wrapping_sub(1))
                } else {
                    (x.wrapping_sub(1), y.wrapping_add(1), room_idx)
                }
            }
            Direction::Left => {
                if x == 0 {
                    (49, y, room_idx.wrapping_sub(1))
                } else {
                    (x.wrapping_sub(1), y, room_idx)
                }
            }
            Direction::TopLeft => {
                if y == 0 && x == 0 {
                    (49, 49, room_idx.wrapping_sub(257))
                } else if y == 0 {
                    (x.wrapping_sub(1), 49, room_idx.wrapping_sub(256))
                } else if x == 0 {
                    (49, y.wrapping_sub(1), room_idx.wrapping_sub(1))
                } else {
                    (x.wrapping_sub(1), y.wrapping_sub(1), room_idx)
                }
            }
        };

        let local_z = Self::interleave(final_x, final_y);
        Self {
            packed: ((final_room_idx as u32) << 16) | (local_z as u32)
        }
    }

    #[inline]
    fn x_major_room(&self) -> u16 {
        let room = (self.packed >> 16) as u16;
        let (room_x, room_y) = Self::decompose_room_idx(room);
        // Convert from y-major to x-major
        ((room_x as u16) << 8) | (room_y as u16)
    }

    #[inline]
    fn x_major_local(&self) -> u16 {
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        ((x as u16) << 8) | (y as u16)
    }

    #[inline]
    fn x_major_global(&self) -> u32 {
        let room_idx = (self.packed >> 16) as u16;
        let (room_x, room_y) = Self::decompose_room_idx(room_idx);
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        ((room_x as u32) << 24) |
        ((room_y as u32) << 16) |
        (x as u32) << 8 |
        (y as u32)
    }

    #[inline]
    fn y_major_room(&self) -> u16 {
        (self.packed >> 16) as u16  // Already in y-major order
    }

    #[inline]
    fn y_major_local(&self) -> u16 {
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        ((y as u16) << 8) | (x as u16)
    }

    #[inline]
    fn y_major_global(&self) -> u32 {
        let room_idx = (self.packed >> 16) as u16;
        let (room_x, room_y) = Self::decompose_room_idx(room_idx);
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        ((room_y as u32) << 24) |
        ((room_x as u32) << 16) |
        (y as u32) << 8 |
        (x as u32)
    }

    #[inline]
    fn z_order_room(&self) -> u16 {
        let room_idx = (self.packed >> 16) as u16;
        let (room_x, room_y) = Self::decompose_room_idx(room_idx);
        INTERLEAVE[room_x as usize][room_y as usize]
    }

    #[inline]
    fn z_order_local(&self) -> u16 {
        (self.packed & 0xFFFF) as u16
    }

    #[inline]
    fn z_order_global(&self) -> u32 {
        let room_idx = (self.packed >> 16) as u16;
        let (room_x, room_y) = Self::decompose_room_idx(room_idx);
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
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
        let room = (self.packed >> 16) as u16;
        let mut x = room >> 8;
        let mut y = room & 0xFF;
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
        let local = (self.packed & 0xFFFF) as u16;
        let mut x = local >> 8;
        let mut y = local & 0xFF;
        let mut d = 0u16;
        let mut rx: u16;
        let mut ry: u16;
        let mut s: u16 = 64; // Start with largest quadrant for 50x50 room
        
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
    fn hilbert_global(&self) -> u32 {
        let room = (self.packed >> 16) as u16;
        let local = (self.packed & 0xFFFF) as u16;
        let room_x = room >> 8;
        let room_y = room & 0xFF;
        let x = local >> 8;
        let y = local & 0xFF;
        let mut pos_x = ((room_x) << 8) | x;
        let mut pos_y = ((room_y) << 8) | y;
        let mut d = 0u32;
        let mut rx: u16;
        let mut ry: u16;
        let mut s: u16 = 32768; // Start with largest quadrant
        
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
        let room_idx = (self.packed >> 16) as u16;
        let (room_x, room_y) = Self::decompose_room_idx(room_idx);
        let (x, y) = Self::deinterleave((self.packed & 0xFFFF) as u16);
        (room_x, room_y, x, y)
    }

    #[inline]
    fn room_name(&self) -> RoomName {
        let (room_x, room_y, _, _) = self.decomposed();
        // Convert from offset coordinates back to signed for room names
        let signed_x = (room_x as i16).wrapping_sub(128);
        let signed_y = (room_y as i16).wrapping_sub(128);
        let h = if signed_x < 0 { 'W' } else { 'E' };
        let v = if signed_y < 0 { 'N' } else { 'S' };
        let x = signed_x.abs() as u32;
        let y = signed_y.abs() as u32;
        RoomName::new(&format!("{}{}{}{}", h, x, v, y)).unwrap()
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
        let fast = ZOrderPosition::from_position(original);
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_xy() {
        let pos = ZOrderPosition::from_position(
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
    fn test_interleave_deinterleave() {
        let x = 25u8;
        let y = 30u8;
        let pos = ZOrderPosition::new(x, y, 0);
        let (dx, dy) = ZOrderPosition::deinterleave(pos.z_order_local());
        assert_eq!(x, dx);
        assert_eq!(y, dy);
    }
} 