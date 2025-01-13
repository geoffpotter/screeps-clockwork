use std::time::Instant;
use std::str::FromStr;
use std::fs::File;
use std::io::Write;
use screeps::{Position, RoomName, RoomCoordinate};
use rand::Rng;
use super::{
    Array4DMap, BitPackedMap, CachedMultiroomMap, CachedRoomArrayMap, CachedRunLengthMap, CachedSparseBlockMap, ChunkedZOrderMap, DenseHashMap, FlatArrayMap, GlobalArrayMap, GlobalPoint, HashGridMap, HierarchicalGridMap, MapTrait, PrefixTreeMap, QuadtreeMap, RleZOrderMap, RoomArrayMap, RunLengthDeltaMap, SimpleHashMap, SparseBlockMap, VectorArrayMap, ZOrderGlobalMap
};
use crate::datatypes::MultiroomDistanceMap;
use plotters::prelude::*;

const IMPLEMENTATIONS: &[(&str, bool)] = &[
    ("MultiroomDistMap", true),  // Always keep this one enabled

    // massive memory usage
    ("GlobalArrayMap", true),
    ("ZOrderGlobalMap", true),

    // very slow
    ("RleZOrderMap", true),
    ("CachedRunLengthMap", true),
    ("RunLengthDeltaMap", true),


    // pretty slow
    ("HashGridMap", true),
    ("ChunkedZOrderMap", true),
    ("QuadtreeMap", true),
    ("SparseBlockMap", true),
    ("PrefixTreeMap", true),

    ("HierarchGridMap", true),
    ("RoomArrayMap", true),
    ("VectorArrayMap", true),
    ("Array4DMap", true),
    ("FlatArrayMap", true),
    ("DenseHashMap", true),
    ("BitPackedMap", true),
    ("CachedMultiroomMap", true),
    ("CachedRoomArrayMap", true),
    ("CachedSparseBlock", true),
    ("SimpleHashMap", true),
];

const MIN_POINTS: usize = 1;
const MAX_POINTS: usize = 10_000;
const NUM_TEST_POINTS: usize = 25;

#[cfg(test)]
mod tests {
    use super::*;

    const ITERATIONS: usize = 5;  // Reduced from 20

    fn run_benchmark_for_size(room_radius: i32, size_name: &str) {
        println!("\nBenchmarking enabled implementations - {}", size_name);
        println!("Testing area: {}x{} rooms (from -{} to +{})", 
            room_radius * 2 + 1,
            room_radius * 2 + 1,
            room_radius,
            room_radius);
        println!("Running {} iterations\n", ITERATIONS);

        // Generate test points once for each pattern
        let (points, positions, transitions, gen_time) = generate_test_points(room_radius, TestPattern::Spiral);
        let mut spiral_results = run_benchmarks(&points, &positions, transitions, ITERATIONS);
        for result in spiral_results.iter_mut() {
            result.gen_time_ms = gen_time;
        }
        println!("=== Spiral Pattern Test ===");
        print_comparison("Spiral Pattern Test", &spiral_results);

        let (points, positions, transitions, gen_time) = generate_test_points(room_radius, TestPattern::ZigzagRoomTransitions);
        let mut zigzag_results = run_benchmarks(&points, &positions, transitions, ITERATIONS);
        for result in zigzag_results.iter_mut() {
            result.gen_time_ms = gen_time;
        }
        println!("\n=== Zigzag Room Transitions Test ===");
        print_comparison("Zigzag Room Transitions Test", &zigzag_results);

        let (points, positions, transitions, gen_time) = generate_test_points(room_radius, TestPattern::FloodFill);
        let mut flood_results = run_benchmarks(&points, &positions, transitions, ITERATIONS);
        for result in flood_results.iter_mut() {
            result.gen_time_ms = gen_time;
        }
        println!("\n=== Flood Fill Pattern Test ===");
        print_comparison("Flood Fill Pattern Test", &flood_results);

        // Save results to file
        if let Err(e) = save_benchmark_results(size_name, &spiral_results, &zigzag_results, &flood_results) {
            println!("Error saving benchmark results: {}", e);
        }
    }

    #[test]
    fn benchmark_single_room() {
        run_benchmark_for_size(0, "Single Room (1x1 rooms)");
    }

    #[test]
    fn benchmark_small() {
        run_benchmark_for_size(1, "Small (3x3 rooms)");
    }

    #[test]
    fn benchmark_medium() {
        run_benchmark_for_size(3, "Medium (7x7 rooms)");
    }

    #[test]
    fn benchmark_large() {
        run_benchmark_for_size(5, "Large (11x11 rooms)");
    }

    #[test]
    fn benchmark_huge() {
        run_benchmark_for_size(10, "Huge (21x21 rooms)");
    }


