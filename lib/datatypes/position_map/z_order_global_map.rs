use std::collections::BTreeMap;
use std::collections::HashSet;
use std::time::Instant;
use screeps::{Position, RoomName};
use std::str::FromStr;
use super::GlobalPoint;

/// Global map using Z-order curve for efficient distance storage
pub struct ZOrderGlobalMap {
    values: Box<[usize]>,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl ZOrderGlobalMap {
    const SIZE_BITS: usize = 14;  // Enough bits to cover -6250 to +6250
    const COORD_OFFSET: i32 = 6250;  // Keeps our original offset
    const SIZE: usize = 1 << (Self::SIZE_BITS * 2);
    
    pub fn new() -> Self {
        Self {
            values: vec![usize::MAX; Self::SIZE].into_boxed_slice(),
            min_x: i32::MAX,
            max_x: i32::MIN,
            min_y: i32::MAX,
            max_y: i32::MIN,
        }
    }

    /// Convert x,y coordinates to x-major index
    pub fn x_major_index(x: i32, y: i32) -> usize {
        (x + Self::COORD_OFFSET) as usize * Self::COORD_OFFSET as usize + (y + Self::COORD_OFFSET) as usize
    }

    /// Convert x,y coordinates to z-order curve index
    #[inline(always)]
    pub fn xy_to_z(x: i32, y: i32) -> usize {
        // Shift coordinates to positive space
        let x_pos = (x + Self::COORD_OFFSET) as usize;
        let y_pos = (y + Self::COORD_OFFSET) as usize;
        
        // Mask to ensure we only use SIZE_BITS bits
        let x_masked = x_pos & ((1 << Self::SIZE_BITS) - 1);
        let y_masked = y_pos & ((1 << Self::SIZE_BITS) - 1);
        
        // Interleave bits directly using a faster method for larger numbers
        let mut z = 0;
        let mut x_temp = x_masked;
        let mut y_temp = y_masked;
        
        for i in 0..Self::SIZE_BITS {
            z |= (x_temp & 1) << (2 * i);
            z |= (y_temp & 1) << (2 * i + 1);
            x_temp >>= 1;
            y_temp >>= 1;
        }
        
        z
    }

    /// Convert z-order index back to x,y coordinates
    #[inline(always)]
    pub fn z_to_xy(z: usize) -> (i32, i32) {
        let mut x = 0;
        let mut y = 0;
        let mut z_temp = z;
        
        // De-interleave bits using a faster method for larger numbers
        for i in 0..Self::SIZE_BITS {
            x |= (z_temp & 1) << i;
            z_temp >>= 1;
            y |= (z_temp & 1) << i;
            z_temp >>= 1;
        }
        
        // Convert back to original coordinate space
        let final_x = x as i32 - Self::COORD_OFFSET;
        let final_y = y as i32 - Self::COORD_OFFSET;
        
        (final_x, final_y)
    }

    /// Set a distance value for a global point
    #[inline(always)]
    pub fn set(&mut self, point: GlobalPoint, value: usize) {
        let z = Self::xy_to_z(point.x, point.y);
        self.values[z] = value;
        self.min_x = self.min_x.min(point.x);
        self.max_x = self.max_x.max(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_y = self.max_y.max(point.y);
    }

    /// Get a distance value for a global point
    #[inline(always)]
    pub fn get(&self, point: GlobalPoint) -> usize {
        let z = Self::xy_to_z(point.x, point.y);
        self.values[z]
    }

    /// Clear all stored values
    pub fn clear(&mut self) {
        self.values.fill(usize::MAX);
        self.min_x = i32::MAX;
        self.max_x = i32::MIN;
        self.min_y = i32::MAX;
        self.max_y = i32::MIN;
    }

    /// Get the current bounds of stored values
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        (self.min_x, self.max_x, self.min_y, self.max_y)
    }

    /// Get the total memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.values.len() * std::mem::size_of::<usize>()
    }

    /// Convert room coordinates to global point
    pub fn room_to_global(room_name: &str, x: u32, y: u32) -> Option<GlobalPoint> {
        // Parse room name like "E10N2" or "W4S15"
        let mut chars = room_name.chars().peekable();
        
        // Get EW direction and value
        let ew = chars.next()?;
        assert!(ew == 'E' || ew == 'W', "Invalid E/W direction: {}", ew);
        
        let mut ew_num = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_numeric() {
                ew_num.push(chars.next()?);
            } else {
                break;
            }
        }
        let ew_val: i32 = ew_num.parse().ok()?;
        
        // Get NS direction and value
        let ns = chars.next()?;
        assert!(ns == 'N' || ns == 'S', "Invalid N/S direction: {}", ns);
        
        let mut ns_num = String::new();
        while let Some(c) = chars.next() {
            if c.is_numeric() {
                ns_num.push(c);
            } else {
                break;
            }
        }
        assert!(!ns_num.is_empty(), "Missing N/S value");
        let ns_val: i32 = ns_num.parse().ok()?;
        
