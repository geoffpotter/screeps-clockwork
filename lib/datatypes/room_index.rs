use std::fmt;
use std::ops::{Index, IndexMut};
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use screeps::{Direction, Position, RoomCoordinate, RoomName};
use std::convert::TryFrom;

/// The tests + the screeps crate expect a 256×256 map, with ±128 "game coordinates."
pub const WORLD_SIZE: i32 = 256;
pub const HALF_WORLD_SIZE: i32 = WORLD_SIZE / 2;
pub const TOTAL_ROOMS: usize = (WORLD_SIZE * WORLD_SIZE) as usize;

pub const WORLD_SIZE_U16: u16 = WORLD_SIZE as u16;
pub const HALF_WORLD_SIZE_U16: u16 = HALF_WORLD_SIZE as u16;


/// Represents a single room's location in the Screeps world, packed into a u16.
/// The coordinate system follows Screeps conventions:
/// - N is negative Y, S is positive Y
/// - E is positive X, W is negative X
/// Uses x-major indexing: packed = x * WORLD_SIZE + y
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct RoomIndex {
    packed: u16,
}

impl RoomIndex {
    /// Create a new RoomIndex from game coords (x,y) in 0..255
    pub fn new(room_x: u8, room_y: u8) -> Self {
        let packed = (room_x as u16) * (WORLD_SIZE as u16) + (room_y as u16);
        Self { packed }
    }

    /// Convert the packed index back into "(x,y)" in 0..255
    pub fn room_xy(&self) -> (u8, u8) {
        let x = (self.packed / (WORLD_SIZE as u16)) as u8;
        let y = (self.packed % (WORLD_SIZE as u16)) as u8;
        (x, y)
    }

    pub fn room_name(&self) -> RoomName {
        RoomName::from_str(&self.to_room_name()).unwrap()
    }

    /// Convert to a room name like "E0N0," "W125S10," etc.  
    pub fn to_room_name(&self) -> String {
        let (x, y) = self.room_xy();
        let x = x as i32;
        let y = y as i32;
        
        // E starts at x=128 (0x80), W ends at x=127 (0x7f)
        let (ew_char, x_coord) = if x >= 128 {
            ('E', x - 128)
        } else {
            ('W', 127 - x)
        };
        
        // S starts at y=128 (0x80), N ends at y=127 (0x7f)
        let (ns_char, y_coord) = if y >= 128 {
            ('S', y - 128)
        } else {
            ('N', 127 - y)
        };
        
        format!("{}{}{}{}", ew_char, x_coord, ns_char, y_coord)
    }

    /// Create a new RoomIndex from a room name like "E0N0" or "W10S15"
    pub fn from_room_name(name: &str) -> Option<Self> {
        if name.len() < 4 {
            return None;
        }
        
        let mut chars = name.chars();
        let ew = chars.next()?;
        
        let mut x_str = String::new();
        for c in chars.by_ref() {
            if c == 'N' || c == 'S' {
                if x_str.is_empty() {
                    return None;
                }
                let x_num: i32 = x_str.parse().ok()?;
                let y_str: String = chars.collect();
                if y_str.is_empty() {
                    return None;
                }
                let y_num: i32 = y_str.parse().ok()?;
                
                // Direct conversion to coordinates
                let x = match ew {
                    'E' => x_num + 128,
                    'W' => 127 - x_num,
                    _ => return None,
                };
                
                let y = match c {
                    'N' => 127 - y_num,
                    'S' => y_num + 128,
                    _ => return None,
                };
                
                if x < 0 || x > 255 || y < 0 || y > 255 {
                    return None;
                }
                
                return Some(Self::new(x as u8, y as u8));
            }
            if !c.is_ascii_digit() {
                return None;
            }
            x_str.push(c);
        }
        None
    }

    /// Returns the packed index in as a usize.
    pub fn index(&self) -> usize {
        self.packed as usize
    }

    pub fn from_index(index: u16) -> Self {
        Self { packed: index }
    }

    /// Standard Manhattan distance: sum of absolute differences in x and y.
    pub fn distance_to(&self, other: &RoomIndex) -> u32 {
        let horizontal_diff = ((self.packed / WORLD_SIZE_U16).abs_diff(other.packed / WORLD_SIZE_U16)) as u32;
        let vertical_diff = ((self.packed % WORLD_SIZE_U16).abs_diff(other.packed % WORLD_SIZE_U16)) as u32;
        horizontal_diff + vertical_diff
    }

