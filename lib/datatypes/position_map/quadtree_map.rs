use std::collections::HashMap;
use super::{GlobalPoint, MapTrait, PositionOptions};
use screeps::Position;
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const MAX_DEPTH: u32 = 8;  // Allows for 2^16 x 2^16 grid which is plenty for global coordinates

pub struct QuadtreeMap {
    root: Option<QuadNode>,
}

struct QuadNode {
    children: Option<Box<[Option<QuadNode>; 4]>>,
    // Store both the value and the point where it was set
    value: Option<(GlobalPoint, usize)>,
    depth: u32,
    bounds: Bounds,
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

impl QuadtreeMap {
    fn get_initial_bounds() -> Bounds {
        // Start with a large enough area to cover the game world
        Bounds {
            min_x: -32768,  // -2^15
            min_y: -32768,
            max_x: 32767,   // 2^15 - 1
            max_y: 32767,
        }
    }
}

impl MapTrait for QuadtreeMap {
    fn new() -> Self {
        Self {
            root: Some(QuadNode {
                children: None,
                value: None,
                depth: 0,
                bounds: Self::get_initial_bounds(),
            }),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        if let Some(root) = &mut self.root {
            root.set(options.global_point, value);
        }
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        self.root.as_ref()
            .and_then(|root| root.get(options.global_point))
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        // Rough estimate: 
        // Each node: 4 child pointers (32 bytes) + value (12 bytes) + depth (4 bytes) + bounds (16 bytes)
        let mut count = 0;
        if let Some(root) = &self.root {
            count = root.count_nodes();
        }
        count * (32 + 12 + 4 + 16)
    }
}

impl QuadNode {
    fn get(&self, point: GlobalPoint) -> Option<usize> {
        if !self.contains(point) {
            return None;
        }

        // If this node has a value, only return it if it's for this exact point
        if let Some((stored_point, value)) = self.value {
            if stored_point == point {
                return Some(value);
            }
        }

        // If we have children, delegate to the appropriate child
        if let Some(children) = &self.children {
            for child in children.iter().flatten() {
                if child.contains(point) {
                    return child.get(point);
                }
            }
        }

        // No child contains the point
        None
    }

    fn set(&mut self, point: GlobalPoint, value: usize) {
        if !self.contains(point) {
            return;
        }

        if self.depth == MAX_DEPTH {
            self.value = Some((point, value));
            return;
        }

        // If we don't have children and no value, just set the value
        if self.children.is_none() && self.value.is_none() {
            self.value = Some((point, value));
            return;
        }

        // If we have a value or children, we need to split
        if self.children.is_none() {
            self.split();
        }

        // Delegate to appropriate child
        if let Some(children) = &mut self.children {
            for child in children.iter_mut().flatten() {
                if child.contains(point) {
                    child.set(point, value);
                    return;
                }
            }
        }
    }

    fn split(&mut self) {
        let mid_x = (self.bounds.min_x + self.bounds.max_x) / 2;
        let mid_y = (self.bounds.min_y + self.bounds.max_y) / 2;
        
        let new_depth = self.depth + 1;
        
        let children = Box::new([
            // Northwest
            Some(QuadNode {
                children: None,
                value: None,
                depth: new_depth,
                bounds: Bounds {
                    min_x: self.bounds.min_x,
                    min_y: self.bounds.min_y,
                    max_x: mid_x,
                    max_y: mid_y,
                },
            }),
            // Northeast
            Some(QuadNode {
                children: None,
                value: None,
                depth: new_depth,
                bounds: Bounds {
                    min_x: mid_x + 1,
                    min_y: self.bounds.min_y,
                    max_x: self.bounds.max_x,
                    max_y: mid_y,
                },
            }),
            // Southwest
            Some(QuadNode {
                children: None,
                value: None,
                depth: new_depth,
                bounds: Bounds {
                    min_x: self.bounds.min_x,
                    min_y: mid_y + 1,
                    max_x: mid_x,
                    max_y: self.bounds.max_y,
                },
            }),
            // Southeast
            Some(QuadNode {
                children: None,
                value: None,
                depth: new_depth,
                bounds: Bounds {
                    min_x: mid_x + 1,
                    min_y: mid_y + 1,
                    max_x: self.bounds.max_x,
                    max_y: self.bounds.max_y,
                },
            }),
        ]);

        // If we had a value, we need to insert it into the appropriate child
        let old_value = self.value.take();
        self.children = Some(children);

        // If we had a value, find which child should contain it and set it there
        if let Some((point, value)) = old_value {
            self.set(point, value);
        }
    }

    fn contains(&self, point: GlobalPoint) -> bool {
        point.x >= self.bounds.min_x && point.x <= self.bounds.max_x &&
        point.y >= self.bounds.min_y && point.y <= self.bounds.max_y
    }

    fn count_nodes(&self) -> usize {
        let mut count = 1;
        if let Some(children) = &self.children {
            for child in children.iter().flatten() {
                count += child.count_nodes();
            }
        }
        count
    }
} 