        // Calculate global coordinates
        let base_x = match ew {
            'W' => -(ew_val * 50),
            'E' => ew_val * 50,
            _ => return None,
        };
        
        let base_y = match ns {
            'N' => -(ns_val * 50),
            'S' => ns_val * 50,
            _ => return None,
        };
        
        Some(GlobalPoint {
            x: base_x + x as i32,
            y: base_y + y as i32,
        })
    }

    /// Convert global point to room coordinates
    pub fn global_to_room(point: GlobalPoint) -> (String, u32, u32) {
        let room_x = point.x.div_euclid(50);
        let room_y = point.y.div_euclid(50);
        
        let local_x = point.x.rem_euclid(50) as u32;
        let local_y = point.y.rem_euclid(50) as u32;
        
        let room_name = format!("{}{}{}{}",
            if room_x < 0 { "W" } else { "E" },
            room_x.abs(),
            if room_y > 0 { "S" } else { "N" },
            room_y.abs()
        );
        
        (room_name, local_x, local_y)
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;


    #[test]
    fn test_z_order_vs_x_major() {
        // benchmark making a z-index vs x-major index
        let mut points = vec![];
        let mut rng = rand::thread_rng();
        let iterations = 1_000_000;
        for _ in 0..iterations {
            let x = rng.gen_range(-1000..1000);
            let y = rng.gen_range(-1000..1000);
            points.push(GlobalPoint { x, y });
        }
        let now = Instant::now();
        for point in points.iter() {
            let _ = ZOrderGlobalMap::xy_to_z(point.x, point.y);
        }
        let z_order_elapsed = now.elapsed();

        let now = Instant::now();
        for point in points.iter() {
            let _ = ZOrderGlobalMap::x_major_index(point.x, point.y);
        }
        let x_major_elapsed = now.elapsed();
        println!("Time taken for {} z-index calcs: {:?} total, {:.3} µs per point", iterations, z_order_elapsed, (z_order_elapsed.as_secs_f64() / iterations as f64) * 1_000_000.0);
        println!("Time taken for {} x-major calcs: {:?} total, {:.3} µs per point", iterations, x_major_elapsed, (x_major_elapsed.as_secs_f64() / iterations as f64) * 1_000_000.0);


    }
    #[test]
    fn test_z_order_conversion() {
        let test_cases = vec![
            (123, 456),
            (-123, -456),
            (-6250, -6250),  // Maximum negative coordinates
            (6249, 6249),    // Maximum positive coordinates
            (0, 0),          // Origin
        ];

        for (x, y) in test_cases {
            let z = ZOrderGlobalMap::xy_to_z(x, y);
            let (new_x, new_y) = ZOrderGlobalMap::z_to_xy(z);
            assert_eq!((x, y), (new_x, new_y), "Failed for coordinates ({}, {})", x, y);
        }
    }

    #[test]
    fn test_room_conversion() {
        // Test room to global
        let global = ZOrderGlobalMap::room_to_global("W4N0", 25, 25).unwrap();
        assert_eq!(global.x, -175, "Global x coordinate should be -175");
        assert_eq!(global.y, 25, "Global y coordinate should be 25");
        
        // Test global to room
        let (room_name, x, y) = ZOrderGlobalMap::global_to_room(global);
        assert_eq!(room_name, "W4N0", "Room name should be W4N0");
        assert_eq!(x, 25, "Local x coordinate should be 25");
        assert_eq!(y, 25, "Local y coordinate should be 25");
        
        // Additional test cases with different formats
        let test_cases = vec![
            ("E10N2", 25, 25, 525, -75),
            ("W4S15", 10, 10, -190, 760),
            ("E0N0", 0, 0, 0, 0),
        ];
        
        for (room, local_x, local_y, expected_global_x, expected_global_y) in test_cases {
            let global = ZOrderGlobalMap::room_to_global(room, local_x, local_y).unwrap();
            assert_eq!(global.x, expected_global_x, "Global x coordinate mismatch for {}", room);
            assert_eq!(global.y, expected_global_y, "Global y coordinate mismatch for {}", room);
            
            let (converted_room, converted_x, converted_y) = ZOrderGlobalMap::global_to_room(global);
            assert_eq!(converted_room, room, "Room name mismatch for {}", room);
            assert_eq!(converted_x, local_x, "Local x coordinate mismatch for {}", room);
            assert_eq!(converted_y, local_y, "Local y coordinate mismatch for {}", room);
        }
    }

    #[test]
    fn test_set_get() {
        let mut map = ZOrderGlobalMap::new();
        let point = GlobalPoint { x: 100, y: 200 };
        
        map.set(point, 42);
        assert_eq!(map.get(point), 42);
        
        // Test point not set
        let missing_point = GlobalPoint { x: 101, y: 201 };
        assert_eq!(map.get(missing_point), usize::MAX);
    }

    #[test]
    fn visualize_z_order_path() {
        use std::fs::File;
        use std::io::Write;

        // Configuration
        let grid_radius = 16;  // This means we'll go from -8 to +8, making a 17x17 grid
        let cell_size = 30;   // Smaller cells to fit the larger grid
        let margin = 50;
        
        let grid_size = grid_radius * 2 + 1;
        let width = cell_size * grid_size + 2 * margin;
        let height = width;

        // Create grid points
        let mut points: BTreeMap<usize, (i32, i32)> = BTreeMap::new();
        for y in -grid_radius..=grid_radius {
            for x in -grid_radius..=grid_radius {
                let z = ZOrderGlobalMap::xy_to_z(x, y);
                points.insert(z, (x, y));
            }
        }
        
        // Start SVG content
        let mut svg = String::new();
        svg.push_str(&format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
<style>
    .background {{ fill: #1e1e1e; }}
    .grid-line {{ stroke: #333; stroke-width: 1; }}
    .axis-line {{ stroke: #000; stroke-width: 2; }}
    .path {{ stroke: #4CAF50; stroke-width: 2; fill: none; opacity: 0.8; }}
    .point {{ fill: #03A9F4; r: 3; }}
    .point-highlight {{ fill: #03A9F4; r: 4; }}
    .text {{ font-family: Arial; font-size: 10px; fill: #888; }}
    .number {{ font-family: Arial; font-size: 8px; fill: #03A9F4; }}
</style>
<rect x="0" y="0" width="{}" height="{}" class="background"/>
"#, width, height, width, height));

        // Draw grid
        for i in 0..=grid_size {
            let pos = margin + i * cell_size;
            // Vertical lines
            svg.push_str(&format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="grid-line"/>"#,
                pos, margin, pos, height - margin));
            // Horizontal lines
            svg.push_str(&format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="grid-line"/>"#,
                margin, pos + margin, width - margin, pos + margin));
        }

        // Draw axes
        let center_x = margin + cell_size * grid_radius;
        let center_y = margin + cell_size * grid_radius;
        svg.push_str(&format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="axis-line"/>"#,
            center_x, margin, center_x, height - margin));
        svg.push_str(&format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="axis-line"/>"#,
            margin, center_y, width - margin, center_y));

        // Draw coordinate numbers (only show every other number if grid is large)
        let step = if grid_size > 15 { 2 } else { 1 };
        for i in (0..grid_size).step_by(step) {
            let num = i as i32 - grid_radius;
            let pos = margin + i * cell_size;
            // X-axis numbers
            svg.push_str(&format!(r#"<text x="{}" y="{}" class="text" text-anchor="middle">{}</text>"#,
                pos + cell_size, height - margin + 15, num));
            // Y-axis numbers
            svg.push_str(&format!(r#"<text x="{}" y="{}" class="text" text-anchor="end">{}</text>"#,
                margin - 5, pos + cell_size + 4, -num));
        }

        // Draw path first (so it's behind points)
        let mut path = String::new();
        let mut points_vec: Vec<_> = points.iter().collect();
        points_vec.sort_by_key(|(z, _)| *z);
        
        // Draw line segments only between consecutive z values
        for window in points_vec.windows(2) {
            let (z1, &(x1, y1)) = window[0];
            let (z2, &(x2, y2)) = window[1];
            
            // Only connect if z values are consecutive
            if z2 - z1 == 1 {
                let px1 = (margin as i32 + (x1 + grid_radius) * cell_size as i32 + cell_size as i32/2) as usize;
                let py1 = (margin as i32 + (y1 + grid_radius) * cell_size as i32 + cell_size as i32/2) as usize;
                let px2 = (margin as i32 + (x2 + grid_radius) * cell_size as i32 + cell_size as i32/2) as usize;
                let py2 = (margin as i32 + (y2 + grid_radius) * cell_size as i32 + cell_size as i32/2) as usize;
                
                path.push_str(&format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="path"/>"#,
                    px1, py1, px2, py2));
            }
        }
        
        // Add path to SVG
        svg.push_str(&path);

        // Draw points and numbers on top
        for (idx, (_, &(x, y))) in points_vec.iter().enumerate() {
            let px = (margin as i32 + (x + grid_radius) * cell_size as i32 + cell_size as i32/2) as usize;
            let py = (margin as i32 + (y + grid_radius) * cell_size as i32 + cell_size as i32/2) as usize;
            
            // Draw point (highlight first few points)
            let point_class = if idx < 4 { "point-highlight" } else { "point" };
            svg.push_str(&format!(r#"<circle cx="{}" cy="{}" class="{}"/>"#, px, py, point_class));
            
            // Draw sequence number (only if grid is not too dense)
            if grid_size <= 15 {
                svg.push_str(&format!(r#"<text x="{}" y="{}" class="number" text-anchor="middle">{}</text>"#,
                    px, py - 4, idx));
            }
        }

        // Close SVG
        svg.push_str("</svg>");

        // Write to file
        let mut file = File::create("z_order_path.svg").unwrap();
        file.write_all(svg.as_bytes()).unwrap();
        
        println!("SVG visualization saved to z_order_path.svg");
        println!("Grid size: {}x{} (from -{} to +{})", grid_size, grid_size, grid_radius, grid_radius);
    }
}
