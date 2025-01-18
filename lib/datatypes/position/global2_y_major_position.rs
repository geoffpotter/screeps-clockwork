use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::{TryInto, TryFrom};
use super::fast_position;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct GlobalYMajorPosition {
    // Stores global coordinates in y-major order:
    // - Top 16 bits: global_y (room_y * 50 + y)
    // - Bottom 16 bits: global_x (room_x * 50 + x)
    // This version uses a different room coordinate system where (0,0) is at the center
    packed: u32,
}

impl GlobalYMajorPosition {
    #[inline]
    pub fn new(room_x: i8, room_y: i8, x: u8, y: u8) -> Self {
        assert!(x < 50, "out of bounds x: {}", x);
        assert!(y < 50, "out of bounds y: {}", y);
        
        // Convert room coordinates to global coordinates
        // Add 128 to room coordinates to make them unsigned (0-255)
        // But do it in a way that won't overflow u16 when multiplied by 50
        let unsigned_room_x = (room_x as i16).wrapping_add(128) as u8;
        let unsigned_room_y = (room_y as i16).wrapping_add(128) as u8;
        
        // Calculate global coordinates carefully to avoid overflow
        let global_x = ((unsigned_room_x as u16) * 50 + x as u16) & 0xFFFF;
        let global_y = ((unsigned_room_y as u16) * 50 + y as u16) & 0xFFFF;
        
        Self { 
            packed: ((global_y as u32) << 16) | (global_x as u32)
        }
    }

    #[inline]
    fn decompose_global(&self) -> (i8, i8, u8, u8) {
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let global_x = (self.packed & 0xFFFF) as u16;
        
        let unsigned_room_x = (global_x / 50) as u8;
        let unsigned_room_y = (global_y / 50) as u8;
        
        // Convert back to signed room coordinates (-128 to 127)
        // Use i16 for intermediate calculation to avoid overflow
        let room_x = ((unsigned_room_x as i16).wrapping_sub(128)).clamp(-128, 127) as i8;
        let room_y = ((unsigned_room_y as i16).wrapping_sub(128)).clamp(-128, 127) as i8;
        
        // Ensure local coordinates are within bounds (0-49)
        let local_x = ((global_x % 50) as u8).min(49);
        let local_y = ((global_y % 50) as u8).min(49);
        
        (room_x, room_y, local_x, local_y)
    }
}

impl fast_position for GlobalYMajorPosition {
    #[inline]
    fn new(packed: u32) -> Self {
        Self::from_packed_repr(packed)
    }

    #[inline]
    fn from_position(position: Position) -> Self {
        let packed = position.packed_repr();
        let room_x = ((packed >> 24) & 0xFF) as i8;
        let room_y = ((packed >> 16) & 0xFF) as i8;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        
        // Clamp coordinates to valid ranges
        let room_x = room_x.clamp(-128, 127);
        let room_y = room_y.clamp(-128, 127);
        let x = x.min(49);
        let y = y.min(49);
        
        Self::new(room_x, room_y, x, y)
    }

    #[inline]
    fn to_position(&self) -> Position {
        let (room_x, room_y, x, y) = self.decompose_global();
        
        // Clamp coordinates to valid ranges
        let room_x = room_x.clamp(-128, 127);
        let room_y = room_y.clamp(-128, 127);
        let x = x.min(49);
        let y = y.min(49);
        
        // Convert to unsigned room coordinates for packing
        let room_x_unsigned = (room_x as u8);
        let room_y_unsigned = (room_y as u8);
        
        let packed = ((room_x_unsigned as u32) << 24) | 
                    ((room_y_unsigned as u32) << 16) | 
                    ((x as u32) << 8) | 
                    (y as u32);
        Position::from_packed(packed)
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        // Clamp coordinates to valid ranges
        let room_x = room_x.clamp(-128, 127);
        let room_y = room_y.clamp(-128, 127);
        let x = x.min(49);
        let y = y.min(49);
        
        // Convert to unsigned room coordinates for packing
        let room_x_unsigned = (room_x as u8);
        let room_y_unsigned = (room_y as u8);
        
        ((room_x_unsigned as u32) << 24) | 
        ((room_y_unsigned as u32) << 16) | 
        ((x as u32) << 8) | 
        (y as u32)
    }