    #[test]
    fn benchmark_scaling() {
        println!("\nGenerating performance scaling graphs...");
        
        // Generate exponentially distributed points with more concentration at lower values
        let point_counts: Vec<usize> = (0..NUM_TEST_POINTS)
            .map(|i| {
                let t = (i as f64) / (NUM_TEST_POINTS - 1) as f64;
                // Use exponential function to concentrate points at lower values
                let exp_t = t.powf(3.0);  // Cube t to get more concentration at lower values
                let value = (MIN_POINTS as f64) * ((MAX_POINTS as f64) / (MIN_POINTS as f64)).powf(exp_t);
                value.round() as usize
            })
            .collect();
        
        let patterns = [
            TestPattern::Spiral,
            TestPattern::ZigzagRoomTransitions,
            TestPattern::FloodFill
        ];
        
        let mut all_set_times = Vec::new();
        let mut all_get_times = Vec::new();
        let mut all_memory_per_point = Vec::new();
        let mut all_total_memory = Vec::new();
        
        for pattern in patterns.iter() {
            let mut set_times = Vec::new();
            let mut get_times = Vec::new();
            let mut memory_per_point = Vec::new();
            let mut total_memory = Vec::new();
            
            // Collect data for each implementation
            for (name, enabled) in IMPLEMENTATIONS.iter() {
                if !enabled {
                    continue;
                }
                
                println!("Testing {} with {:?} pattern", name, pattern);
                let mut impl_set_times = Vec::new();
                let mut impl_get_times = Vec::new();
                let mut impl_memory_per_point = Vec::new();
                let mut impl_total_memory = Vec::new();
                let iterations = 100;
                
                for &count in &point_counts {
                    // Calculate room radius needed for target point count
                    let room_radius = ((count as f64).sqrt() / 500.0).ceil() as i32;
                    let (points, positions, transitions, _) = generate_test_points(room_radius, *pattern);
                    
                    // Only use the first 'count' points
                    let points = &points[..count.min(points.len())];
                    let positions = &positions[..count.min(positions.len())];
                    
                    let results = match *name {
                        "ZOrderGlobalMap" => benchmark_implementation::<ZOrderGlobalMap>(points, positions, transitions, iterations),
                        "ChunkedZOrderMap" => benchmark_implementation::<ChunkedZOrderMap>(points, positions, transitions, iterations),
                        "MultiroomDistMap" => benchmark_implementation::<MultiroomDistanceMap>(points, positions, transitions, iterations),
                        "QuadtreeMap" => benchmark_implementation::<QuadtreeMap>(points, positions, transitions, iterations),
                        "HierarchGridMap" => benchmark_implementation::<HierarchicalGridMap>(points, positions, transitions, iterations),
                        "HashGridMap" => benchmark_implementation::<HashGridMap>(points, positions, transitions, iterations),
                        "SparseBlockMap" => benchmark_implementation::<SparseBlockMap>(points, positions, transitions, iterations),
                        "PrefixTreeMap" => benchmark_implementation::<PrefixTreeMap>(points, positions, transitions, iterations),
                        "GlobalArrayMap" => benchmark_implementation::<GlobalArrayMap>(points, positions, transitions, iterations),
                        "RoomArrayMap" => benchmark_implementation::<RoomArrayMap>(points, positions, transitions, iterations),
                        "VectorArrayMap" => benchmark_implementation::<VectorArrayMap>(points, positions, transitions, iterations),
                        "Array4DMap" => benchmark_implementation::<Array4DMap>(points, positions, transitions, iterations),
                        "FlatArrayMap" => benchmark_implementation::<FlatArrayMap>(points, positions, transitions, iterations),
                        "DenseHashMap" => benchmark_implementation::<DenseHashMap>(points, positions, transitions, iterations),
                        "BitPackedMap" => benchmark_implementation::<BitPackedMap>(points, positions, transitions, iterations),
                        "CachedMultiroomMap" => benchmark_implementation::<CachedMultiroomMap>(points, positions, transitions, iterations),
                        "CachedRoomArrayMap" => benchmark_implementation::<CachedRoomArrayMap>(points, positions, transitions, iterations),
                        "CachedSparseBlock" => benchmark_implementation::<CachedSparseBlockMap>(points, positions, transitions, iterations),
                        "SimpleHashMap" => benchmark_implementation::<SimpleHashMap>(points, positions, transitions, iterations),
                        "RleZOrderMap" => benchmark_implementation::<RleZOrderMap>(points, positions, transitions, iterations),
                        "CachedRunLengthMap" => benchmark_implementation::<CachedRunLengthMap>(points, positions, transitions, iterations),
                        "RunLengthDeltaMap" => benchmark_implementation::<RunLengthDeltaMap>(points, positions, transitions, iterations),
                        _ => panic!("Unknown implementation: {}", name),
                    };
                    
                    impl_set_times.push((count as f64, results.set_time_ms * 1000.0 / count as f64)); // µs per point
                    impl_get_times.push((count as f64, results.get_time_ms * 1000.0 / count as f64)); // µs per point
                    impl_memory_per_point.push((count as f64, results.memory_bytes as f64 / count as f64)); // bytes per point
                    impl_total_memory.push((count as f64, results.memory_bytes as f64)); // total bytes
                }
                
                set_times.push((*name, impl_set_times));
                get_times.push((*name, impl_get_times));
                memory_per_point.push((*name, impl_memory_per_point));
                total_memory.push((*name, impl_total_memory));
            }
            
            all_set_times.push(set_times);
            all_get_times.push(get_times);
            all_memory_per_point.push(memory_per_point);
            all_total_memory.push(total_memory);
        }
        
        // Generate combined graphs
        generate_pattern_graphs(
            &all_set_times,
            &all_get_times,
            &all_memory_per_point,
            &all_total_memory,
            &patterns
        ).expect("Failed to generate performance graphs");
        
        println!("Graphs generated successfully");
    }

}

#[derive(Debug, Clone, Copy)]
enum TestPattern {
    Spiral,
    ZigzagRoomTransitions,
    FloodFill,
}

pub struct BenchmarkStats {
    pub z_set_times: Vec<f64>,
    pub z_get_times: Vec<f64>,
    pub mr_set_times: Vec<f64>,
    pub mr_get_times: Vec<f64>,
    pub total_points: usize,
    pub total_transitions: usize,
    pub z_memory: Vec<usize>,
    pub mr_memory: Vec<usize>,
}

