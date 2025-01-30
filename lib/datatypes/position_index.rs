use std::fmt;
use screeps::{Direction, Position, RoomCoordinate, RoomName, ROOM_SIZE};
use super::{local_index::LocalIndex, room_index::RoomIndex};

/// Represents a position in the Screeps world, combining room and local coordinates.
/// Uses y-major indexing for consistency.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct PositionIndex {
    room: RoomIndex,
    local: LocalIndex,
}

impl PositionIndex {
    /// Create a new PositionIndex from room and local indices
    pub fn new(room: RoomIndex, local: LocalIndex) -> Self {
        Self { room, local }
    }

    /// Get the room component
    pub fn room(&self) -> RoomIndex {
        self.room
    }

    /// Get the local component
    pub fn local(&self) -> LocalIndex {
        self.local
    }

    /// Get the room name
    pub fn room_name(&self) -> RoomName {
        self.room.to_room_name().parse().unwrap()
    }

    /// Get local x coordinate (0-49)
    pub fn x(&self) -> RoomCoordinate {
        // SAFETY: LocalIndex guarantees coordinates are in valid range
        unsafe { RoomCoordinate::unchecked_new(self.local.x()) }
    }

    /// Get local y coordinate (0-49)
    pub fn y(&self) -> RoomCoordinate {
        // SAFETY: LocalIndex guarantees coordinates are in valid range
        unsafe { RoomCoordinate::unchecked_new(self.local.y()) }
    }

    /// Move one square in the given direction, returning None if moving off map edge
    pub fn r#move(&self, dir: Direction) -> Option<Self> {
        // Get current position
        let x = self.local.x();
        let y = self.local.y();
        let (room_x, room_y) = self.room.room_xy();
        