    /// Move in a direction with wrapping
    pub fn move_direction(&self, dir: Direction) -> Self {
        let world_size_u16 = WORLD_SIZE as u16;
        
        match dir {
            Direction::Top => {
                // Moving north decreases y
                let new_packed = if self.packed % world_size_u16 == 0 {
                    self.packed + world_size_u16 - 1
                } else {
                    self.packed - 1
                };
                Self { packed: new_packed }
            },
            Direction::Bottom => {
                // Moving south increases y
                let new_packed = if (self.packed + 1) % world_size_u16 == 0 {
                    self.packed + 1 - world_size_u16
                } else {
                    self.packed + 1
                };
                Self { packed: new_packed }
            },
            Direction::Right => {
                // Moving east increases x
                let new_packed = if self.packed >= world_size_u16 * (world_size_u16 - 1) {
                    self.packed % world_size_u16
                } else {
                    self.packed + world_size_u16
                };
                Self { packed: new_packed }
            },
            Direction::Left => {
                // Moving west decreases x
                let new_packed = if self.packed < world_size_u16 {
                    self.packed + world_size_u16 * (world_size_u16 - 1)
                } else {
                    self.packed - world_size_u16
                };
                Self { packed: new_packed }
            },
            Direction::TopRight => {
                // First move north (decrease y)
                let temp = if self.packed % world_size_u16 == 0 {
                    self.packed + world_size_u16 - 1
                } else {
                    self.packed - 1
                };
                // Then move east (increase x)
                if temp >= world_size_u16 * (world_size_u16 - 1) {
                    Self { packed: temp % world_size_u16 }
                } else {
                    Self { packed: temp + world_size_u16 }
                }
            },
            Direction::TopLeft => {
                // First move north (decrease y)
                let temp = if self.packed % world_size_u16 == 0 {
                    self.packed + world_size_u16 - 1
                } else {
                    self.packed - 1
                };
                // Then move west (decrease x)
                if temp < world_size_u16 {
                    Self { packed: temp + world_size_u16 * (world_size_u16 - 1) }
                } else {
                    Self { packed: temp - world_size_u16 }
                }
            },
            Direction::BottomRight => {
                // First move south (increase y)
                let temp = if (self.packed + 1) % world_size_u16 == 0 {
                    self.packed + 1 - world_size_u16
                } else {
                    self.packed + 1
                };
                // Then move east (increase x)
                if temp >= world_size_u16 * (world_size_u16 - 1) {
                    Self { packed: temp % world_size_u16 }
                } else {
                    Self { packed: temp + world_size_u16 }
                }
            },
            Direction::BottomLeft => {
                // First move south (increase y)
                let temp = if (self.packed + 1) % world_size_u16 == 0 {
                    self.packed + 1 - world_size_u16
                } else {
                    self.packed + 1
                };
                // Then move west (decrease x)
                if temp < world_size_u16 {
                    Self { packed: temp + world_size_u16 * (world_size_u16 - 1) }
                } else {
                    Self { packed: temp - world_size_u16 }
                }
            },
        }
    }
}

impl fmt::Debug for RoomIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (x, y) = self.room_xy();
        write!(f, "RoomIndex({}, {})", x, y)
    }
}

impl fmt::Display for RoomIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_room_name())
    }
}

impl From<RoomIndex> for usize {
    fn from(idx: RoomIndex) -> Self {
        idx.packed as usize
    }
}

// Support indexing arrays and Vecs directly with a RoomIndex.
impl<T> Index<RoomIndex> for [T] {
    type Output = T;

    fn index(&self, index: RoomIndex) -> &Self::Output {
        &self[index.packed as usize]
    }
}

impl<T> IndexMut<RoomIndex> for [T] {
    fn index_mut(&mut self, index: RoomIndex) -> &mut Self::Output {
        &mut self[index.packed as usize]
    }
}

impl<T> Index<RoomIndex> for Vec<T> {
    type Output = T;

    fn index(&self, index: RoomIndex) -> &Self::Output {
        &self[index.packed as usize]
    }
}