impl BenchmarkStats {
    pub fn new() -> Self {
        Self {
            z_set_times: Vec::new(),
            z_get_times: Vec::new(),
            mr_set_times: Vec::new(),
            mr_get_times: Vec::new(),
            total_points: 0,
            total_transitions: 0,
            z_memory: Vec::new(),
            mr_memory: Vec::new(),
        }
    }

    pub fn print_stats(&self, iterations: usize) {
        let avg_points = self.total_points as f64 / iterations as f64;
        let avg_transitions = self.total_transitions as f64 / iterations as f64;
        let transitions_per_point = avg_transitions / avg_points;
        
        let z_total_memory = self.z_memory.iter().sum::<usize>();
        let mr_total_memory = self.mr_memory.iter().sum::<usize>();

        let z_set_avg = self.z_set_times.iter().sum::<f64>() / iterations as f64;
        let mr_set_avg = self.mr_set_times.iter().sum::<f64>() / iterations as f64;
        let z_get_avg = self.z_get_times.iter().sum::<f64>() / iterations as f64;
        let mr_get_avg = self.mr_get_times.iter().sum::<f64>() / iterations as f64;

        println!("\n{:<15} | {:<24} | {:<24} | {:<20}", 
            "Metric", "ZOrderGlobalMap", "MultiRoomMap", "Difference");
        println!("{:-<15}-+-{:-<24}-+-{:-<24}-+-{:-<20}", "", "", "", "");
        
        // Points and transitions
        println!("{:<15} | {:<24.0} | {:<24.0} | {:<20}", 
            "Points", avg_points, avg_points, "-");
        println!("{:<15} | {:<24.0} | {:<24.0} | {:<20}", 
            "Transitions", avg_transitions, avg_transitions, "-");
        println!("{:<15} | {:<24.3} | {:<24.3} | {:<20}", 
            "Trans/point", transitions_per_point, transitions_per_point, "-");
        
        // Timing stats
        let set_diff = (z_set_avg - mr_set_avg) * 1000.0;
        let set_pct = ((z_set_avg / mr_set_avg) - 1.0) * 100.0;
        let z_set_per_point = (z_set_avg * 1_000_000.0) / avg_points;
        let mr_set_per_point = (mr_set_avg * 1_000_000.0) / avg_points;
        println!("{:<15} | {:>8.3} ({:>6.3} µs) | {:>8.3} ({:>6.3} µs) | {:+.1} ms ({:+.1}%)", 
            "Set time (ms)", 
            z_set_avg * 1000.0, z_set_per_point,
            mr_set_avg * 1000.0, mr_set_per_point,
            set_diff, set_pct);

        let get_diff = (z_get_avg - mr_get_avg) * 1000.0;
        let get_pct = ((z_get_avg / mr_get_avg) - 1.0) * 100.0;
        let z_get_per_point = (z_get_avg * 1_000_000.0) / avg_points;
        let mr_get_per_point = (mr_get_avg * 1_000_000.0) / avg_points;
        println!("{:<15} | {:>8.3} ({:>6.3} µs) | {:>8.3} ({:>6.3} µs) | {:+.1} ms ({:+.1}%)", 
            "Get time (ms)", 
            z_get_avg * 1000.0, z_get_per_point,
            mr_get_avg * 1000.0, mr_get_per_point,
            get_diff, get_pct);
        
        // Memory stats
        let mem_per_point_z = z_total_memory as f64 / (iterations as f64 * avg_points);
        let mem_per_point_mr = mr_total_memory as f64 / (iterations as f64 * avg_points);
        let mem_diff = mem_per_point_z - mem_per_point_mr;
        let mem_pct = ((mem_per_point_z / mem_per_point_mr) - 1.0) * 100.0;
        println!("{:<15} | {:>8.2} B          | {:>8.2} B          | {:+.2} B ({:+.1}%)",
            "Memory/point", mem_per_point_z, mem_per_point_mr, mem_diff, mem_pct);
        
        let total_mem_z = z_total_memory as f64 / (1024.0 * 1024.0);
        let total_mem_mr = mr_total_memory as f64 / (1024.0 * 1024.0);
        let total_mem_diff = total_mem_z - total_mem_mr;
        let total_mem_pct = ((total_mem_z / total_mem_mr) - 1.0) * 100.0;
        println!("{:<15} | {:>8.2} MB         | {:>8.2} MB         | {:+.2} MB ({:+.1}%)",
            "Total memory", total_mem_z, total_mem_mr, total_mem_diff, total_mem_pct);
    }
}

pub fn print_timing_stats(name: &str, times: &[f64], points: f64) {
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let min = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let variance = times.iter().map(|x| (x - avg).powi(2)).sum::<f64>() / times.len() as f64;
    let std_dev = variance.sqrt();

    println!("  {} time: {:.3} ms (avg), {:.3} ms (min), {:.3} ms (max), {:.3} ms (std dev)", 
        name, avg * 1000.0, min * 1000.0, max * 1000.0, std_dev * 1000.0);
    println!("  {} time per point: {:.3} µs", 
        name, (avg * 1_000_000.0) / points);
}