        // Handle room transitions
        if (x == 49 && matches!(dir, Direction::Right)) ||
           (x == 0 && matches!(dir, Direction::Left)) ||
           (y == 0 && matches!(dir, Direction::Top)) ||
           (y == 49 && matches!(dir, Direction::Bottom)) {
            
            // Check if we're at the map edge first
            if (room_x == 255 && matches!(dir, Direction::Right)) ||
               (room_x == 0 && matches!(dir, Direction::Left)) ||
               (room_y == 255 && matches!(dir, Direction::Top)) ||
               (room_y == 0 && matches!(dir, Direction::Bottom)) {
                return None;
            }
            
            // Move the room first - note that room coordinates increase going east
            // For north/south: room_y=127 is N0, going north INCREASES room_y, going south DECREASES room_y
            let new_room = match dir {
                Direction::Right => RoomIndex::new(room_x.wrapping_add(1), room_y),
                Direction::Left => RoomIndex::new(room_x.wrapping_sub(1), room_y),
                Direction::Top => RoomIndex::new(room_x, room_y.wrapping_add(1)),  // Going north increases Y
                Direction::Bottom => RoomIndex::new(room_x, room_y.wrapping_sub(1)),  // Going south decreases Y
                _ => unreachable!()
            };
            // let (new_room_x, new_room_y) = new_room.room_xy();
            
            // Calculate new local position
            let new_local = match dir {
                Direction::Right => LocalIndex::new(0, y),
                Direction::Left => LocalIndex::new(49, y),
                Direction::Top => LocalIndex::new(x, 49),
                Direction::Bottom => LocalIndex::new(x, 0),
                _ => unreachable!()
            };
            
            Some(Self::new(new_room, new_local))
        } else if (x == 49 && y == 0 && matches!(dir, Direction::TopRight)) ||
                  (x == 0 && y == 0 && matches!(dir, Direction::TopLeft)) ||
                  (x == 49 && y == 49 && matches!(dir, Direction::BottomRight)) ||
                  (x == 0 && y == 49 && matches!(dir, Direction::BottomLeft)) {
            
            
            // Check if we're at the map edge first
            if (room_x == 255 && matches!(dir, Direction::TopRight | Direction::BottomRight)) ||
               (room_x == 0 && matches!(dir, Direction::TopLeft | Direction::BottomLeft)) ||
               (room_y == 255 && matches!(dir, Direction::TopLeft | Direction::TopRight)) ||
               (room_y == 0 && matches!(dir, Direction::BottomLeft | Direction::BottomRight)) {
                return None;
            }
            
            // Move the room first - handle both x and y changes
            let new_room = match dir {
                Direction::TopRight => RoomIndex::new(room_x.wrapping_add(1), room_y.wrapping_add(1)),  // North increases Y
                Direction::TopLeft => RoomIndex::new(room_x.wrapping_sub(1), room_y.wrapping_add(1)),
                Direction::BottomRight => RoomIndex::new(room_x.wrapping_add(1), room_y.wrapping_sub(1)),  // South decreases Y
                Direction::BottomLeft => RoomIndex::new(room_x.wrapping_sub(1), room_y.wrapping_sub(1)),
                _ => unreachable!()
            };
            // let (new_room_x, new_room_y) = new_room.room_xy();
           
            
            // Calculate new local position
            let new_local = match dir {
                Direction::TopRight => LocalIndex::new(0, 49),
                Direction::TopLeft => LocalIndex::new(49, 49),
                Direction::BottomRight => LocalIndex::new(0, 0),
                Direction::BottomLeft => LocalIndex::new(49, 0),
                _ => unreachable!()
            };
            
            Some(Self::new(new_room, new_local))
        } else if matches!(dir, Direction::TopRight | Direction::TopLeft | Direction::BottomRight | Direction::BottomLeft) {
            // Block diagonal movement at edges
            if (x == 49 && matches!(dir, Direction::TopRight | Direction::BottomRight)) ||
               (x == 0 && matches!(dir, Direction::TopLeft | Direction::BottomLeft)) ||
               (y == 0 && matches!(dir, Direction::TopLeft | Direction::TopRight)) ||
               (y == 49 && matches!(dir, Direction::BottomLeft | Direction::BottomRight)) {
                return None;
            }
            
            // Normal diagonal movement within room
            let new_local = self.local.r#move(dir);
            Some(Self::new(self.room, new_local))
        } else {
            // Normal movement within room
            let new_local = self.local.r#move(dir);
            Some(Self::new(self.room, new_local))
        }
    }

    /// Distance to another position, using Chebyshev distance (max of dx, dy)
    pub fn distance_to(&self, other: &PositionIndex) -> u32 {
        if self.room == other.room {
            // Same room - use local distance
            self.local.distance_to(&other.local)
        } else {
            // Convert to global coordinates
            let (room_x1, room_y1) = self.room.room_xy();
            let (room_x2, room_y2) = other.room.room_xy();
            
            let global_x1 = (room_x1 as u32 * ROOM_SIZE as u32) + self.local.x() as u32;
            let global_y1 = (room_y1 as u32 * ROOM_SIZE as u32) + self.local.y() as u32;
            let global_x2 = (room_x2 as u32 * ROOM_SIZE as u32) + other.local.x() as u32;
            let global_y2 = (room_y2 as u32 * ROOM_SIZE as u32) + other.local.y() as u32;
            
            // Use Chebyshev distance on global coordinates
            global_x1.abs_diff(global_x2).max(global_y1.abs_diff(global_y2))
        }
    }

    /// Check if this position is adjacent to another (including diagonals)
    pub fn is_adjacent_to(&self, other: &PositionIndex) -> bool {
        if self.room == other.room {
            // Same room - use local adjacency
            self.local.is_adjacent_to(&other.local)
        } else {
            // Check if rooms are adjacent and positions are at the edges
            let (room_x1, room_y1) = self.room.room_xy();
            let (room_x2, room_y2) = other.room.room_xy();
            
            // Calculate absolute differences using wrapping_sub and max
            let room_dx = room_x1.wrapping_sub(room_x2).min(room_x2.wrapping_sub(room_x1));
            let room_dy = room_y1.wrapping_sub(room_y2).min(room_y2.wrapping_sub(room_y1));
            
            if room_dx <= 1 && room_dy <= 1 && (room_dx + room_dy > 0) {
                // Rooms are adjacent, check if positions are at the connecting edges
                let x1 = self.local.x() as u8;
                let y1 = self.local.y() as u8;
                let x2 = other.local.x() as u8;
                let y2 = other.local.y() as u8;
                
                match (room_dx, room_dy) {
                    (1, 0) => (x1 == 49 && x2 == 0) || (x1 == 0 && x2 == 49),
                    (0, 1) => (y1 == 49 && y2 == 0) || (y1 == 0 && y2 == 49),
                    (1, 1) => (x1 == 49 && y1 == 49 && x2 == 0 && y2 == 0) ||
                             (x1 == 49 && y1 == 0 && x2 == 0 && y2 == 49) ||
                             (x1 == 0 && y1 == 49 && x2 == 49 && y2 == 0) ||
                             (x1 == 0 && y1 == 0 && x2 == 49 && y2 == 49),
                    _ => false,
                }
            } else {
                false
            }
        }
    }

    /// Check if this position is within range of another
    pub fn in_range_to(&self, other: &PositionIndex, range: u32) -> bool {
        self.distance_to(other) <= range
    }
}


