use screeps::{Direction, Position, RoomXY, RoomCoordinate, RoomName};
use std::convert::{TryInto, TryFrom};
use super::fast_position;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct GlobalHilbertPosition {
    packed: u32,
}

impl GlobalHilbertPosition {
    // Convert (x,y) to Hilbert curve value
    #[inline]
    fn xy2hilbert(x: u8, y: u8) -> u16 {
        let mut x = x as u16;
        let mut y = y as u16;
        let mut h = 0u16;
        let mut s = 32u16; // Start with largest square size

        while s > 0 {
            let rx = ((x & s) > 0) as u16;
            let ry = ((y & s) > 0) as u16;
            
            h += s.wrapping_mul(s).wrapping_mul((3 * rx) ^ ry);
            
            if ry == 0 {
                if rx == 1 {
                    x = s.wrapping_sub(1).wrapping_sub(x);
                    y = s.wrapping_sub(1).wrapping_sub(y);
                }
                std::mem::swap(&mut x, &mut y);
            }
            
            s = s.wrapping_shr(1);
        }
        
        h
    }

    // Convert Hilbert curve value back to (x,y)
    #[inline]
    fn hilbert2xy(h: u16) -> (u8, u8) {
        let mut x = 0u16;
        let mut y = 0u16;
        let mut t = h;
        let mut s = 1u16;

        while s < 64 {
            let rx = 1 & (t.wrapping_shr(1));
            let ry = 1 & (t ^ rx);
            
            if ry == 0 {
                if rx == 1 {
                    x = s.wrapping_sub(1).wrapping_sub(x);
                    y = s.wrapping_sub(1).wrapping_sub(y);
                }
                std::mem::swap(&mut x, &mut y);
            }
            
            x = x.wrapping_add(s.wrapping_mul(rx));
            y = y.wrapping_add(s.wrapping_mul(ry));
            t = t.wrapping_shr(2);
            s = s.wrapping_shl(1);
        }
        
        (x as u8, y as u8)
    }

    #[inline]
    fn decompose_global(&self) -> (u8, u8, u8, u8) {
        let room_x = ((self.packed >> 24) & 0xFF) as u8;
        let room_y = ((self.packed >> 16) & 0xFF) as u8;
        let (x, y) = Self::hilbert2xy((self.packed & 0xFFFF) as u16);
        (room_x, room_y, x, y)
    }

    #[inline]
    fn from_coords(room_x: u8, room_y: u8, x: u8, y: u8) -> Self {
        let h = Self::xy2hilbert(x, y);
        Self {
            packed: ((room_x as u32) << 24) | 
                   ((room_y as u32) << 16) | 
                   (h as u32)
        }
    }
}