#[test]
fn benchmark_room_parse_failure() {
    use std::time::Instant;
    
    println!("\nBenchmarking room name parse failures");
    println!("Running 1,000,000 iterations\n");
    
    // Create an invalid room name that will fail to parse
    let invalid_room = "N5W1";  // Wrong order, should be W1N5
    
    let start = Instant::now();
    for _ in 0..250_000 {
        let _ = RoomName::from_str(invalid_room);
    }
    let elapsed = start.elapsed();
    println!("elapsed: {:?}", elapsed);
    
    println!("{:<30} | {:<20}", "Metric", "Value");
    println!("{:-<30}-+-{:-<20}", "", "");
    println!("{:<30} | {:<20.3}", "Total time (ms)", elapsed.as_secs_f64() * 1000.0);
    println!("{:<30} | {:<20.3}", "Time per parse (µs)", 
        (elapsed.as_secs_f64() * 1_000_000.0) / 1_000_000.0);
} 

#[derive(Debug)]
struct BenchmarkResults {
    points: usize,
    transitions: usize,
    gen_time_ms: f64,
    init_time_ms: f64,  // Add initialization time
    set_time_ms: f64,
    get_time_ms: f64,
    memory_bytes: usize,
}


fn print_comparison(test_name: &str, results: &[BenchmarkResults]) {
    println!("\n{}\n", test_name);
    
    // Find the MultiroomDistanceMap results for comparison
    let multiroom_idx = IMPLEMENTATIONS.iter()
        .position(|(name, enabled)| *name == "MultiroomDistMap" && *enabled)
        .expect("MultiroomDistMap must be enabled");
    
    let multiroom_points = results[multiroom_idx].points as f64;
    let multiroom_set_per_point = (results[multiroom_idx].set_time_ms * 1000.0) / multiroom_points;
    let multiroom_get_per_point = (results[multiroom_idx].get_time_ms * 1000.0) / multiroom_points;

    println!("Points: {}, Room Transitions: {}", results[0].points, results[0].transitions);
    println!("Position Generation: {:.2}ms ({:.3}µs/pt)", 
        results[0].gen_time_ms,
        results[0].gen_time_ms * 1000.0 / results[0].points as f64);
    println!();
    println!("Implementation        Init(ms)  Set(ms)   Set(µs/pt)  Set%    Get(ms)   Get(µs/pt)  Get%    Memory");
    println!("-------------------- --------- --------- ----------- ------- --------- ----------- ------- ----------");
    
    // First print MultiroomDistanceMap
    let multiroom_result = &results[multiroom_idx];
    let points = multiroom_result.points as f64;
    let set_per_point = (multiroom_result.set_time_ms * 1000.0) / points;
    let get_per_point = (multiroom_result.get_time_ms * 1000.0) / points;
    
    // Format memory size
    let memory_str = format_memory(multiroom_result.memory_bytes);
    println!("{:20} {:8.3} {:9.2} {:10.3} {:>7} {:9.2} {:10.3} {:>7} {:>9}",
        "MultiroomDistMap",
        multiroom_result.init_time_ms,
        multiroom_result.set_time_ms,
        set_per_point,
        0.0,  // Base comparison
        multiroom_result.get_time_ms,
        get_per_point,
        0.0,  // Base comparison
        memory_str);
    
    println!("-------------------- --------- --------- ----------- ------- --------- ----------- ------- ----------");
    
    // Create a vector of (index, result) pairs for sorting
    let mut indexed_results: Vec<_> = results.iter().enumerate()
        .filter(|(i, _)| *i != multiroom_idx)
        .collect();
    
    // Sort by get time
    indexed_results.sort_by(|(_, a), (_, b)| {
        a.get_time_ms.partial_cmp(&b.get_time_ms).unwrap()
    });
    
    // Then print the rest of the results
    let mut printed_gap = false;
    for (i, result) in indexed_results {
        let points = result.points as f64;
        let set_per_point = (result.set_time_ms * 1000.0) / points;
        let get_per_point = (result.get_time_ms * 1000.0) / points;
        
        // Calculate percentage differences
        let set_pct = ((result.set_time_ms / multiroom_result.set_time_ms) - 1.0) * 100.0;
        let get_pct = ((result.get_time_ms / multiroom_result.get_time_ms) - 1.0) * 100.0;
        
        // Add a blank line before the first implementation that's slower than multiroom
        if !printed_gap && result.get_time_ms > multiroom_result.get_time_ms {
            println!();
            printed_gap = true;
        }
        
        // Format percentages
        let set_pct_str = format_percentage(set_pct);
        let get_pct_str = format_percentage(get_pct);
        
        // Format memory size
        let memory_str = format_memory(result.memory_bytes);
        
        // Get implementation name from enabled implementations
        let name = IMPLEMENTATIONS.iter()
            .filter(|(_, enabled)| *enabled)
            .nth(i)
            .map(|(name, _)| *name)
            .unwrap_or("Unknown");
        
        // Print with aligned columns and percentage comparisons
        println!("{:20} {:8.3} {:9.2} {:10.3} {:>7} {:9.2} {:10.3} {:>7} {:>9}",
            name,
            result.init_time_ms,
            result.set_time_ms,
            set_per_point,
            set_pct_str,
            result.get_time_ms,
            get_per_point,
            get_pct_str,
            memory_str);
    }
    
    // Add a note about the percentage comparisons
    println!("\nNote: Set% and Get% show performance relative to MultiroomDistanceMap (+ is slower, - is faster)");
    println!("      Times are in milliseconds, per-point times in microseconds");
    println!("      Results are sorted by Get time (fastest to slowest)");
}

fn format_percentage(pct: f64) -> String {
    // if pct > 500.0 {
    //     "lol".to_string()
    // } else if pct > 100.0 {
    //     "bad".to_string()
    // } else {
    //     format!("{:+.1}%", pct)
    // }
    format!("{:+.1}%", pct)
}

