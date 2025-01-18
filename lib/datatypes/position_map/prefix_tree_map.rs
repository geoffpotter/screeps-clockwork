use std::collections::HashMap;
use screeps::{Position, RoomName};
use super::{GlobalPoint, MapTrait, PositionOptions};
use crate::datatypes::position::y_major_packed_position::YMajorPackedPosition;

const ROOM_SIZE: i32 = 50;
const BITS_PER_COORD: u32 = 16;  // Using 16 bits per coordinate for global positions

pub struct PrefixTreeMap {
    root: TrieNode,
}

struct TrieNode {
    // Value stored at this node (if it's a leaf)
    value: usize,
    // Child nodes for 0 and 1 bits
    children: Option<Box<[Option<TrieNode>; 2]>>,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            value: usize::MAX,
            children: None,
        }
    }

    fn insert(&mut self, path: u64, depth: u32, value: usize) {
        if depth == BITS_PER_COORD * 2 {
            self.value = value;
            return;
        }

        if self.children.is_none() {
            self.children = Some(Box::new([None, None]));
        }

        let bit = ((path >> (BITS_PER_COORD * 2 - 1 - depth)) & 1) as usize;
        
        if let Some(children) = &mut self.children {
            if children[bit].is_none() {
                children[bit] = Some(TrieNode::new());
            }
            
            if let Some(child) = &mut children[bit] {
                child.insert(path, depth + 1, value);
            }
        }
    }

    fn get(&self, path: u64, depth: u32) -> usize {
        if depth == BITS_PER_COORD * 2 {
            return self.value;
        }

        if let Some(children) = &self.children {
            let bit = ((path >> (BITS_PER_COORD * 2 - 1 - depth)) & 1) as usize;
            if let Some(child) = &children[bit] {
                return child.get(path, depth + 1);
            }
        }

        usize::MAX
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

impl PrefixTreeMap {
    fn encode_position(point: GlobalPoint) -> u64 {
        // Interleave bits of x and y coordinates to preserve locality
        let mut x = point.x as u32;
        let mut y = point.y as u32;
        
        // Ensure we only use BITS_PER_COORD bits
        x &= ((1u32 << BITS_PER_COORD) - 1);
        y &= ((1u32 << BITS_PER_COORD) - 1);
        
        let mut result = 0u64;
        for i in 0..BITS_PER_COORD {
            result |= (((x >> i) & 1) as u64) << (2 * i);
            result |= (((y >> i) & 1) as u64) << (2 * i + 1);
        }
        result
    }
}

impl MapTrait for PrefixTreeMap {
    fn new() -> Self {
        Self {
            root: TrieNode::new(),
        }
    }

    fn set(&mut self, options: PositionOptions, value: usize) {
        let encoded = Self::encode_position(options.global_point);
        self.root.insert(encoded, 0, value);
    }

    fn get(&mut self, options: PositionOptions) -> usize {
        let encoded = Self::encode_position(options.global_point);
        self.root.get(encoded, 0)
    }

    fn memory_usage(&self) -> usize {
        // Each node contains:
        // - usize for value (8 bytes)
        // - Option<Box<[Option<TrieNode>; 2]>> for children (24 bytes)
        let bytes_per_node = 32;
        self.root.count_nodes() * bytes_per_node
    }
} 