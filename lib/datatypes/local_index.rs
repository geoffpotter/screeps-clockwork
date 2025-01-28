use std::fmt;
use std::ops::{Add, Sub};
use screeps::{Direction, RoomCoordinate, RoomXY, ROOM_SIZE};

/// Size of a single room in tiles
const ROOM_SIZE_U16: u16 = ROOM_SIZE as u16;

/// Represents a position within a room in the Screeps world, packed into a u16.
/// Uses x-major indexing: packed = x * ROOM_SIZE + y
/// Coordinates wrap around the edges of the room.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Ord, PartialOrd)]
pub struct LocalIndex {
    // Packed as: x * ROOM_SIZE + y
    // Needs u16 since max value is 50 * 50 = 2500
    packed: u16,
}

impl LocalIndex {
    /// Create a new LocalIndex from coordinates (x,y).
    /// Coordinates wrap around the edges of the room.
    pub fn new(x: u8, y: u8) -> Self {
        // Wrap coordinates to 0..ROOM_SIZE range
        let wrapped_x = x % ROOM_SIZE_U16 as u8;
        let wrapped_y = y % ROOM_SIZE_U16 as u8;

        let packed = (wrapped_x as u16) * ROOM_SIZE_U16 + (wrapped_y as u16);
        Self { packed }
    }

    /// Get the x coordinate (0-49)
    pub fn x(&self) -> u8 {
        (self.packed / ROOM_SIZE_U16) as u8
    }

    /// Get the y coordinate (0-49)
    pub fn y(&self) -> u8 {
        (self.packed % ROOM_SIZE_U16) as u8
    }

    /// Get the coordinates as a tuple
    pub fn xy(&self) -> (u8, u8) {
        (self.x(), self.y())
    }

    /// Returns the packed index in [0..2500)
    pub fn index(&self) -> usize {
        self.packed as usize
    }

    /// Move one square in the given direction, wrapping around room edges
    pub fn r#move(&self, dir: Direction) -> Self {
        // Direction values:
        // Top = 1, TopRight = 2, Right = 3, BottomRight = 4,
        // Bottom = 5, BottomLeft = 6, Left = 7, TopLeft = 8
        let new_packed = match dir {
            // Vertical/Horizontal moves
            Direction::Top => {
                // Moving north decreases y
                if self.packed % ROOM_SIZE_U16 == 0 {
                    self.packed + ROOM_SIZE_U16 - 1  // Wrap to bottom
                } else {
                    self.packed - 1  // Move up one row
                }
            },
            Direction::Bottom => {
                // Moving south increases y
                if (self.packed + 1) % ROOM_SIZE_U16 == 0 {
                    self.packed + 1 - ROOM_SIZE_U16  // Wrap to top
                } else {
                    self.packed + 1  // Move down one row
                }
            },
            Direction::Right => {
                // Moving east increases x
                if self.packed >= ROOM_SIZE_U16 * (ROOM_SIZE_U16 - 1) {
                    self.packed % ROOM_SIZE_U16  // Wrap to left side
                } else {
                    self.packed + ROOM_SIZE_U16  // Move right one column
                }
            },
            Direction::Left => {
                // Moving west decreases x
                if self.packed < ROOM_SIZE_U16 {
                    self.packed + ROOM_SIZE_U16 * (ROOM_SIZE_U16 - 1)  // Wrap to right side
                } else {
                    self.packed - ROOM_SIZE_U16  // Move left one column
                }
            },
            // Diagonal moves combine vertical and horizontal shifts
            Direction::TopRight => {
                // First move north (decrease y)
                let temp = if self.packed % ROOM_SIZE_U16 == 0 {
                    self.packed + ROOM_SIZE_U16 - 1  // Wrap to bottom
                } else {
                    self.packed - 1  // Move up one row
                };
                // Then move east (increase x)
                if temp >= ROOM_SIZE_U16 * (ROOM_SIZE_U16 - 1) {
                    temp % ROOM_SIZE_U16  // Wrap to left side
                } else {
                    temp + ROOM_SIZE_U16  // Move right one column
                }
            },
            Direction::TopLeft => {
                // First move north (decrease y)
                let temp = if self.packed % ROOM_SIZE_U16 == 0 {
                    self.packed + ROOM_SIZE_U16 - 1  // Wrap to bottom
                } else {
                    self.packed - 1  // Move up one row
                };
                // Then move west (decrease x)
                if temp < ROOM_SIZE_U16 {
                    temp + ROOM_SIZE_U16 * (ROOM_SIZE_U16 - 1)  // Wrap to right side
                } else {
                    temp - ROOM_SIZE_U16  // Move left one column
                }
            },
            Direction::BottomRight => {
                // First move south (increase y)
                let temp = if (self.packed + 1) % ROOM_SIZE_U16 == 0 {
                    self.packed + 1 - ROOM_SIZE_U16  // Wrap to top
                } else {
                    self.packed + 1  // Move down one row
                };
                // Then move east (increase x)
                if temp >= ROOM_SIZE_U16 * (ROOM_SIZE_U16 - 1) {
                    temp % ROOM_SIZE_U16  // Wrap to left side
                } else {
                    temp + ROOM_SIZE_U16  // Move right one column
                }
            },
            Direction::BottomLeft => {
                // First move south (increase y)
                let temp = if (self.packed + 1) % ROOM_SIZE_U16 == 0 {
                    self.packed + 1 - ROOM_SIZE_U16  // Wrap to top
                } else {
                    self.packed + 1  // Move down one row
                };
                // Then move west (decrease x)
                if temp < ROOM_SIZE_U16 {
                    temp + ROOM_SIZE_U16 * (ROOM_SIZE_U16 - 1)  // Wrap to right side
                } else {
                    temp - ROOM_SIZE_U16  // Move left one column
                }
            },
        };