fn format_memory(bytes: usize) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb > 1.0 {
        format!("{:.1}GB", gb)
    } else if mb > 1.0 {
        format!("{:.1}MB", mb)
    } else {
        let kb = bytes as f64 / 1024.0;
        if kb > 1.0 {
            format!("{:.1}KB", kb)
        } else {
            format!("{}B", bytes)
        }
    }
}

fn generate_test_points(room_radius: i32, pattern: TestPattern) -> (Vec<GlobalPoint>, Vec<Position>, usize, f64) {
    let start = Instant::now();
    let mut points = Vec::new();
    let mut positions = Vec::new();
    let mut transitions = 0;
    let mut last_room = None;

    match pattern {
        TestPattern::Spiral => {
            // Generate spiral pattern
            let mut x = 0;
            let mut y = 0;
            let mut dx = 1;
            let mut dy = 0;
            let mut segment_length = 1;
            let mut segment_passed = 0;
            let mut segments_done = 0;
            let room_size = 50;
            
            // Scale points with map size
            let total_rooms = (2 * room_radius + 1).pow(2) as usize;
            let points_per_room = 500;  // Base number of points for a single room
            let target_points = if room_radius == 0 { 
                points_per_room 
            } else { 
                total_rooms * points_per_room
            };
            
            // Keep generating points until we have enough valid ones
            while points.len() < target_points {
                let point = GlobalPoint { x, y };
                
                // For single room, only add points within room bounds
                if room_radius == 0 {
                    if x >= 0 && x < room_size && y >= 0 && y < room_size {
                        points.push(point);
                        let pos = Position::from_world_coords(x, y);
                        positions.push(pos);
                        let (room, _, _) = ZOrderGlobalMap::global_to_room(point);
                        if let Some(last) = last_room {
                            if last != room {
                                transitions += 1;
                            }
                        }
                        last_room = Some(room);
                    }
                } else {
                    // For multi-room, check against room radius
                    if (x as i32).abs() <= room_radius * room_size && 
                       (y as i32).abs() <= room_radius * room_size {
                        points.push(point);
                        let pos = Position::from_world_coords(x, y);
                        positions.push(pos);
                        let (room, _, _) = ZOrderGlobalMap::global_to_room(point);
                        if let Some(last) = last_room {
                            if last != room {
                                transitions += 1;
                            }
                        }
                        last_room = Some(room);
                    }
                }
                
                // Move to next point in spiral
                x += dx;
                y += dy;
                segment_passed += 1;
                
                if segment_passed == segment_length {
                    segment_passed = 0;
                    segments_done += 1;
                    match segments_done % 4 {
                        0 => segment_length += 1,
                        2 => segment_length += 1,
                        _ => {}
                    }
                    match segments_done % 4 {
                        0 => { dx = 0; dy = -1; }
                        1 => { dx = -1; dy = 0; }
                        2 => { dx = 0; dy = 1; }
                        3 => { dx = 1; dy = 0; }
                        _ => unreachable!()
                    }
                }
            }
        },
        TestPattern::ZigzagRoomTransitions => {
            let mut rng = rand::thread_rng();
            let points_per_room = 500;
            let total_rooms = (2 * room_radius + 1) * (2 * room_radius + 1);
            let total_points = total_rooms as usize * points_per_room;

            for _ in 0..total_points {
                let room_x = rng.gen_range(-room_radius..=room_radius);
                let room_y = rng.gen_range(-room_radius..=room_radius);
                let local_x = rng.gen_range(0..50);
                let local_y = rng.gen_range(0..50);
                let x = room_x * 50 + local_x;
                let y = room_y * 50 + local_y;
                let point = GlobalPoint { x, y };
                points.push(point);
                
                let (room, _, _) = ZOrderGlobalMap::global_to_room(point);
                if let Some(last) = last_room {
                    if last != room {
                        transitions += 1;
                    }
                }

                let pos = Position::from_world_coords(x, y);
                positions.push(pos);
                last_room = Some(room);
            }
        },
        TestPattern::FloodFill => {
            let room_size = 50;
            let total_rooms = (2 * room_radius + 1).pow(2) as usize;
            let points_per_room = 500;
            let target_points = if room_radius == 0 { 
                points_per_room 
            } else { 
                // Ensure we generate enough points to reach all rooms
                total_rooms * points_per_room * 2  // Double the points to ensure coverage
            };
            
            let mut point_queue = std::collections::VecDeque::new();
            let mut visited_points = std::collections::HashSet::new();
            let mut last_room = None;
            
            // Start at center point
            let start_x = 25;
            let start_y = 25;
            point_queue.push_back((start_x, start_y));
            visited_points.insert((start_x, start_y));
            
            // Use 8 directions for natural flood fill spread
            let directions = [
                (0, 1), (1, 0), (0, -1), (-1, 0),  // Cardinal
                (1, 1), (1, -1), (-1, 1), (-1, -1) // Diagonal
            ];
            
            while points.len() < target_points {
                if let Some((x, y)) = point_queue.pop_front() {
                    let point = GlobalPoint { x, y };
                    let room_x = (x as f64 / room_size as f64).floor() as i32;
                    let room_y = (y as f64 / room_size as f64).floor() as i32;
                    
                    // Check if point is within bounds
                    if room_radius == 0 {
                        if x >= 0 && x < room_size && y >= 0 && y < room_size {
                            points.push(point);
                            let pos = Position::from_world_coords(x, y);
                            positions.push(pos);
                            
                            // Track room transitions
                            let current_room = (room_x, room_y);
                            if let Some(last) = last_room {
                                if last != current_room {
                                    transitions += 1;
                                }
                            }
                            last_room = Some(current_room);
                        }
                    } else if room_x.abs() <= room_radius && room_y.abs() <= room_radius {
                        points.push(point);
                        let pos = Position::from_world_coords(x, y);
                        positions.push(pos);
                        
                        // Track room transitions
                        let current_room = (room_x, room_y);
                        if let Some(last) = last_room {
                            if last != current_room {
                                transitions += 1;
                            }
                        }
                        last_room = Some(current_room);
                    }
                    
                    // Add all neighboring points
                    for &(dx, dy) in &directions {
                        let next_x = x + dx;
                        let next_y = y + dy;
                        if visited_points.insert((next_x, next_y)) {
                            point_queue.push_back((next_x, next_y));
                        }
                    }
                }
            }
        }
    }

    let gen_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    (points, positions, transitions, gen_time_ms)
}

