use std::collections::{BinaryHeap, HashMap};
use screeps::constants::extra::ROOM_SIZE;
use super::terrain::{RoomTerrain, WorldMap};

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    room_x: i32,
    room_y: i32,
    x: u8,
    y: u8,
    f_score: usize,
    g_score: usize,
}

// For the priority queue
impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct PathFinder {
    world: WorldMap,
}

impl PathFinder {
    pub fn new(world: WorldMap) -> Self {
        Self { world }
    }

    fn heuristic(room_x1: i32, room_y1: i32, x1: u8, y1: u8, room_x2: i32, room_y2: i32, x2: u8, y2: u8) -> usize {
        // Chebyshev distance across rooms
        let dx = ((room_x2 - room_x1) * ROOM_SIZE as i32 + (x2 as i32 - x1 as i32)).abs() as usize;
        let dy = ((room_y2 - room_y1) * ROOM_SIZE as i32 + (y2 as i32 - y1 as i32)).abs() as usize;
        dx.max(dy)
    }

    fn get_neighbors(&self, room_x: i32, room_y: i32, x: u8, y: u8) -> Vec<(i32, i32, u8, u8)> {
        let mut neighbors = Vec::with_capacity(8);
        for &(dx, dy) in &[(0, 1), (1, 0), (0, -1), (-1, 0), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let mut new_x = x as i32 + dx;
            let mut new_y = y as i32 + dy;
            let mut new_room_x = room_x;
            let mut new_room_y = room_y;
            
            // Handle room transitions
            if new_x < 0 {
                new_room_x -= 1;
                new_x = ROOM_SIZE as i32 - 1;
            } else if new_x >= ROOM_SIZE as i32 {
                new_room_x += 1;
                new_x = 0;
            }
            
            if new_y < 0 {
                new_room_y -= 1;
                new_y = ROOM_SIZE as i32 - 1;
            } else if new_y >= ROOM_SIZE as i32 {
                new_room_y += 1;
                new_y = 0;
            }

            // Check if the room exists and the position is walkable
            if let Some(room) = self.world.get_room(new_room_x, new_room_y) {
                if room.is_walkable(new_x as u8, new_y as u8) {
                    neighbors.push((new_room_x, new_room_y, new_x as u8, new_y as u8));
                }
            }
        }
        neighbors
    }

    pub fn find_path_multiroom(
        &self,
        start_room_x: i32, 
        start_room_y: i32, 
        start_x: u8, 
        start_y: u8,
        goal_room_x: i32, 
        goal_room_y: i32, 
        goal_x: u8, 
        goal_y: u8
    ) -> Option<Vec<(i32, i32, u8, u8)>> {
        let mut open_set = BinaryHeap::new();
        let mut g_scores = HashMap::new();
        let mut came_from = HashMap::new();

        // Initialize start
        let start_pos = (start_room_x, start_room_y, start_x, start_y);
        g_scores.insert(start_pos, 0);
        open_set.push(Node {
            room_x: start_room_x,
            room_y: start_room_y,
            x: start_x,
            y: start_y,
            f_score: Self::heuristic(start_room_x, start_room_y, start_x, start_y, goal_room_x, goal_room_y, goal_x, goal_y),
            g_score: 0,
        });

        while let Some(current) = open_set.pop() {
            let current_pos = (current.room_x, current.room_y, current.x, current.y);
            
            if current_pos == (goal_room_x, goal_room_y, goal_x, goal_y) {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = current_pos;
                while let Some(&prev) = came_from.get(&current) {
                    path.push(current);
                    current = prev;
                }
                path.push(start_pos);
                path.reverse();
                return Some(path);
            }

            let current_g = g_scores[&current_pos];

            for neighbor_pos in self.get_neighbors(current.room_x, current.room_y, current.x, current.y) {
                let tentative_g = current_g + 1;

                if tentative_g < *g_scores.get(&neighbor_pos).unwrap_or(&usize::MAX) {
                    came_from.insert(neighbor_pos, current_pos);
                    g_scores.insert(neighbor_pos, tentative_g);
                    let f_score = tentative_g + Self::heuristic(
                        neighbor_pos.0, neighbor_pos.1, neighbor_pos.2, neighbor_pos.3,
                        goal_room_x, goal_room_y, goal_x, goal_y
                    );
                    open_set.push(Node {
                        room_x: neighbor_pos.0,
                        room_y: neighbor_pos.1,
                        x: neighbor_pos.2,
                        y: neighbor_pos.3,
                        f_score,
                        g_score: tentative_g,
                    });
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use screeps::constants::extra::ROOM_SIZE;

    #[test]
    fn test_pathfinder_basic() {
        let mut world_map = WorldMap::new();
        let mut room = RoomTerrain::new();
        
        // Create a simple walkable room
        for x in 0..ROOM_SIZE {
            for y in 0..ROOM_SIZE {
                room.set_walkable(x, y, true);
            }
        }
        
        world_map.add_room(0, 0, room);
        
        let pathfinder = PathFinder::new(world_map);
        
        // Test a simple path within the same room
        let path = pathfinder.find_path_multiroom(
            0, 0, 10, 10,  // start
            0, 0, 20, 20   // goal
        );
        
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.len() > 0);
        assert_eq!(path[0], (0, 0, 10, 10));
        assert_eq!(path[path.len() - 1], (0, 0, 20, 20));
    }

    #[test]
    fn test_pathfinder_wall_avoidance() {
        let mut world_map = WorldMap::new();
        let mut room = RoomTerrain::new();
        
        // Create a room with walls blocking direct path
        for x in 0..ROOM_SIZE {
            for y in 0..ROOM_SIZE {
                room.set_walkable(x, y, true);
            }
        }
        
        // Create a wall-like barrier
        for x in 15..25 {
            for y in 15..25 {
                room.set_walkable(x, y, false);
            }
        }
        
        world_map.add_room(0, 0, room);
        
        let pathfinder = PathFinder::new(world_map);
        
        // Test path around the wall
        let path = pathfinder.find_path_multiroom(
            0, 0, 10, 10,  // start
            0, 0, 30, 30   // goal
        );
        
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.len() > 0);
        assert_eq!(path[0], (0, 0, 10, 10));
        assert_eq!(path[path.len() - 1], (0, 0, 30, 30));
        
        // Ensure the path does not go through the wall
        for (_, _, x, y) in &path[1..path.len()-1] {
            assert!(!(15 <= *x && *x < 25 && 15 <= *y && *y < 25), "Path went through wall");
        }
    }
}