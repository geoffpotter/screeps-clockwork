use std::collections::HashMap;
use super::{GlobalPoint, MapTrait};
use screeps::{Position, RoomName};

const ROOM_SIZE: usize = 50;
const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;

pub struct HashGridMap {
    // Map from room name to room data
    rooms: HashMap<RoomName, Box<[usize; ROOM_AREA]>>,
    // LRU cache for frequently accessed positions
    cache: LruCache,
}

struct LruCache {
    // Fixed size cache of most recently accessed positions
    cache: HashMap<(RoomName, u8, u8), usize>,
    keys: Vec<(RoomName, u8, u8)>,
    capacity: usize,
}

impl LruCache {
    fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
            keys: Vec::with_capacity(capacity),
            capacity,
        }
    }

    fn get(&mut self, key: &(RoomName, u8, u8)) -> usize {
        if let Some(&value) = self.cache.get(key) {
            // Move key to end (most recently used)
            if let Some(pos) = self.keys.iter().position(|k| k == key) {
                self.keys.remove(pos);
                self.keys.push(*key);
            }
            value
        } else {
            usize::MAX
        }
    }

    fn insert(&mut self, key: (RoomName, u8, u8), value: usize) {
        if self.cache.len() >= self.capacity {
            // Remove least recently used
            if let Some(old_key) = self.keys.first().copied() {
                self.cache.remove(&old_key);
                self.keys.remove(0);
            }
        }
        self.cache.insert(key, value);
        self.keys.push(key);
    }
}

impl MapTrait for HashGridMap {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            cache: LruCache::new(1000), // Cache last 1000 positions
        }
    }

    fn set(&mut self, _wpos: GlobalPoint, pos: Position, value: usize) {
        let room_name = pos.room_name();
        let local_x = pos.x().u8();
        let local_y = pos.y().u8();
        let index = (local_y as usize) * ROOM_SIZE + (local_x as usize);
        
        // Try cache first
        self.cache.insert((room_name, local_x, local_y), value);
        
        // Update main storage
        let room = self.rooms.entry(room_name)
            .or_insert_with(|| Box::new([usize::MAX; ROOM_AREA]));
        room[index] = value;
    }

    fn get(&mut self, _wpos: GlobalPoint, pos: Position) -> usize {
        let room_name = pos.room_name();
        let local_x = pos.x().u8();
        let local_y = pos.y().u8();
        let index = (local_y as usize) * ROOM_SIZE + (local_x as usize);
        
        // Try cache first
        let cached = self.cache.get(&(room_name, local_x, local_y));
        if cached != usize::MAX {
            return cached;
        }
        
        // Fall back to main storage
        self.rooms.get(&room_name)
            .map(|room| room[index])
            .unwrap_or(usize::MAX)
    }

    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // Size of room HashMap
        total += self.rooms.len() * std::mem::size_of::<(RoomName, Box<[usize; ROOM_AREA]>)>();
        
        // Size of arrays in each room
        total += self.rooms.len() * std::mem::size_of::<[usize; ROOM_AREA]>();
        
        // Size of LRU cache
        total += self.cache.cache.capacity() * std::mem::size_of::<((RoomName, u8, u8), usize)>();
        total += self.cache.keys.capacity() * std::mem::size_of::<(RoomName, u8, u8)>();
        
        total
    }
} 