        Self { packed: new_packed }
    }

    /// Distance to another position, using Chebyshev distance (max of dx, dy)
    /// This correctly accounts for diagonal movement having the same cost as orthogonal movement
    pub fn distance_to(&self, other: &LocalIndex) -> u32 {
        let horizontal_diff = ((self.packed / ROOM_SIZE_U16).abs_diff(other.packed / ROOM_SIZE_U16)) as u32;
        let vertical_diff = ((self.packed % ROOM_SIZE_U16).abs_diff(other.packed % ROOM_SIZE_U16)) as u32;
        
        // For Chebyshev distance, we take the max
        horizontal_diff.max(vertical_diff)
    }

    /// Check if this position is adjacent to another (including diagonals)
    pub fn is_adjacent_to(&self, other: &LocalIndex) -> bool {
        let horizontal_diff = ((self.packed / ROOM_SIZE_U16).abs_diff(other.packed / ROOM_SIZE_U16)) as u32;
        let vertical_diff = ((self.packed % ROOM_SIZE_U16).abs_diff(other.packed % ROOM_SIZE_U16)) as u32;
        
        horizontal_diff <= 1 && vertical_diff <= 1 && (horizontal_diff != 0 || vertical_diff != 0)
    }

    /// Check if this position is within range of another
    pub fn in_range_to(&self, other: &LocalIndex, range: u32) -> bool {
        self.distance_to(other) <= range
    }
}

impl fmt::Debug for LocalIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LocalIndex({}, {})", self.x(), self.y())
    }
}

impl fmt::Display for LocalIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{},{}]", self.x(), self.y())
    }
}

impl From<LocalIndex> for RoomXY {
    fn from(value: LocalIndex) -> Self {
        RoomXY::new(RoomCoordinate::new(value.x() as u8).unwrap(), RoomCoordinate::new(value.y() as u8).unwrap())
    }
}