fn run_benchmarks(points: &[GlobalPoint], positions: &[Position], transitions: usize, iterations: usize) -> Vec<BenchmarkResults> {
    let mut results = Vec::new();
    
    for (name, enabled) in IMPLEMENTATIONS.iter() {
        if !enabled {
            continue;
        }
        
        let result = match *name {
            "ZOrderGlobalMap" => benchmark_implementation::<ZOrderGlobalMap>(points, positions, transitions, iterations),
            "ChunkedZOrderMap" => benchmark_implementation::<ChunkedZOrderMap>(points, positions, transitions, iterations),
            "MultiroomDistMap" => benchmark_implementation::<MultiroomDistanceMap>(points, positions, transitions, iterations),
            "QuadtreeMap" => benchmark_implementation::<QuadtreeMap>(points, positions, transitions, iterations),
            "HierarchGridMap" => benchmark_implementation::<HierarchicalGridMap>(points, positions, transitions, iterations),
            "HashGridMap" => benchmark_implementation::<HashGridMap>(points, positions, transitions, iterations),
            "SparseBlockMap" => benchmark_implementation::<SparseBlockMap>(points, positions, transitions, iterations),
            "PrefixTreeMap" => benchmark_implementation::<PrefixTreeMap>(points, positions, transitions, iterations),
            "GlobalArrayMap" => benchmark_implementation::<GlobalArrayMap>(points, positions, transitions, iterations),
            "RoomArrayMap" => benchmark_implementation::<RoomArrayMap>(points, positions, transitions, iterations),
            "VectorArrayMap" => benchmark_implementation::<VectorArrayMap>(points, positions, transitions, iterations),  // Add the new implementation
            "Array4DMap" => benchmark_implementation::<Array4DMap>(points, positions, transitions, iterations),
            "FlatArrayMap" => benchmark_implementation::<FlatArrayMap>(points, positions, transitions, iterations),
            "DenseHashMap" => benchmark_implementation::<DenseHashMap>(points, positions, transitions, iterations),
            "BitPackedMap" => benchmark_implementation::<BitPackedMap>(points, positions, transitions, iterations),
            "CachedMultiroomMap" => benchmark_implementation::<CachedMultiroomMap>(points, positions, transitions, iterations),
            "CachedRoomArrayMap" => benchmark_implementation::<CachedRoomArrayMap>(points, positions, transitions, iterations),
            "CachedSparseBlock" => benchmark_implementation::<CachedSparseBlockMap>(points, positions, transitions, iterations),
            "SimpleHashMap" => benchmark_implementation::<SimpleHashMap>(points, positions, transitions, iterations),
            "RleZOrderMap" => benchmark_implementation::<RleZOrderMap>(points, positions, transitions, iterations),
            "CachedRunLengthMap" => benchmark_implementation::<CachedRunLengthMap>(points, positions, transitions, iterations),
            "RunLengthDeltaMap" => benchmark_implementation::<RunLengthDeltaMap>(points, positions, transitions, iterations),
            _ => panic!("Unknown implementation: {}", name),
        };
        results.push(result);
    }
    
    results
}

fn benchmark_implementation<T: MapTrait>(points: &[GlobalPoint], positions: &[Position], transitions: usize, iterations: usize) -> BenchmarkResults {
    let mut set_time_total = 0.0;
    let mut get_time_total = 0.0;
    let mut init_time_total = 0.0;
    let mut memory_total = 0;

    for _ in 0..iterations {
        // Benchmark initialization - measure just the new() call
        let start = Instant::now();
        let map = T::new();
        init_time_total += start.elapsed().as_secs_f64();
        
        // Move map into mutable binding for operations
        let mut map = map;
        
        // Benchmark set operations
        let start = Instant::now();
        for (i, (&point, &pos)) in points.iter().zip(positions.iter()).enumerate() {
            map.set(point, pos, i);
        }
        set_time_total += start.elapsed().as_secs_f64();
        
        // Benchmark get operations
        let start = Instant::now();
        for (&point, &pos) in points.iter().zip(positions.iter()) {
            let _ = map.get(point, pos);
        }
        get_time_total += start.elapsed().as_secs_f64();
        
        memory_total += map.memory_usage();
    }

    BenchmarkResults {
        points: points.len(),
        transitions,
        gen_time_ms: 0.0,  // Placeholder for generation time
        init_time_ms: init_time_total * 1000.0 / iterations as f64,
        set_time_ms: set_time_total * 1000.0 / iterations as f64,
        get_time_ms: get_time_total * 1000.0 / iterations as f64,
        memory_bytes: memory_total / iterations,
    }
} 

