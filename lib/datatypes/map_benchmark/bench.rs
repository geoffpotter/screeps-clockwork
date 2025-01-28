use std::time::Instant;
use rand::Rng;
use super::terrain::{RoomTerrain, WorldMap};
use super::pathfinding::PathFinder;
use screeps::constants::extra::ROOM_SIZE;
use image::{ImageBuffer, Rgb};

const CELL_SIZE: u32 = 5; // Reduced size for multi-room visualization
const WORLD_SIZE: i32 = 5; // Will create a 3x3 world of rooms
const TOTAL_SIZE: u32 = (WORLD_SIZE as u32 * ROOM_SIZE as u32);
const IMAGE_SIZE: u32 = TOTAL_SIZE * CELL_SIZE;

fn draw_world_and_paths(
    world: &WorldMap,
    paths: &[Vec<(i32, i32, u8, u8)>], // (room_x, room_y, x, y)
    filename: &str,
) {
    let mut img = ImageBuffer::new(IMAGE_SIZE, IMAGE_SIZE);

    // Draw terrain for all rooms
    for room_y in 0..WORLD_SIZE {
        for room_x in 0..WORLD_SIZE {
            if let Some(terrain) = world.get_room(room_x, room_y) {
                for y in 0..ROOM_SIZE {
                    for x in 0..ROOM_SIZE {
                        let color = match terrain.get(x, y) {
                            super::terrain::TerrainType::Wall => Rgb([50, 50, 50]),   // Dark gray for walls
                            super::terrain::TerrainType::Swamp => Rgb([76, 110, 60]), // Greenish for swamps
                            super::terrain::TerrainType::Plain => Rgb([200, 200, 200]), // Light gray for plains
                        };

                        // Calculate global pixel position
                        let px = (room_x as u32 * ROOM_SIZE as u32 + x as u32) * CELL_SIZE;
                        let py = (room_y as u32 * ROOM_SIZE as u32 + y as u32) * CELL_SIZE;

                        // Fill the cell
                        for dy in 0..CELL_SIZE {
                            for dx in 0..CELL_SIZE {
                                img.put_pixel(px + dx, py + dy, color);
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw paths with different colors
    for (path_idx, path) in paths.iter().enumerate() {
        let hue = (path_idx as f32 / paths.len() as f32) * 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
        let path_color = Rgb([r, g, b]);

        for window in path.windows(2) {
            let (room_x1, room_y1, x1, y1) = window[0];
            let (room_x2, room_y2, x2, y2) = window[1];
            
            // Calculate global pixel positions
            let px1 = ((room_x1 as u32 * ROOM_SIZE as u32 + x1 as u32) * CELL_SIZE) + CELL_SIZE / 2;
            let py1 = ((room_y1 as u32 * ROOM_SIZE as u32 + y1 as u32) * CELL_SIZE) + CELL_SIZE / 2;
            let px2 = ((room_x2 as u32 * ROOM_SIZE as u32 + x2 as u32) * CELL_SIZE) + CELL_SIZE / 2;
            let py2 = ((room_y2 as u32 * ROOM_SIZE as u32 + y2 as u32) * CELL_SIZE) + CELL_SIZE / 2;
            
            draw_line(&mut img, px1, py1, px2, py2, path_color);
        }
    }

    img.save(filename).expect("Failed to save image");
}

fn draw_line(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, x1: u32, y1: u32, x2: u32, y2: u32, color: Rgb<u8>) {
    // Bresenham's line algorithm
    let dx = (x2 as i32 - x1 as i32).abs();
    let dy = (y2 as i32 - y1 as i32).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx - dy;

    let mut x = x1 as i32;
    let mut y = y1 as i32;

    loop {
        if x >= 0 && y >= 0 && x < IMAGE_SIZE as i32 && y < IMAGE_SIZE as i32 {
            img.put_pixel(x as u32, y as u32, color);
        }

        if x == x2 as i32 && y == y2 as i32 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

fn generate_random_world_points(num_points: usize, world_size: i32) -> Vec<(i32, i32, u8, u8)> {
    let mut rng = rand::thread_rng();
    let mut points = Vec::with_capacity(num_points);
    
    for _ in 0..num_points {
        let room_x = rng.gen_range(0..world_size);
        let room_y = rng.gen_range(0..world_size);
        let x = rng.gen_range(0..ROOM_SIZE);
        let y = rng.gen_range(0..ROOM_SIZE);
        points.push((room_x, room_y, x, y));
    }
    points
}

pub fn run_benchmark() {
    const ITERATIONS: usize = 10;
    
    // Create and initialize world
    let mut world = WorldMap::new();
    for y in 0..WORLD_SIZE {
        for x in 0..WORLD_SIZE {
            world.generate_screeps_like_terrain(x, y);
        }
    }
    
    let points = generate_random_world_points(20, WORLD_SIZE);
    
    let mut paths = Vec::new();
    
    // PathFinder version
    let mut pathfinder = PathFinder::new(world.clone());
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for window in points.windows(2) {
            let (start_room_x, start_room_y, start_x, start_y) = window[0];
            let (goal_room_x, goal_room_y, goal_x, goal_y) = window[1];
            if let Some(path) = pathfinder.find_path_multiroom(
                start_room_x, start_room_y, start_x, start_y,
                goal_room_x, goal_room_y, goal_x, goal_y
            ) {
                if paths.len() < 10 {
                    paths.push(path);
                }
            }
        }
    }
    let pathfinder_time = start.elapsed();
    
    println!("PathFinder version took: {:?}", pathfinder_time);

    // Generate visualizations
    draw_world_and_paths(&world, &paths, "pathfinder_paths.png");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_pathfinding() {
        let mut world_map = WorldMap::new();

        // Create a large world with varying terrain
        for room_x in 0..10 {
            for room_y in 0..10 {
                let mut room = RoomTerrain::new();
                
                // Create a mix of walkable and non-walkable terrain
                for x in 0..ROOM_SIZE {
                    for y in 0..ROOM_SIZE {
                        // Randomly create some walls
                        if (x + y) % 7 == 0 {
                            room.set_walkable(x, y, false);
                        } else {
                            room.set_walkable(x, y, true);
                        }
                    }
                }
                
                world_map.add_room(room_x, room_y, room);
            }
        }

        let pathfinder = PathFinder::new(world_map);

        // Benchmark multiple path finding scenarios
        let start_time = Instant::now();
        let mut total_paths_found = 0;
        let mut total_path_length = 0;

        // Test paths across multiple rooms with different start and end points
        for _ in 0..1000 {
            let start_room_x = rand::random::<i32>() % 10;
            let start_room_y = rand::random::<i32>() % 10;
            let start_x = rand::random::<u8>() % ROOM_SIZE as u8;
            let start_y = rand::random::<u8>() % ROOM_SIZE as u8;

            let goal_room_x = rand::random::<i32>() % 10;
            let goal_room_y = rand::random::<i32>() % 10;
            let goal_x = rand::random::<u8>() % ROOM_SIZE as u8;
            let goal_y = rand::random::<u8>() % ROOM_SIZE as u8;

            if let Some(path) = pathfinder.find_path_multiroom(
                start_room_x, start_room_y, start_x, start_y,
                goal_room_x, goal_room_y, goal_x, goal_y
            ) {
                total_paths_found += 1;
                total_path_length += path.len();
            }
        }

        let duration = start_time.elapsed();

        println!("Pathfinding Benchmark Results:");
        println!("Total Paths Found: {}", total_paths_found);
        println!("Average Path Length: {}", 
            if total_paths_found > 0 { total_path_length / total_paths_found } else { 0 }
        );
        println!("Total Execution Time: {:?}", duration);
    }

    #[test]
    fn test_hsv_to_rgb() {
        // Test some known color conversions
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), (255, 0, 0)); // Red
        assert_eq!(hsv_to_rgb(120.0, 1.0, 1.0), (0, 255, 0)); // Green
        assert_eq!(hsv_to_rgb(240.0, 1.0, 1.0), (0, 0, 255)); // Blue
    }

    #[test]
    fn test_pathfinding_basic() {
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
    fn test_pathfinding_wall_avoidance() {
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