    #[inline]
    fn from_packed_repr(packed: u32) -> Self {
        Self::from_position(Position::from_packed(packed))
    }

    #[inline]
    fn xy(&self) -> RoomXY {
        let global_x = (self.packed & 0xFFFF) as u16;
        let local_x = (global_x % 50) as u8;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let local_y = (global_y % 50) as u8;
        // SAFETY: x,y are always valid room coordinates (0-49)
        unsafe { RoomXY::unchecked_new(local_x, local_y) }
    }

    #[inline]
    fn x_major_room(&self) -> u16 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let unsigned_room_x = (global_x / 50) as u8;
        let unsigned_room_y = (global_y / 50) as u8;
        ((unsigned_room_x as u16) << 8) | (unsigned_room_y as u16)
    }

    #[inline]
    fn x_major_local(&self) -> u16 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let local_x = (global_x % 50) as u8;
        let local_y = (global_y % 50) as u8;
        ((local_x as u16) << 8) | (local_y as u16)
    }

    #[inline]
    fn x_major_global(&self) -> u32 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        ((global_x as u32) << 16) | (global_y as u32)
    }

    #[inline]
    fn y_major_room(&self) -> u16 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let unsigned_room_x = (global_x / 50) as u8;
        let unsigned_room_y = (global_y / 50) as u8;
        ((unsigned_room_y as u16) << 8) | (unsigned_room_x as u16)
    }

    #[inline]
    fn y_major_local(&self) -> u16 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let local_x = (global_x % 50) as u8;
        let local_y = (global_y % 50) as u8;
        ((local_y as u16) << 8) | (local_x as u16)
    }

    #[inline]
    fn y_major_global(&self) -> u32 {
        self.packed
    }

    #[inline]
    fn neighbor_in_dir(&self, dir: Direction) -> Self {
        let (room_x, room_y, x, y) = self.decompose_global();
        
        // Calculate new coordinates based on direction
        let (new_x, new_y, new_room_x, new_room_y) = match dir {
            Direction::Top => {
                if y == 0 {
                    (x, 49, room_x, room_y.wrapping_sub(1))
                } else {
                    (x, y - 1, room_x, room_y)
                }
            }
            Direction::TopRight => {
                if x == 49 && y == 0 {
                    (0, 49, room_x.wrapping_add(1), room_y.wrapping_sub(1))
                } else if x == 49 {
                    (0, y - 1, room_x.wrapping_add(1), room_y)
                } else if y == 0 {
                    (x + 1, 49, room_x, room_y.wrapping_sub(1))
                } else {
                    (x + 1, y - 1, room_x, room_y)
                }
            }
            Direction::Right => {
                if x == 49 {
                    (0, y, room_x.wrapping_add(1), room_y)
                } else {
                    (x + 1, y, room_x, room_y)
                }
            }
            Direction::BottomRight => {
                if x == 49 && y == 49 {
                    (0, 0, room_x.wrapping_add(1), room_y.wrapping_add(1))
                } else if x == 49 {
                    (0, y + 1, room_x.wrapping_add(1), room_y)
                } else if y == 49 {
                    (x + 1, 0, room_x, room_y.wrapping_add(1))
                } else {
                    (x + 1, y + 1, room_x, room_y)
                }
            }
            Direction::Bottom => {
                if y == 49 {
                    (x, 0, room_x, room_y.wrapping_add(1))
                } else {
                    (x, y + 1, room_x, room_y)
                }
            }
            Direction::BottomLeft => {
                if x == 0 && y == 49 {
                    (49, 0, room_x.wrapping_sub(1), room_y.wrapping_add(1))
                } else if x == 0 {
                    (49, y + 1, room_x.wrapping_sub(1), room_y)
                } else if y == 49 {
                    (x - 1, 0, room_x, room_y.wrapping_add(1))
                } else {
                    (x - 1, y + 1, room_x, room_y)
                }
            }
            Direction::Left => {
                if x == 0 {
                    (49, y, room_x.wrapping_sub(1), room_y)
                } else {
                    (x - 1, y, room_x, room_y)
                }
            }
            Direction::TopLeft => {
                if x == 0 && y == 0 {
                    (49, 49, room_x.wrapping_sub(1), room_y.wrapping_sub(1))
                } else if x == 0 {
                    (49, y - 1, room_x.wrapping_sub(1), room_y)
                } else if y == 0 {
                    (x - 1, 49, room_x, room_y.wrapping_sub(1))
                } else {
                    (x - 1, y - 1, room_x, room_y)
                }
            }
        };

        // Create new position with updated coordinates
        Self::new(new_room_x, new_room_y, new_x, new_y)
    }

    #[inline]
    fn decomposed(&self) -> (u8, u8, u8, u8) {
        let (room_x, room_y, x, y) = self.decompose_global();
        // Convert signed room coordinates to unsigned by adding 128
        let room_x_unsigned = ((room_x as i16).wrapping_add(128)) as u8;
        let room_y_unsigned = ((room_y as i16).wrapping_add(128)) as u8;
        (room_x_unsigned, room_y_unsigned, x, y)
    }

    #[inline]
    fn room_name(&self) -> RoomName {
        let (room_x, room_y, _, _) = self.decompose_global();
        // Clamp coordinates to valid ranges
        let room_x = room_x.clamp(-128, 127);
        let room_y = room_y.clamp(-128, 127);
        
        let h = if room_x < 0 { 'W' } else { 'E' };
        let v = if room_y < 0 { 'N' } else { 'S' };
        let x = room_x.unsigned_abs().min(127);  // Ensure within valid range
        let y = room_y.unsigned_abs().min(127);  // Ensure within valid range
        RoomName::new(&format!("{}{}{}{}", h, x, v, y)).unwrap()
    }

    #[inline]
    fn z_order_room(&self) -> u16 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let unsigned_room_x = (global_x / 50) as u8;
        let unsigned_room_y = (global_y / 50) as u8;
        let mut z = 0u16;
        for i in 0..8 {
            z |= ((unsigned_room_x & (1 << i)) as u16) << i | ((unsigned_room_y & (1 << i)) as u16) << (i + 1);
        }
        z
    }

    #[inline]
    fn z_order_local(&self) -> u16 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let local_x = (global_x % 50) as u8;
        let local_y = (global_y % 50) as u8;
        let mut z = 0u16;
        for i in 0..8 {
            z |= ((local_x & (1 << i)) as u16) << i | ((local_y & (1 << i)) as u16) << (i + 1);
        }
        z
    }

    #[inline]
    fn z_order_global(&self) -> u32 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let mut z = 0u32;
        for i in 0..16 {
            z |= ((global_x & (1 << i)) as u32) << i | ((global_y & (1 << i)) as u32) << (i + 1);
        }
        z
    }

    #[inline]
    fn hilbert_room(&self) -> u16 {
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let unsigned_room_x = (global_x / 50) as u8;
        let unsigned_room_y = (global_y / 50) as u8;
        let mut x = unsigned_room_x as u16;
        let mut y = unsigned_room_y as u16;
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
        let global_x = (self.packed & 0xFFFF) as u16;
        let global_y = ((self.packed >> 16) & 0xFFFF) as u16;
        let local_x = (global_x % 50) as u8;
        let local_y = (global_y % 50) as u8;
        let mut x = local_x as u16;
        let mut y = local_y as u16;
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
        let mut global_x = (self.packed & 0xFFFF) as u16;
        let mut global_y = ((self.packed >> 16) & 0xFFFF) as u16;
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
        let fast = GlobalYMajorPosition::from_position(original);
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_xy() {
        let pos = GlobalYMajorPosition::from_position(
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