fn save_benchmark_results(size_name: &str, spiral_results: &[BenchmarkResults], zigzag_results: &[BenchmarkResults], flood_results: &[BenchmarkResults]) -> std::io::Result<()> {
    let filename = format!("lib/datatypes/position_map/benchmark_results_{}.txt", size_name.to_lowercase().replace(" ", "_"));
    let mut file = File::create(filename)?;

    // Write header
    writeln!(file, "Benchmark Results - {}\n", size_name)?;
    
    // Write spiral test results
    writeln!(file, "=== Spiral Pattern Test ===")?;
    writeln!(file, "Points: {}, Room Transitions: {}", spiral_results[0].points, spiral_results[0].transitions)?;
    writeln!(file, "Position Generation: {:.2}ms ({:.3}µs/pt)", 
        spiral_results[0].gen_time_ms,
        spiral_results[0].gen_time_ms * 1000.0 / spiral_results[0].points as f64)?;
    writeln!(file)?;
    writeln!(file, "{:20} {:>8} {:>9} {:>11} {:>7} {:>9} {:>11} {:>7} {:>10}",
        "Implementation", "Init(ms)", "Set(ms)", "Set(µs/pt)", "Set%", "Get(ms)", "Get(µs/pt)", "Get%", "Memory")?;
    writeln!(file, "{:-<20} {:-<8} {:-<9} {:-<11} {:-<7} {:-<9} {:-<11} {:-<7} {:-<10}",
        "", "", "", "", "", "", "", "", "")?;
    
    write_results_section(&mut file, spiral_results)?;
    
    // Write zigzag test results
    writeln!(file, "\n\n=== Zigzag Room Transitions Test ===")?;
    writeln!(file, "Points: {}, Room Transitions: {}", zigzag_results[0].points, zigzag_results[0].transitions)?;
    writeln!(file, "Position Generation: {:.2}ms ({:.3}µs/pt)", 
        zigzag_results[0].gen_time_ms,
        zigzag_results[0].gen_time_ms * 1000.0 / zigzag_results[0].points as f64)?;
    writeln!(file)?;
    writeln!(file, "{:20} {:>8} {:>9} {:>11} {:>7} {:>9} {:>11} {:>7} {:>10}",
        "Implementation", "Init(ms)", "Set(ms)", "Set(µs/pt)", "Set%", "Get(ms)", "Get(µs/pt)", "Get%", "Memory")?;
    writeln!(file, "{:-<20} {:-<8} {:-<9} {:-<11} {:-<7} {:-<9} {:-<11} {:-<7} {:-<10}",
        "", "", "", "", "", "", "", "", "")?;
    
    write_results_section(&mut file, zigzag_results)?;
    
    // Write flood fill test results
    writeln!(file, "\n\n=== Flood Fill Pattern Test ===")?;
    writeln!(file, "Points: {}, Room Transitions: {}", flood_results[0].points, flood_results[0].transitions)?;
    writeln!(file, "Position Generation: {:.2}ms ({:.3}µs/pt)", 
        flood_results[0].gen_time_ms,
        flood_results[0].gen_time_ms * 1000.0 / flood_results[0].points as f64)?;
    writeln!(file)?;
    writeln!(file, "{:20} {:>8} {:>9} {:>11} {:>7} {:>9} {:>11} {:>7} {:>10}",
        "Implementation", "Init(ms)", "Set(ms)", "Set(µs/pt)", "Set%", "Get(ms)", "Get(µs/pt)", "Get%", "Memory")?;
    writeln!(file, "{:-<20} {:-<8} {:-<9} {:-<11} {:-<7} {:-<9} {:-<11} {:-<7} {:-<10}",
        "", "", "", "", "", "", "", "", "")?;
    
    write_results_section(&mut file, flood_results)?;
    
    writeln!(file, "\nNote: Set% and Get% show performance relative to MultiroomDistanceMap (+ is slower, - is faster)")?;
    writeln!(file, "      Times are in milliseconds, per-point times in microseconds")?;
    writeln!(file, "      Results are sorted by Get time (fastest to slowest)")?;
    
    Ok(())
}

fn write_results_section(file: &mut File, results: &[BenchmarkResults]) -> std::io::Result<()> {
    // Find MultiroomDistanceMap index
    let multiroom_idx = IMPLEMENTATIONS.iter()
        .position(|(name, enabled)| *name == "MultiroomDistMap" && *enabled)
        .expect("MultiroomDistMap must be enabled");
    
    let multiroom_result = &results[multiroom_idx];
    
    // First write MultiroomDistanceMap results
    write_result_line(file, "MultiroomDistMap", multiroom_result, multiroom_result)?;
    
    writeln!(file, "{:-<20} {:-<8} {:-<9} {:-<11} {:-<7} {:-<9} {:-<11} {:-<7} {:-<10}",
        "", "", "", "", "", "", "", "", "")?;
    
    // Create sorted results excluding MultiroomDistanceMap
    let mut indexed_results: Vec<_> = results.iter().enumerate()
        .filter(|(i, _)| *i != multiroom_idx)
        .collect();
    
    indexed_results.sort_by(|(_, a), (_, b)| {
        a.get_time_ms.partial_cmp(&b.get_time_ms).unwrap()
    });
    
    // Write remaining results
    let mut printed_gap = false;
    for (i, result) in indexed_results {
        if !printed_gap && result.get_time_ms > multiroom_result.get_time_ms {
            writeln!(file)?;
            printed_gap = true;
        }
        
        let name = IMPLEMENTATIONS.iter()
            .filter(|(_, enabled)| *enabled)
            .nth(i)
            .map(|(name, _)| *name)
            .unwrap_or("Unknown");
        
        write_result_line(file, name, result, multiroom_result)?;
    }
    
    Ok(())
}

