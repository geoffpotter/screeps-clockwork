use screeps::constants::extra::ROOM_SIZE;
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};

const ROOM_AREA: usize = (ROOM_SIZE as usize) * (ROOM_SIZE as usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerrainType {
    Plain,
    Wall,
    Swamp,
}

#[derive(Clone)]
pub struct RoomTerrain {
    terrain: [TerrainType; ROOM_AREA],
}

impl RoomTerrain {
    pub fn new() -> Self {
        Self {
            terrain: [TerrainType::Plain; ROOM_AREA],
        }
    }

    pub fn with_walls(walls: &[(u8, u8)]) -> Self {
        let mut terrain = Self::new();
        for &(x, y) in walls {
            terrain.set_wall(x, y);
        }
        terrain
    }

    pub fn with_random_walls(wall_probability: f32) -> Self {
        let mut terrain = Self::new();
        for y in 0..ROOM_SIZE {
            for x in 0..ROOM_SIZE {
                if rand::random::<f32>() < wall_probability {
                    terrain.set_wall(x, y);
                }
            }
        }
        terrain
    }

    pub fn get(&self, x: u8, y: u8) -> TerrainType {
        self.terrain[Self::get_index(x, y)]
    }

    pub fn set_wall(&mut self, x: u8, y: u8) {
        self.terrain[Self::get_index(x, y)] = TerrainType::Wall;
    }

    pub fn set_swamp(&mut self, x: u8, y: u8) {
        self.terrain[Self::get_index(x, y)] = TerrainType::Swamp;
    }

    pub fn set_walkable(&mut self, x: u8, y: u8, walkable: bool) {
        if walkable {
            self.terrain[Self::get_index(x, y)] = TerrainType::Plain;
        } else {
            self.terrain[Self::get_index(x, y)] = TerrainType::Wall;
        }
    }

    fn get_index(x: u8, y: u8) -> usize {
        (y as usize) * (ROOM_SIZE as usize) + (x as usize)
    }

    pub fn is_walkable(&self, x: u8, y: u8) -> bool {
        self.get(x, y) != TerrainType::Wall
    }

    pub fn is_wall(&self, x: u8, y: u8) -> bool {
        self.get(x, y) == TerrainType::Wall
    }
}

impl Default for RoomTerrain {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct WorldMap {
    rooms: HashMap<(i32, i32), RoomTerrain>,
}

impl WorldMap {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    pub fn get_or_create_room(&mut self, room_x: i32, room_y: i32) -> &mut RoomTerrain {
        self.rooms.entry((room_x, room_y)).or_insert_with(RoomTerrain::new)
    }

    pub fn get_room(&self, room_x: i32, room_y: i32) -> Option<&RoomTerrain> {
        self.rooms.get(&(room_x, room_y))
    }

    pub fn add_room(&mut self, room_x: i32, room_y: i32, room: RoomTerrain) {
        self.rooms.insert((room_x, room_y), room);
    }

    pub fn generate_screeps_like_terrain(&mut self, room_x: i32, room_y: i32) {
        let mut rng = rand::thread_rng();
        let room = self.get_or_create_room(room_x, room_y);

        // Generate border walls with exits
        // Top and bottom walls (with middle exit)
        for x in 0..ROOM_SIZE {
            if x < 22 || x > 27 { // Leave middle section open for exit
                room.set_wall(x, 0);
                room.set_wall(x, ROOM_SIZE - 1);
            }
        }
        // Left and right walls (with middle exit)
        for y in 0..ROOM_SIZE {
            if y < 22 || y > 27 { // Leave middle section open for exit
                room.set_wall(0, y);
                room.set_wall(ROOM_SIZE - 1, y);
            }
        }

        // Generate wall clusters
        let mut wall_seeds = HashSet::new();
        let num_wall_clusters = rng.gen_range(10..20);
        for _ in 0..num_wall_clusters {
            let x = rng.gen_range(5..ROOM_SIZE-5);
            let y = rng.gen_range(5..ROOM_SIZE-5);
            wall_seeds.insert((x, y));
        }

        // Grow wall clusters
        let mut walls = HashSet::new();
        for &(seed_x, seed_y) in &wall_seeds {
            let cluster_size = rng.gen_range(10..25);
            let mut cluster = HashSet::new();
            cluster.insert((seed_x, seed_y));

            for _ in 0..cluster_size {
                let (x, y) = {
                    let points: Vec<(u8, u8)> = cluster.iter().copied().collect();
                    points.choose(&mut rng).copied().unwrap()
                };
                // Add neighboring positions with decreasing probability
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        let new_x = (x as i32) + dx;
                        let new_y = (y as i32) + dy;
                        if new_x > 3 && new_x < ROOM_SIZE as i32 - 3 && 
                           new_y > 3 && new_y < ROOM_SIZE as i32 - 3 {
                            if rng.gen_bool(0.7) { // 70% chance to extend cluster
                                cluster.insert((new_x as u8, new_y as u8));
                            }
                        }
                    }
                }
            }
            walls.extend(cluster);
        }

        // Generate swamp clusters similarly
        let mut swamp_seeds = HashSet::new();
        let num_swamp_clusters = rng.gen_range(4..20);
        for _ in 0..num_swamp_clusters {
            let x = rng.gen_range(5..ROOM_SIZE-5);
            let y = rng.gen_range(5..ROOM_SIZE-5);
            swamp_seeds.insert((x, y));
        }

        let mut swamps = HashSet::new();
        for &(seed_x, seed_y) in &swamp_seeds {
            let cluster_size = rng.gen_range(20..40);
            let mut cluster = HashSet::new();
            cluster.insert((seed_x, seed_y));

            for _ in 0..cluster_size {
                let (x, y) = {
                    let points: Vec<(u8, u8)> = cluster.iter().copied().collect();
                    points.choose(&mut rng).copied().unwrap()
                };
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        let new_x = (x as i32) + dx;
                        let new_y = (y as i32) + dy;
                        if new_x > 3 && new_x < ROOM_SIZE as i32 - 3 && 
                           new_y > 3 && new_y < ROOM_SIZE as i32 - 3 {
                            if rng.gen_bool(0.8) { // 80% chance to extend swamp
                                cluster.insert((new_x as u8, new_y as u8));
                            }
                        }
                    }
                }
            }
            swamps.extend(cluster);
        }

        // Apply terrain features, ensuring they don't block exits
        for y in 1..ROOM_SIZE-1 {
            for x in 1..ROOM_SIZE-1 {
                // Don't place terrain near exits
                let is_near_exit = (x >= 21 && x <= 28 && (y <= 2 || y >= ROOM_SIZE-3)) || // Near top/bottom exits
                                 (y >= 21 && y <= 28 && (x <= 2 || x >= ROOM_SIZE-3));     // Near left/right exits
                
                if !is_near_exit {
                    if walls.contains(&(x, y)) {
                        room.set_wall(x, y);
                    } else if swamps.contains(&(x, y)) {
                        room.set_swamp(x, y);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_creation() {
        let terrain = RoomTerrain::new();
        assert_eq!(terrain.get(0, 0), TerrainType::Plain);
        assert_eq!(terrain.get(49, 49), TerrainType::Plain);
    }

    #[test]
    fn test_wall_placement() {
        let walls = vec![(10, 10), (20, 20), (30, 30)];
        let terrain = RoomTerrain::with_walls(&walls);
        
        for &(x, y) in &walls {
            assert_eq!(terrain.get(x, y), TerrainType::Wall);
            assert!(!terrain.is_walkable(x, y));
        }
        
        assert_eq!(terrain.get(0, 0), TerrainType::Plain);
        assert!(terrain.is_walkable(0, 0));
    }
}