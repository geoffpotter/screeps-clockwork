use std::collections::{BinaryHeap, HashMap};
use screeps::constants::extra::ROOM_SIZE;
use super::terrain::{RoomTerrain, WorldMap};

const ROOM_AREA: usize = (ROOM_SIZE as usize) * (ROOM_SIZE as usize);

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    room_x: i32,
    room_y: i32,
    x: u8,
    y: u8,
    f_score: usize,
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

// Working data for pathfinding that can be either on stack or heap
struct PathFindingData {
    g_scores: HashMap<(i32, i32, u8, u8), usize>,
    came_from: HashMap<(i32, i32, u8, u8), (i32, i32, u8, u8)>,
}

impl PathFindingData {
    fn new() -> Self {
        Self {
            g_scores: HashMap::new(),
            came_from: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.g_scores.clear();
        self.came_from.clear();
    }
}

// Stack-based pathfinder
pub struct StackPathFinder {
    open_set: BinaryHeap<Node>,
    working_data: PathFindingData,
    world: WorldMap,
}

// Heap-based pathfinder
pub struct HeapPathFinder {
    open_set: BinaryHeap<Node>,
    working_data: Box<PathFindingData>,
    world: WorldMap,
}

// Common functionality for both pathfinders
pub trait PathFinding {
    fn get_working_data(&mut self) -> &mut PathFindingData;
    fn get_world(&self) -> &WorldMap;
    fn get_open_set(&mut self) -> &mut BinaryHeap<Node>;

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
            if let Some(room) = self.get_world().get_room(new_room_x, new_room_y) {
                if room.is_walkable(new_x as u8, new_y as u8) {
                    neighbors.push((new_room_x, new_room_y, new_x as u8, new_y as u8));
                }
            }
        }
        neighbors
    }

    fn heuristic(room_x1: i32, room_y1: i32, x1: u8, y1: u8, room_x2: i32, room_y2: i32, x2: u8, y2: u8) -> usize {
        // Chebyshev distance across rooms
        let dx = ((room_x2 - room_x1) * ROOM_SIZE as i32 + (x2 as i32 - x1 as i32)).abs() as usize;
        let dy = ((room_y2 - room_y1) * ROOM_SIZE as i32 + (y2 as i32 - y1 as i32)).abs() as usize;
        dx.max(dy)
    }

    fn find_path_multiroom(
        &mut self,
        start_room_x: i32, start_room_y: i32, start_x: u8, start_y: u8,
        goal_room_x: i32, goal_room_y: i32, goal_x: u8, goal_y: u8
    ) -> Option<Vec<(i32, i32, u8, u8)>> {
        // Reset state
        self.get_open_set().clear();
        self.get_working_data().reset();

        // Initialize start
        let start_pos = (start_room_x, start_room_y, start_x, start_y);
        self.get_working_data().g_scores.insert(start_pos, 0);
        self.get_open_set().push(Node {
            room_x: start_room_x,
            room_y: start_room_y,
            x: start_x,
            y: start_y,
            f_score: Self::heuristic(start_room_x, start_room_y, start_x, start_y, goal_room_x, goal_room_y, goal_x, goal_y),
        });

        while let Some(current) = self.get_open_set().pop() {
            let current_pos = (current.room_x, current.room_y, current.x, current.y);
            
            if current_pos == (goal_room_x, goal_room_y, goal_x, goal_y) {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = current_pos;
                while let Some(&prev) = self.get_working_data().came_from.get(&current) {
                    path.push(current);
                    current = prev;
                }
                path.push(start_pos);
                path.reverse();
                return Some(path);
            }

            let current_g = self.get_working_data().g_scores[&current_pos];

            for neighbor_pos in self.get_neighbors(current.room_x, current.room_y, current.x, current.y) {
                let tentative_g = current_g + 1;

                if tentative_g < *self.get_working_data().g_scores.get(&neighbor_pos).unwrap_or(&usize::MAX) {
                    self.get_working_data().came_from.insert(neighbor_pos, current_pos);
                    self.get_working_data().g_scores.insert(neighbor_pos, tentative_g);
                    let f_score = tentative_g + Self::heuristic(
                        neighbor_pos.0, neighbor_pos.1, neighbor_pos.2, neighbor_pos.3,
                        goal_room_x, goal_room_y, goal_x, goal_y
                    );
                    self.get_open_set().push(Node {
                        room_x: neighbor_pos.0,
                        room_y: neighbor_pos.1,
                        x: neighbor_pos.2,
                        y: neighbor_pos.3,
                        f_score,
                    });
                }
            }
        }

        None
    }
}

impl StackPathFinder {
    pub fn new(world: WorldMap) -> Self {
        Self {
            open_set: BinaryHeap::new(),
            working_data: PathFindingData::new(),
            world,
        }
    }
}

impl HeapPathFinder {
    pub fn new(world: WorldMap) -> Self {
        Self {
            open_set: BinaryHeap::new(),
            working_data: Box::new(PathFindingData::new()),
            world,
        }
    }
}

impl PathFinding for StackPathFinder {
    fn get_working_data(&mut self) -> &mut PathFindingData {
        &mut self.working_data
    }

    fn get_world(&self) -> &WorldMap {
        &self.world
    }

    fn get_open_set(&mut self) -> &mut BinaryHeap<Node> {
        &mut self.open_set
    }
}

impl PathFinding for HeapPathFinder {
    fn get_working_data(&mut self) -> &mut PathFindingData {
        &mut self.working_data
    }

    fn get_world(&self) -> &WorldMap {
        &self.world
    }

    fn get_open_set(&mut self) -> &mut BinaryHeap<Node> {
        &mut self.open_set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pathfinder_implementation<T: PathFinding>(mut pathfinder: T) {
        let path = pathfinder.find_path_multiroom(0, 0, 0, 0, 0, 0, 3, 3).unwrap();
        assert_eq!(path.first(), Some(&(0, 0, 0, 0)));
        assert_eq!(path.last(), Some(&(0, 0, 3, 3)));
    }

    #[test]
    fn test_stack_pathfinder() {
        let mut world = WorldMap::new();
        world.generate_screeps_like_terrain(0, 0);
        let pathfinder = StackPathFinder::new(world);
        test_pathfinder_implementation(pathfinder);
    }

    #[test]
    fn test_heap_pathfinder() {
        let mut world = WorldMap::new();
        world.generate_screeps_like_terrain(0, 0);
        let pathfinder = HeapPathFinder::new(world);
        test_pathfinder_implementation(pathfinder);
    }

    #[test]
    fn test_wall_avoidance() {
        let mut world = WorldMap::new();
        world.generate_screeps_like_terrain(0, 0);
        world.get_or_create_room(0, 0).set_wall(1, 1);
        let mut pathfinder = StackPathFinder::new(world);
        
        let path = pathfinder.find_path_multiroom(0, 0, 0, 0, 0, 0, 2, 2).unwrap();
        assert!(!path.contains(&(0, 0, 1, 1)));
    }
} 