fn write_result_line(file: &mut File, name: &str, result: &BenchmarkResults, multiroom_result: &BenchmarkResults) -> std::io::Result<()> {
    let points = result.points as f64;
    let set_per_point = (result.set_time_ms * 1000.0) / points;
    let get_per_point = (result.get_time_ms * 1000.0) / points;
    
    let set_pct = ((result.set_time_ms / multiroom_result.set_time_ms) - 1.0) * 100.0;
    let get_pct = ((result.get_time_ms / multiroom_result.get_time_ms) - 1.0) * 100.0;
    
    let set_pct_str = format_percentage(set_pct);
    let get_pct_str = format_percentage(get_pct);
    let memory_str = format_memory(result.memory_bytes);
    
    writeln!(file, "{:20} {:8.3} {:9.2} {:10.3} {:>7} {:9.2} {:10.3} {:>7} {:>9}",
        name,
        result.init_time_ms,
        result.set_time_ms,
        set_per_point,
        set_pct_str,
        result.get_time_ms,
        get_per_point,
        get_pct_str,
        memory_str)
} 

fn generate_pattern_graphs(
    set_times: &[Vec<(&str, Vec<(f64, f64)>)>],
    get_times: &[Vec<(&str, Vec<(f64, f64)>)>],
    memory_per_point: &[Vec<(&str, Vec<(f64, f64)>)>],
    total_memory: &[Vec<(&str, Vec<(f64, f64)>)>],
    patterns: &[TestPattern]
) -> Result<(), Box<dyn std::error::Error>> {
    let path = "lib/datatypes/position_map/benchmark_scaling.png";
    let root = BitMapBackend::new(path, (3072, 3072)).into_drawing_area();
    root.fill(&WHITE)?;
    
    // Split into 4x3 grid (4 metrics x 3 patterns)
    let areas = root.split_evenly((4, 3));
    
    // Define a color palette
    let colors = [
        RGBColor(0, 0, 255),    // Blue
        RGBColor(255, 0, 0),    // Red
        RGBColor(0, 255, 0),    // Green
        RGBColor(128, 0, 128),  // Purple
        RGBColor(255, 165, 0),  // Orange
        RGBColor(0, 128, 128),  // Teal
        RGBColor(255, 192, 203),// Pink
        RGBColor(165, 42, 42),  // Brown
    ];

    // Helper function to draw a single graph
    let draw_graph = |area: &DrawingArea<BitMapBackend, _>, 
                     data: &[(&str, Vec<(f64, f64)>)],
                     title: &str,
                     y_label: &str| -> Result<(), Box<dyn std::error::Error>> {
        let max_x = data.iter()
            .flat_map(|(_, points)| points.iter().map(|(x, _)| *x))
            .fold(0.0_f64, |a, b| a.max(b));
        let max_y = data.iter()
            .flat_map(|(_, points)| points.iter().map(|(_, y)| *y))
            .fold(0.0_f64, |a, b| a.max(b));
        
        // Calculate nice bounds for y-axis
        let log_max_y = max_y.ln();
        let y_magnitude = log_max_y.exp().log10().floor();
        let y_max = (10.0_f64).powf(y_magnitude + 1.0);
        
        let mut chart = ChartBuilder::on(area)
            .caption(title, ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                (MIN_POINTS as f64..MAX_POINTS as f64 * 1.1).log_scale(),
                (0.0..y_max).log_scale(),
            )?;
        
        chart.configure_mesh()
            .x_desc("Number of Points")
            .y_desc(y_label)
            .draw()?;
        
        // Plot each implementation's data
        for (i, (name, points)) in data.iter().enumerate() {
            let color = colors[i % colors.len()];
            chart.draw_series(LineSeries::new(
                points.iter().copied(),
                color,
            ))?.label(*name)
             .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
        }
        
        // Draw the legend
        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;
            
        Ok(())
    };
    
    // Draw graphs for each pattern
    for (pattern_idx, pattern) in patterns.iter().enumerate() {
        let pattern_name = match pattern {
            TestPattern::Spiral => "Spiral",
            TestPattern::ZigzagRoomTransitions => "Zigzag",
            TestPattern::FloodFill => "Flood Fill",
        };
        
        // Set Time
        draw_graph(
            &areas[pattern_idx],
            &set_times[pattern_idx],
            &format!("Set Time vs Points ({})", pattern_name),
            "Set Time (µs/point)"
        )?;
        
        // Get Time
        draw_graph(
            &areas[pattern_idx + 3],
            &get_times[pattern_idx],
            &format!("Get Time vs Points ({})", pattern_name),
            "Get Time (µs/point)"
        )?;
        
        // Memory per point
        draw_graph(
            &areas[pattern_idx + 6],
            &memory_per_point[pattern_idx],
            &format!("Memory per Point ({})", pattern_name),
            "Memory (bytes/point)"
        )?;
        
        // Total memory
        draw_graph(
            &areas[pattern_idx + 9],
            &total_memory[pattern_idx],
            &format!("Total Memory ({})", pattern_name),
            "Memory (bytes)"
        )?;
    }
    
    root.present()?;
    Ok(())
} 