impl<T> IndexMut<RoomIndex> for Vec<T> {
    fn index_mut(&mut self, index: RoomIndex) -> &mut Self::Output {
        &mut self[index.packed as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use screeps::RoomXY;

    fn verify_room_name(x: u8, y: u8, expected_name: &str) {
        let room_idx = RoomIndex::new(x, y);
        assert_eq!(room_idx.to_room_name(), expected_name);
        
        // Only verify against screeps::RoomName for coordinates that map to valid room names
        let room_name = RoomIndex::from_room_name(expected_name).unwrap();
        assert_eq!(room_name.room_xy(), (x, y));
    }

    #[test]
    fn test_room_coordinates() {
        // Origin points for each quadrant
        let room = RoomName::new("W0N0").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "W0N0");

        let room = RoomName::new("E0N0").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "E0N0");

        let room = RoomName::new("W0S0").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "W0S0");

        let room = RoomName::new("E0S0").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "E0S0");

        // Mid-range values
        let room = RoomName::new("E15N10").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "E15N10");

        let room = RoomName::new("W15S10").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "W15S10");

        let room = RoomName::new("E10N15").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "E10N15");

        let room = RoomName::new("W10S15").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "W10S15");

        // Small offsets
        let room = RoomName::new("E1N1").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "E1N1");

        let room = RoomName::new("W1N1").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "W1N1");

        let room = RoomName::new("E1S1").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "E1S1");

        let room = RoomName::new("W1S1").unwrap();
        let packed = room.packed_repr();
        verify_room_name((packed >> 8) as u8, (packed & 0xFF) as u8, "W1S1");
    }

    #[test]
    fn test_distance() {
        let room1 = RoomName::new("E0N0").unwrap();
        let packed1 = room1.packed_repr();
        let room1_idx = RoomIndex::new((packed1 >> 8) as u8, (packed1 & 0xFF) as u8);

        let room2 = RoomName::new("E0N5").unwrap();
        let packed2 = room2.packed_repr();
        let room2_idx = RoomIndex::new((packed2 >> 8) as u8, (packed2 & 0xFF) as u8);

        let room3 = RoomName::new("E5N0").unwrap(); 
        let packed3 = room3.packed_repr();
        let room3_idx = RoomIndex::new((packed3 >> 8) as u8, (packed3 & 0xFF) as u8);

        let room4 = RoomName::new("E5N5").unwrap();
        let packed4 = room4.packed_repr();
        let room4_idx = RoomIndex::new((packed4 >> 8) as u8, (packed4 & 0xFF) as u8);

        assert_eq!(room1_idx.distance_to(&room2_idx), 5);
        assert_eq!(room1_idx.distance_to(&room3_idx), 5);
        assert_eq!(room1_idx.distance_to(&room4_idx), 10);


    }

    #[test]
    fn test_movement() {
        // Moving north from E0N0 goes to E0N1
        let room = RoomName::new("E0N0").unwrap();
        let packed = room.packed_repr();
        let room_idx = RoomIndex::new((packed >> 8) as u8, (packed & 0xFF) as u8);
        let north = room_idx.move_direction(Direction::Top);
        assert_eq!(north.to_room_name(), "E0N1");

        // Wrapping from W127 to E127
        let room = RoomName::new("W127N0").unwrap();
        let packed = room.packed_repr();
        let room_idx = RoomIndex::new((packed >> 8) as u8, (packed & 0xFF) as u8);
        let west = room_idx.move_direction(Direction::Left);
        assert_eq!(west.to_room_name(), "E127N0");
    }

    #[test]
    fn test_direct_indexing() {
        let mut vec = vec![0; TOTAL_ROOMS];
        let room = RoomName::new("E0N0").unwrap();
        let packed = room.packed_repr();
        let idx = RoomIndex::new((packed >> 8) as u8, (packed & 0xFF) as u8);
        vec[idx] = 42;
        assert_eq!(vec[idx], 42);

        let arr = [0; TOTAL_ROOMS];
        let room2 = RoomName::new("E1S1").unwrap();
        let packed2 = room2.packed_repr();
        let idx2 = RoomIndex::new((packed2 >> 8) as u8, (packed2 & 0xFF) as u8);
        let _val = arr[idx2];
    }

    #[test]
    fn test_hashmap() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        let room = RoomName::new("E0N0").unwrap();
        let packed = room.packed_repr();
        let idx = RoomIndex::new((packed >> 8) as u8, (packed & 0xFF) as u8);
        map.insert(idx, "test");
        assert_eq!(map.get(&idx), Some(&"test"));
    }

    #[test]
    fn test_wrapping_movement() {
        // North-South wrapping
        let room = RoomName::new("E0N127").unwrap();
        let packed = room.packed_repr();
        let north_edge = RoomIndex::new((packed >> 8) as u8, (packed & 0xFF) as u8);
        assert_eq!(north_edge.move_direction(Direction::Top).to_room_name(), "E0S127");
        
        let room = RoomName::new("E0S127").unwrap();
        let packed = room.packed_repr();
        let south_edge = RoomIndex::new((packed >> 8) as u8, (packed & 0xFF) as u8);
        assert_eq!(south_edge.move_direction(Direction::Bottom).to_room_name(), "E0N127");
    }

    #[test]
    fn test_diagonal_movement() {
        let room = RoomName::new("E0S0").unwrap();
        let packed = room.packed_repr();
        let center = RoomIndex::new((packed >> 8) as u8, (packed & 0xFF) as u8);
        assert_eq!(center.move_direction(Direction::TopLeft).to_room_name(), "W0N0");
    }

    #[test]
    fn test_distance_calculations() {
        // Test distances in all directions
        let origin_room = RoomName::new("E0N0").unwrap();
        let origin_packed = origin_room.packed_repr();
        let origin = RoomIndex::new((origin_packed >> 8) as u8, (origin_packed & 0xFF) as u8);

        // Cardinal directions
        let north_room = RoomName::new("E0N20").unwrap();
        let north_packed = north_room.packed_repr();
        let north = RoomIndex::new((north_packed >> 8) as u8, (north_packed & 0xFF) as u8);

        let south_room = RoomName::new("E0S20").unwrap();
        let south_packed = south_room.packed_repr();
        let south = RoomIndex::new((south_packed >> 8) as u8, (south_packed & 0xFF) as u8);

        let east_room = RoomName::new("E20N0").unwrap();
        let east_packed = east_room.packed_repr();
        let east = RoomIndex::new((east_packed >> 8) as u8, (east_packed & 0xFF) as u8);

        let west_room = RoomName::new("W20N0").unwrap();
        let west_packed = west_room.packed_repr();
        let west = RoomIndex::new((west_packed >> 8) as u8, (west_packed & 0xFF) as u8);

        assert_eq!(origin.distance_to(&north), 20);
        assert_eq!(origin.distance_to(&south), 21);
        assert_eq!(origin.distance_to(&east), 20);
        assert_eq!(origin.distance_to(&west), 21);

        // Test diagonal distances
        let diagonal_room = RoomName::new("E10N10").unwrap();
        let diagonal_packed = diagonal_room.packed_repr();
        let diagonal = RoomIndex::new((diagonal_packed >> 8) as u8, (diagonal_packed & 0xFF) as u8);

        assert_eq!(origin.distance_to(&diagonal), 20);
    }

    #[test]
    fn test_room_name_roundtrip() {
        // Test cases from the JavaScript engine
        let test_cases = [
            "E0N0", "W0N0", "E0S0", "W0S0",
            "E15N10", "W15S10", "E10N15", "W10S15",
            "E127N127", "W127N127", "E127S127", "W127S127",
        ];
        
        for name in test_cases {
            if let Some(room_idx) = RoomIndex::from_room_name(name) {
                assert_eq!(room_idx.to_room_name(), name, "Failed for room {}", name);
            } else {
                assert!(false, "Failed to parse room name: {}", name);
            }
        }
    }

    #[test]
    fn test_invalid_room_names() {
        let invalid_names = [
            "", "X0N0", "E-1N0", "EN0", "E0n0",
            "E128N0", "E0N128", "W128S0", "W0S128",
        ];
        
        for name in invalid_names {
            assert!(RoomIndex::from_room_name(name).is_none());
        }
    }

    #[cfg(test)]
    mod room_name_investigation {
        use super::*;

        #[test]
        fn investigate_room_name_coords() {
            let test_cases = [
                "W0N0", "E0N0", "W0S0", "E0S0",
                "E15N10", "W15S10", "E10N15", "W10S15",
                "E1N1", "W1N1", "E1S1", "W1S1",
            ];

            for name in test_cases {
                let room = RoomName::new(name).unwrap();
                let packed = room.packed_repr();
                let x = (packed >> 8) as u8;
                let y = packed as u8;
                println!("{} -> ({}, {})", name, x, y);
            }
        }
    }
}