impl fmt::Debug for PositionIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PositionIndex({}, {} in {})", self.x(), self.y(), self.room_name())
    }
}

impl fmt::Display for PositionIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Position::from(*self))
    }
}

impl Ord for PositionIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // First compare rooms
        match self.room.cmp(&other.room) {
            std::cmp::Ordering::Equal => self.local.index().cmp(&other.local.index()),
            ord => ord,
        }
    }
}

impl PartialOrd for PositionIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Position> for PositionIndex {
    fn from(pos: Position) -> Self {
        let packed = pos.packed_repr();
        // In packed format:
        // - Bits 31-24: room_x
        // - Bits 23-16: room_y
        // - Bits 15-8:  local_x
        // - Bits 7-0:   local_y
        let room_x = ((packed >> 24) & 0xFF) as u8;
        let room_y = ((packed >> 16) & 0xFF) as u8;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        
            
        
        
        let room = RoomIndex::new(room_x, room_y);
        
        

        
        Self {
            room,
            local: LocalIndex::new(x, y),
        }
    }
}

impl From<PositionIndex> for Position {
    fn from(pos: PositionIndex) -> Self {
        Position::new(pos.x(), pos.y(), pos.room_name())
    }
}

impl From<u32> for PositionIndex {
    fn from(packed: u32) -> Self {
        let room_x = ((packed >> 24) & 0xFF) as u8;
        let room_y = ((packed >> 16) & 0xFF) as u8;
        let x = ((packed >> 8) & 0xFF) as u8;
        let y = (packed & 0xFF) as u8;
        Self::new(RoomIndex::new(room_x, room_y), LocalIndex::new(x, y))
    }
}