impl From<RoomXY> for LocalIndex {
    fn from(value: RoomXY) -> Self {
        LocalIndex::new(value.x.u8(), value.y.u8())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_coordinates() {
        let pos = LocalIndex::new(25, 25);
        assert_eq!(pos.x(), 25);
        assert_eq!(pos.y(), 25);
        assert_eq!(pos.xy(), (25, 25));
        assert_eq!(pos.index(), 25 * ROOM_SIZE as usize + 25);
    }

    #[test]
    fn test_wrapping() {
        // Test wrapping in x direction
        let pos1 = LocalIndex::new(75, 10);  // Should wrap to (25, 10)
        assert_eq!(pos1.x(), 25);
        assert_eq!(pos1.y(), 10);

        // Test wrapping in y direction
        let pos2 = LocalIndex::new(10, 60);  // Should wrap to (10, 10)
        assert_eq!(pos2.x(), 10);
        assert_eq!(pos2.y(), 10);

        // Test wrapping in both directions
        let pos3 = LocalIndex::new(75, 60);  // Should wrap to (25, 10)
        assert_eq!(pos3.x(), 25);
        assert_eq!(pos3.y(), 10);

        // Test edge cases
        let pos4 = LocalIndex::new(49, 49);  // Should stay as (49, 49)
        assert_eq!(pos4.x(), 49);
        assert_eq!(pos4.y(), 49);

        let pos5 = LocalIndex::new(50, 50);  // Should wrap to (0, 0)
        assert_eq!(pos5.x(), 0);
        assert_eq!(pos5.y(), 0);
    }

    #[test]
    fn test_distance_calculations() {
        // Test distances in all directions from center
        let center = LocalIndex::new(25, 25);
        
        // Cardinal directions
        let north = LocalIndex::new(25, 20);
        let south = LocalIndex::new(25, 30);
        let east = LocalIndex::new(30, 25);
        let west = LocalIndex::new(20, 25);
        
        assert_eq!(center.distance_to(&north), 5);
        assert_eq!(center.distance_to(&south), 5);
        assert_eq!(center.distance_to(&east), 5);
        assert_eq!(center.distance_to(&west), 5);

        // Diagonal distances (should be max of dx, dy)
        let northeast = LocalIndex::new(30, 20);
        let southeast = LocalIndex::new(30, 30);
        let southwest = LocalIndex::new(20, 30);
        let northwest = LocalIndex::new(20, 20);
        
        assert_eq!(center.distance_to(&northeast), 5);
        assert_eq!(center.distance_to(&southeast), 5);
        assert_eq!(center.distance_to(&southwest), 5);
        assert_eq!(center.distance_to(&northwest), 5);

        let pos = LocalIndex::new(40, 45);  // Should wrap to (40, 45)
        assert_eq!(pos.x(), 40);
        assert_eq!(pos.y(), 45);
    }

    #[test]
    fn test_movement_comprehensive() {
        // Test from center position
        let center = LocalIndex::new(25, 25);
        
        // Test all 8 directions
        assert_eq!(center.r#move(Direction::Top).xy(), (25, 24));
        assert_eq!(center.r#move(Direction::TopRight).xy(), (26, 24));
        assert_eq!(center.r#move(Direction::Right).xy(), (26, 25));
        assert_eq!(center.r#move(Direction::BottomRight).xy(), (26, 26));
        assert_eq!(center.r#move(Direction::Bottom).xy(), (25, 26));
        assert_eq!(center.r#move(Direction::BottomLeft).xy(), (24, 26));
        assert_eq!(center.r#move(Direction::Left).xy(), (24, 25));
        assert_eq!(center.r#move(Direction::TopLeft).xy(), (24, 24));
        
        // Test wrapping from all edges
        let top_edge = LocalIndex::new(25, 0);
        assert_eq!(top_edge.r#move(Direction::Top).xy(), (25, 49));
        
        let right_edge = LocalIndex::new(49, 25);
        assert_eq!(right_edge.r#move(Direction::Right).xy(), (0, 25));
        
        let bottom_edge = LocalIndex::new(25, 49);
        assert_eq!(bottom_edge.r#move(Direction::Bottom).xy(), (25, 0));
        
        let left_edge = LocalIndex::new(0, 25);
        assert_eq!(left_edge.r#move(Direction::Left).xy(), (49, 25));
        
        // Test wrapping from corners
        let top_right = LocalIndex::new(49, 0);
        assert_eq!(top_right.r#move(Direction::TopRight).xy(), (0, 49));
        
        let bottom_right = LocalIndex::new(49, 49);
        assert_eq!(bottom_right.r#move(Direction::BottomRight).xy(), (0, 0));
        
        let bottom_left = LocalIndex::new(0, 49);
        assert_eq!(bottom_left.r#move(Direction::BottomLeft).xy(), (49, 0));
        
        let top_left = LocalIndex::new(0, 0);
        assert_eq!(top_left.r#move(Direction::TopLeft).xy(), (49, 49));
    }

    #[test]
    fn test_adjacency() {
        let center = LocalIndex::new(25, 25);
        
        // Test all 8 adjacent positions
        let adjacent_positions = [
            LocalIndex::new(25, 24),  // Top
            LocalIndex::new(26, 24),  // TopRight
            LocalIndex::new(26, 25),  // Right
            LocalIndex::new(26, 26),  // BottomRight
            LocalIndex::new(25, 26),  // Bottom
            LocalIndex::new(24, 26),  // BottomLeft
            LocalIndex::new(24, 25),  // Left
            LocalIndex::new(24, 24),  // TopLeft
        ];
        
        for pos in adjacent_positions.iter() {
            assert!(center.is_adjacent_to(pos), "Position {:?} should be adjacent to center {:?}", pos, center);
        }
        
        // Test non-adjacent positions
        let non_adjacent = [
            LocalIndex::new(25, 23),  // Two steps up
            LocalIndex::new(27, 25),  // Two steps right
            LocalIndex::new(25, 27),  // Two steps down
            LocalIndex::new(23, 25),  // Two steps left
            LocalIndex::new(27, 27),  // Two steps diagonal
            center,                   // Same position
        ];
        
        for pos in non_adjacent.iter() {
            assert!(!center.is_adjacent_to(pos), "Position {:?} should not be adjacent to center {:?}", pos, center);
        }

    }

    #[test]
    fn test_range_checks() {
        let center = LocalIndex::new(25, 25);
        
        // Test positions at various ranges
        assert!(center.in_range_to(&LocalIndex::new(25, 24), 1));  // Range 1
        assert!(center.in_range_to(&LocalIndex::new(27, 27), 2));  // Range 2
        assert!(center.in_range_to(&LocalIndex::new(28, 28), 3));  // Range 3
        
        // Test positions just outside range
        assert!(!center.in_range_to(&LocalIndex::new(25, 23), 1));  // Just outside range 1
        assert!(!center.in_range_to(&LocalIndex::new(28, 28), 2));  // Just outside range 2
    }

    #[test]
    fn test_index_operations() {
        let pos = LocalIndex::new(25, 25);
        let idx = pos.index();
        
        // Test that index is correctly calculated
        assert_eq!(idx, 25 * ROOM_SIZE as usize + 25);
        
        // Test that index is within bounds
        assert!(idx < ROOM_SIZE as usize * ROOM_SIZE as usize);
        
        // Test index to coordinates conversion
        let pos2 = LocalIndex { packed: idx as u16 };
        assert_eq!(pos2.x(), 25);
        assert_eq!(pos2.y(), 25);
    }
} 