impl fast_position for GlobalHilbertPosition {
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
        Self::from_coords(room_x, room_y, x, y)
    }

    #[inline]
    fn to_position(&self) -> Position {
        let (room_x, room_y, x, y) = self.decompose_global();
        let room_idx = (room_x as u16) | ((room_y as u16) << 8);
        let packed = ((room_idx as u32) << 16) | ((x as u32) << 8) | (y as u32);
        Position::from_packed(packed)
    }

    #[inline]
    fn packed_repr(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        let room_idx = (room_x as u16) | ((room_y as u16) << 8);
        ((room_idx as u32) << 16) | ((x as u32) << 8) | (y as u32)
    }

    #[inline]
    fn from_packed_repr(packed: u32) -> Self {
        let room_idx = (packed >> 16) & 0xFFFF;
        let room_x = (room_idx & 0xFF) as u8;
        let room_y = ((room_idx >> 8) & 0xFF) as u8;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        Self::from_coords(room_x, room_y, x, y)
    }

    #[inline]
    fn xy(&self) -> RoomXY {
        let (_, _, x, y) = self.decompose_global();
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
        let (room_x, room_y, x, y) = self.decompose_global();
        ((room_x as u32) << 24) |
        ((room_y as u32) << 16) |
        ((x as u32) << 8) |
        (y as u32)
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
        let global_x = (room_x as u32 * 50 + x as u32) as u16;
        let global_y = (room_y as u32 * 50 + y as u32) as u16;
        let mut z = 0u32;
        for i in 0..16 {
            z |= ((global_x & (1 << i)) as u32) << i | ((global_y & (1 << i)) as u32) << (i + 1);
        }
        z
    }

    #[inline]
    fn hilbert_room(&self) -> u16 {
        let (room_x, room_y, _, _) = self.decompose_global();
        Self::xy2hilbert(room_x, room_y)
    }

    #[inline]
    fn hilbert_local(&self) -> u16 {
        (self.packed & 0xFFFF) as u16
    }

    #[inline]
    fn hilbert_global(&self) -> u32 {
        let (room_x, room_y, x, y) = self.decompose_global();
        let global_x = room_x as u32 * 50 + x as u32;
        let global_y = room_y as u32 * 50 + y as u32;
        Self::xy2hilbert(global_x as u8, global_y as u8) as u32
    }

    #[inline]
    fn decomposed(&self) -> (u8, u8, u8, u8) {
        self.decompose_global()
    }

    #[inline]
    fn room_name(&self) -> RoomName {
        let (room_x, room_y, _, _) = self.decompose_global();
        let signed_x = room_x.wrapping_sub(128) as i8;
        let signed_y = room_y.wrapping_sub(128) as i8;
        let h = if signed_x < 0 { 'W' } else { 'E' };
        let v = if signed_y < 0 { 'N' } else { 'S' };
        let x = signed_x.abs() as u32;
        let y = signed_y.abs() as u32;
        RoomName::new(&format!("{}{}{}{}", h, x, v, y)).unwrap()
    }

    #[inline]
    fn neighbor_in_dir(&self, dir: Direction) -> Self {
        let room_x = ((self.packed >> 24) & 0xFF) as u8;
        let room_y = ((self.packed >> 16) & 0xFF) as u8;
        let (x, y) = Self::hilbert2xy((self.packed & 0xFFFF) as u16);
        
        let (final_x, final_y, new_room_x, new_room_y) = match dir {
            Direction::Top => {
                if y == 0 {
                    (x, 49, room_x, room_y.wrapping_sub(1))
                } else {
                    (x, y - 1, room_x, room_y)
                }
            }
            Direction::TopRight => {
                if y == 0 && x == 49 {
                    (0, 49, room_x.wrapping_add(1), room_y.wrapping_sub(1))
                } else if y == 0 {
                    (x + 1, 49, room_x, room_y.wrapping_sub(1))
                } else if x == 49 {
                    (0, y - 1, room_x.wrapping_add(1), room_y)
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
                if y == 49 && x == 49 {
                    (0, 0, room_x.wrapping_add(1), room_y.wrapping_add(1))
                } else if y == 49 {
                    (x + 1, 0, room_x, room_y.wrapping_add(1))
                } else if x == 49 {
                    (0, y + 1, room_x.wrapping_add(1), room_y)
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
                if y == 49 && x == 0 {
                    (49, 0, room_x.wrapping_sub(1), room_y.wrapping_add(1))
                } else if y == 49 {
                    (x - 1, 0, room_x, room_y.wrapping_add(1))
                } else if x == 0 {
                    (49, y + 1, room_x.wrapping_sub(1), room_y)
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
                if y == 0 && x == 0 {
                    (49, 49, room_x.wrapping_sub(1), room_y.wrapping_sub(1))
                } else if y == 0 {
                    (x - 1, 49, room_x, room_y.wrapping_sub(1))
                } else if x == 0 {
                    (49, y - 1, room_x.wrapping_sub(1), room_y)
                } else {
                    (x - 1, y - 1, room_x, room_y)
                }
            }
        };

        let h = Self::xy2hilbert(final_x, final_y);
        Self {
            packed: ((new_room_x as u32) << 24) | 
                   ((new_room_y as u32) << 16) | 
                   (h as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use screeps::Position;

    #[test]
    fn test_basic_construction() {
        let pos = GlobalHilbertPosition::from_coords(128, 128, 25, 25);
        assert_eq!(pos.decompose_global(), (128, 128, 25, 25));
    }

    #[test]
    fn test_decompose_global() {
        let test_cases = [
            (128, 128, 25, 25),  // Center of E0N0
            (128, 128, 0, 0),    // Top left of E0N0
            (128, 128, 49, 49),  // Bottom right of E0N0
            (129, 128, 0, 0),    // Top left of E1N0
            (127, 128, 0, 0),    // Top left of W1N0
            (128, 127, 0, 0),    // Top left of E0N1
            (128, 129, 0, 0),    // Top left of E0S1
        ];

        for (room_x, room_y, x, y) in test_cases {
            let pos = GlobalHilbertPosition::from_coords(room_x, room_y, x, y);
            let (rx, ry, lx, ly) = pos.decompose_global();
            assert_eq!((rx, ry, lx, ly), (room_x, room_y, x, y),
                      "Failed decompose_global for room ({},{}) pos ({},{})", room_x, room_y, x, y);
        }
    }

    #[test]
    fn test_position_conversion() {
        let original = Position::new(
            RoomCoordinate::try_from(25_u8).unwrap(),
            RoomCoordinate::try_from(25_u8).unwrap(),
            "E0N0".parse().unwrap()
        );
        
        let fast = GlobalHilbertPosition::from_position(original);
        let (room_x, room_y, x, y) = fast.decompose_global();
        
        assert_eq!(room_x, 128);
        assert_eq!(room_y, 128);
        assert_eq!(x, 25);
        assert_eq!(y, 25);
        
        let roundtrip = fast.to_position();
        assert_eq!(original, roundtrip, "Position roundtrip failed");

        let w1n1 = Position::new(
            RoomCoordinate::try_from(10_u8).unwrap(),
            RoomCoordinate::try_from(10_u8).unwrap(),
            "W1N1".parse().unwrap()
        );
        
        let fast = GlobalHilbertPosition::from_position(w1n1);
        let (room_x, room_y, x, y) = fast.decompose_global();
        
        assert_eq!(room_x, 127);
        assert_eq!(room_y, 127);
        assert_eq!(x, 10);
        assert_eq!(y, 10);
        
        let roundtrip = fast.to_position();
        assert_eq!(w1n1, roundtrip, "Position roundtrip failed for W1N1");
    }

    #[test]
    fn test_xy_coordinates() {
        let pos = GlobalHilbertPosition::from_coords(128, 128, 25, 30);
        let xy = pos.xy();
        assert_eq!(u8::from(xy.x), 25);
        assert_eq!(u8::from(xy.y), 30);
    }

    #[test]
    fn test_neighbor_basic() {
        let center = GlobalHilbertPosition::from_coords(128, 128, 25, 25);
        
        let top = center.neighbor_in_dir(Direction::Top);
        assert_eq!(top.decompose_global(), (128, 128, 25, 24));

        let right = center.neighbor_in_dir(Direction::Right);
        assert_eq!(right.decompose_global(), (128, 128, 26, 25));

        let bottom = center.neighbor_in_dir(Direction::Bottom);
        assert_eq!(bottom.decompose_global(), (128, 128, 25, 26));

        let left = center.neighbor_in_dir(Direction::Left);
        assert_eq!(left.decompose_global(), (128, 128, 24, 25));
    }

    #[test]
    fn test_room_transitions() {
        let edge = GlobalHilbertPosition::from_coords(128, 128, 0, 0);
        
        let up = edge.neighbor_in_dir(Direction::Top);
        assert_eq!(up.decompose_global(), (128, 127, 0, 49));

        let left = edge.neighbor_in_dir(Direction::Left);
        assert_eq!(left.decompose_global(), (127, 128, 49, 0));
        
        let corner = GlobalHilbertPosition::from_coords(128, 128, 49, 49);
        let diagonal = corner.neighbor_in_dir(Direction::BottomRight);
        assert_eq!(diagonal.decompose_global(), (129, 129, 0, 0));
    }
} 