#[cfg(test)]
mod tests {


    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn test_movement() {
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        
        // Test normal movement
        let moved = pos.r#move(Direction::Right).unwrap();
        assert_eq!(moved.x(), RoomCoordinate::try_from(26).unwrap());
        assert_eq!(moved.y(), RoomCoordinate::try_from(25).unwrap());
        
        // Test room transition
        let edge = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(49).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        let moved = edge.r#move(Direction::Right).unwrap();
        assert_eq!(moved.room_name().to_string(), "E1N0");
        assert_eq!(moved.x(), RoomCoordinate::try_from(0).unwrap());
        assert_eq!(moved.y(), RoomCoordinate::try_from(25).unwrap());
        
        // Test map edge
        let map_edge = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(49).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E127N0".parse().unwrap(),
        ));
        assert!(map_edge.r#move(Direction::Right).is_none());
    }

    #[test]
    fn test_distance() {
        let pos1 = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(10).unwrap(),
            RoomCoordinate::try_from(10).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        let pos2 = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(15).unwrap(),
            RoomCoordinate::try_from(15).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        assert_eq!(pos1.distance_to(&pos2), 5);
        
        // Test cross-room distance
        let pos3 = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(45).unwrap(),
            RoomCoordinate::try_from(45).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        let pos4 = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(5).unwrap(),
            RoomCoordinate::try_from(5).unwrap(),
            "E1N1".parse().unwrap(),
        ));
        assert_eq!(pos3.distance_to(&pos4), 90);  // Updated for Chebyshev distance
    }

    #[test]
    fn test_adjacency() {
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        let adjacent = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(26).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        let diagonal = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(26).unwrap(),
            RoomCoordinate::try_from(26).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        
        assert!(pos.is_adjacent_to(&adjacent));
        assert!(pos.is_adjacent_to(&diagonal));
        
        // Test cross-room adjacency
        let edge = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(49).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        let next_room = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(0).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E1N0".parse().unwrap(),
        ));
        assert!(edge.is_adjacent_to(&next_room));
    }

    #[test]
    fn test_room_transitions() {
        // Test right transition
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(49).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        let moved = pos.r#move(Direction::Right).unwrap();
        assert_eq!(moved.room_name().to_string(), "E1N0");
        assert_eq!(moved.x(), RoomCoordinate::try_from(0).unwrap());
        assert_eq!(moved.y(), RoomCoordinate::try_from(25).unwrap());

        // Test left transition
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(0).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E1N0".parse().unwrap(),
        ));
        let moved = pos.r#move(Direction::Left).unwrap();
        assert_eq!(moved.room_name().to_string(), "E0N0");
        assert_eq!(moved.x(), RoomCoordinate::try_from(49).unwrap());
        assert_eq!(moved.y(), RoomCoordinate::try_from(25).unwrap());

        // Test top transition
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(0).unwrap(),
            "E0N1".parse().unwrap(),
        ));
        let moved = pos.r#move(Direction::Top).unwrap();
        assert_eq!(moved.room_name().to_string(), "E0N0");
        assert_eq!(moved.x(), RoomCoordinate::try_from(25).unwrap());
        assert_eq!(moved.y(), RoomCoordinate::try_from(49).unwrap());

        // Test bottom transition
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(49).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        let moved = pos.r#move(Direction::Bottom).unwrap();
        assert_eq!(moved.room_name().to_string(), "E0N1");
        assert_eq!(moved.x(), RoomCoordinate::try_from(25).unwrap());
        assert_eq!(moved.y(), RoomCoordinate::try_from(0).unwrap());

        // Test diagonal transitions
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(49).unwrap(),
            RoomCoordinate::try_from(0).unwrap(),
            "E0N1".parse().unwrap(),
        ));
        let moved = pos.r#move(Direction::TopRight).unwrap();
        assert_eq!(moved.room_name().to_string(), "E1N0");
        assert_eq!(moved.x(), RoomCoordinate::try_from(0).unwrap());
        assert_eq!(moved.y(), RoomCoordinate::try_from(49).unwrap());
    }

    #[test]
    fn test_room_name_consistency() {
        // Test E0N0 coordinates
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E0N0".parse().unwrap(),
        ));
        assert_eq!(pos.room_name().to_string(), "E0N0");

        // Test W1N1 coordinates
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "W1N1".parse().unwrap(),
        ));
        assert_eq!(pos.room_name().to_string(), "W1N1");

        // Test E1S1 coordinates
        let pos = PositionIndex::from(Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E1S1".parse().unwrap(),
        ));
        assert_eq!(pos.room_name().to_string(), "E1S1");
    }

    #[test]
    fn test_from_position() {
        // Test E0N0 coordinates
        let pos = Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E0N0".parse().unwrap(),
        );
        let pos_index = PositionIndex::from(pos);
        assert_eq!(pos_index.room_name().to_string(), "E0N0");
        assert_eq!(pos_index.x(), RoomCoordinate::try_from(25).unwrap());
        assert_eq!(pos_index.y(), RoomCoordinate::try_from(25).unwrap());

        // Test W1N1 coordinates
        let pos = Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "W1N1".parse().unwrap(),
        );
        let pos_index = PositionIndex::from(pos);
        assert_eq!(pos_index.room_name().to_string(), "W1N1");
        assert_eq!(pos_index.x(), RoomCoordinate::try_from(25).unwrap());
        assert_eq!(pos_index.y(), RoomCoordinate::try_from(25).unwrap());

        // Test E15N10 coordinates at room edge
        let pos = Position::new(
            RoomCoordinate::try_from(49).unwrap(),
            RoomCoordinate::try_from(0).unwrap(),
            "E15N10".parse().unwrap(),
        );
        let pos_index = PositionIndex::from(pos);
        assert_eq!(pos_index.room_name().to_string(), "E15N10");
        assert_eq!(pos_index.x(), RoomCoordinate::try_from(49).unwrap());
        assert_eq!(pos_index.y(), RoomCoordinate::try_from(0).unwrap());

        // Test E1S1 coordinates
        let pos = Position::new(
            RoomCoordinate::try_from(25).unwrap(),
            RoomCoordinate::try_from(25).unwrap(),
            "E1S1".parse().unwrap(),
        );
        let pos_index = PositionIndex::from(pos);
        assert_eq!(pos_index.room_name().to_string(), "E1S1");
        assert_eq!(pos_index.x(), RoomCoordinate::try_from(25).unwrap());
        assert_eq!(pos_index.y(), RoomCoordinate::try_from(25).unwrap());
    }
} 