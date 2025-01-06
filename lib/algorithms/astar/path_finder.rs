use screeps::{Position, RoomName, Direction, RoomXY, RoomCoordinate, xy_to_linear_index};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet, HashMap};
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use js_sys;

use crate::log;

const MAX_ROOMS: usize = 64;
const OBSTACLE_COST: u32 = u32::MAX;

type Cost = u32;
type PosIndex = u32;
type RoomIndex = u32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct MapPosition {
    xx: u8,
    yy: u8,
}

impl MapPosition {
    fn new(xx: u8, yy: u8) -> Self {
        Self { xx, yy }
    }

    fn from_room_name(room_name: RoomName) -> Self {
        let x = room_name.x_coord();
        let y = room_name.y_coord();
        let xx = (x + 128) as u8;
        let yy = (y + 128) as u8;
        // log(&format!("Converting room {:?} ({}, {}) to map position ({}, {})", room_name, x, y, xx, yy));
        Self { xx, yy }
    }

    fn id(&self) -> u16 {
        ((self.xx as u16) << 8) | (self.yy as u16)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct WorldPosition {
    xx: u32,
    yy: u32,
}

impl WorldPosition {
    fn new(xx: u32, yy: u32) -> Self {
        Self { xx, yy }
    }

    fn null() -> Self {
        Self { xx: 0, yy: 0 }
    }

    fn is_null(&self) -> bool {
        self.xx == 0 && self.yy == 0
    }

    fn from_position(pos: Position) -> Self {
        let x = pos.room_name().x_coord();
        let y = pos.room_name().y_coord();
        let base_x = (x + 128) * 50;
        let base_y = (y + 128) * 50;
        let result = Self {
            xx: (base_x + pos.x().u8() as i32) as u32,
            yy: (base_y + pos.y().u8() as i32) as u32,
        };
        // log(&format!("Converting room pos ({}, {}) in room {} to world pos ({}, {})", 
        //     pos.x().u8(), pos.y().u8(), pos.room_name(), result.xx, result.yy));
        result
    }

    fn to_position(&self) -> Position {
        let room_x = (self.xx / 50) as i32;
        let room_y = (self.yy / 50) as i32;
        
        // Convert from our internal coordinate system (0-255) to Screeps coordinates
        let x = -128 + room_x;  // Convert from 0-255 to -128-127
        let y = -128 + room_y;  // Convert from 0-255 to -128-127

        // Pack room coordinates into a u16
        let packed_room = (((x + 128) as u16) << 8) | ((y + 128) as u16);
        let room_name = RoomName::from_packed(packed_room);
        let x_coord = RoomCoordinate::new((self.xx % 50) as u8).unwrap();
        let y_coord = RoomCoordinate::new((self.yy % 50) as u8).unwrap();
        let pos = Position::new(x_coord, y_coord, room_name);
        // log(&format!("Converting world pos ({}, {}) to room pos ({}, {}) in room {}", 
        //     self.xx, self.yy, x_coord.u8(), y_coord.u8(), room_name));
        pos
    }

    fn map_position(&self) -> MapPosition {
        MapPosition::new((self.xx / 50) as u8, (self.yy / 50) as u8)
    }

    fn range_to(&self, other: WorldPosition) -> u32 {
        let dx = if other.xx > self.xx { other.xx - self.xx } else { self.xx - other.xx };
        let dy = if other.yy > self.yy { other.yy - self.yy } else { self.yy - other.yy };
        std::cmp::max(dx, dy)
    }

    fn checked_add_direction(&self, dir: Direction) -> Option<WorldPosition> {
        match dir {
            Direction::Top => if self.yy > 0 { Some(WorldPosition::new(self.xx, self.yy - 1)) } else { None },
            Direction::TopRight => if self.yy > 0 { Some(WorldPosition::new(self.xx + 1, self.yy - 1)) } else { None },
            Direction::Right => Some(WorldPosition::new(self.xx + 1, self.yy)),
            Direction::BottomRight => Some(WorldPosition::new(self.xx + 1, self.yy + 1)),
            Direction::Bottom => Some(WorldPosition::new(self.xx, self.yy + 1)),
            Direction::BottomLeft => if self.xx > 0 { Some(WorldPosition::new(self.xx - 1, self.yy + 1)) } else { None },
            Direction::Left => if self.xx > 0 { Some(WorldPosition::new(self.xx - 1, self.yy)) } else { None },
            Direction::TopLeft => if self.xx > 0 && self.yy > 0 { Some(WorldPosition::new(self.xx - 1, self.yy - 1)) } else { None },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PathFinderState {
    f_score: Cost,
    g_score: Cost,
    position: WorldPosition,
    parent: Option<PosIndex>,
}

impl Ord for PathFinderState {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score)
            // .then_with(|| other.g_score.cmp(&self.g_score))
            .then_with(|| self.g_score.cmp(&other.g_score))
    }
}

impl PartialOrd for PathFinderState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct RoomInfo {
    terrain: Vec<u8>,
    cost_matrix: Option<Vec<u8>>,
    pos: MapPosition,
}

impl RoomInfo {
    fn new(terrain: Vec<u8>, cost_matrix: Option<Vec<u8>>, pos: MapPosition) -> Self {
        Self {
            terrain,
            cost_matrix,
            pos,
        }
    }

    fn get_cost(&self, x: u8, y: u8) -> u8 {
        if let Some(ref cost_matrix) = self.cost_matrix {
            let cost = cost_matrix[y as usize * 50 + x as usize];
            if cost > 0 {
                return cost;
            }
        }
        // Extract terrain cost from packed terrain data
        let index = (y as usize * 50 + x as usize) / 4;
        let shift = ((y as usize * 50 + x as usize) % 4) * 2;
        (self.terrain[index] >> shift) & 0x03
    }
}

pub struct PathFinder {
    room_table: Vec<RoomInfo>,
    reverse_room_table: Vec<RoomIndex>,
    blocked_rooms: HashSet<MapPosition>,
    open_set: BinaryHeap<PathFinderState>,
    closed_set: HashSet<PosIndex>,
    plain_cost: Cost,
    swamp_cost: Cost,
    max_rooms: u8,
    max_ops: u32,
    max_cost: u32,
    flee: bool,
    heuristic_weight: f64,
    debug: bool,
}

impl PathFinder {
    pub fn new(
        plain_cost: Cost,
        swamp_cost: Cost,
        max_rooms: u8,
        max_ops: u32,
        max_cost: u32,
        flee: bool,
        heuristic_weight: f64,
    ) -> Self {
        Self {
            room_table: Vec::with_capacity(MAX_ROOMS),
            reverse_room_table: vec![0; 65536], // 2^16 possible room positions
            blocked_rooms: HashSet::new(),
            open_set: BinaryHeap::new(),
            closed_set: HashSet::new(),
            plain_cost,
            swamp_cost,
            max_rooms,
            max_ops,
            max_cost,
            flee,
            heuristic_weight,
            debug: false,
        }
    }

    fn debug_log(&self, msg: &str) {
        if self.debug {
            log(msg);
        }
    }

    fn get_cost(&self, pos: WorldPosition) -> Cost {
        let map_pos = pos.map_position();
        let room_index = self.reverse_room_table[map_pos.id() as usize];
        if room_index == 0 {
            self.debug_log(&format!("No room data for position {:?}", pos));
            return OBSTACLE_COST;
        }
        let room_info = &self.room_table[(room_index - 1) as usize];
        let x = (pos.xx % 50) as u8;
        let y = (pos.yy % 50) as u8;
        let terrain_cost = room_info.get_cost(x, y);
        let cost = match terrain_cost {
            0 => self.plain_cost,  // TERRAIN_MASK_PLAIN = 0
            2 => self.swamp_cost,  // TERRAIN_MASK_SWAMP = 2
            1 => OBSTACLE_COST,    // TERRAIN_MASK_WALL = 1
            _ => OBSTACLE_COST,
        };
        self.debug_log(&format!("Cost for pos {:?}: terrain={}, cost={}", pos, terrain_cost, cost));
        cost
    }

    fn heuristic(&self, pos: WorldPosition, goals: &[Position]) -> Cost {
        let mut min_cost = Cost::MAX;
        for goal in goals {
            let goal_pos = WorldPosition::from_position(*goal);
            let cost = if self.flee {
                pos.range_to(goal_pos)  // For flee mode, larger distance is better
            } else {
                pos.range_to(goal_pos)
            };
            if cost < min_cost {
                min_cost = cost;
            }
            self.debug_log(&format!("Distance to goal {:?}: {}", goal_pos, cost));
        }
        let weighted_cost = (min_cost as f64 * self.heuristic_weight) as Cost;
        self.debug_log(&format!("Heuristic for pos {:?}: {} (weighted: {})", pos, min_cost, weighted_cost));
        weighted_cost
    }

    fn index_from_pos(&self, pos: WorldPosition) -> Option<PosIndex> {
        let map_pos = pos.map_position();
        let room_index = self.reverse_room_table[map_pos.id() as usize];
        if room_index == 0 {
            self.debug_log(&format!("Room not found in table: {:?}", map_pos));
            if self.room_table.len() >= self.max_rooms as usize {
                self.debug_log("Too many rooms");
                return None;
            }
            if self.blocked_rooms.contains(&map_pos) {
                self.debug_log("Room is blocked");
                return None;
            }
            self.debug_log("Room not loaded");
            return None;
        }

        let x = (pos.xx % 50) as PosIndex;
        let y = (pos.yy % 50) as PosIndex;
        let pos_index = ((room_index - 1) as PosIndex * 2500) + x * 50 + y;
        self.debug_log(&format!("Returning index {} for pos {:?}", pos_index, pos));
        Some(pos_index)
    }

    fn pos_from_index(&self, index: PosIndex) -> WorldPosition {
        let room_index = (index / 2500) as usize;
        let room_info = &self.room_table[room_index];
        let x = ((index % 2500) / 50) as u32;
        let y = (index % 50) as u32;
        let world_pos = WorldPosition::new(
            (room_info.pos.xx as u32 * 50) + x,
            (room_info.pos.yy as u32 * 50) + y,
        );
        self.debug_log(&format!("Converting index {} to pos {:?}", index, world_pos));
        world_pos
    }

    fn push_node(&mut self, parent_index: PosIndex, node: WorldPosition, g_cost: Cost, goals: &[Position]) {
        let node_index = match self.index_from_pos(node) {
            Some(index) => index,
            None => return,
        };

        if self.closed_set.contains(&node_index) {
            return;
        }

        let h_cost = self.heuristic(node, goals);
        let f_cost = g_cost + h_cost;

        let state = PathFinderState {
            f_score: f_cost,
            g_score: g_cost,
            position: node,
            parent: Some(parent_index),
        };

        // If node is already in open set, update it if this path is better
        if let Some(existing_state) = self.open_set.iter().find(|s| s.position == node) {
            if existing_state.f_score > f_cost {
                self.open_set.retain(|s| s.position != node);
                self.open_set.push(state);
            }
        } else {
            self.open_set.push(state);
        }
    }

    pub fn load_room_data(&mut self, room_name: RoomName, terrain: Vec<u8>, cost_matrix: Option<Vec<u8>>) {
        let pos = MapPosition::from_room_name(room_name);
        self.debug_log(&format!("Loading room data for {:?} at map position {:?}", room_name, pos));
        let room_info = RoomInfo::new(terrain, cost_matrix, pos);
        self.room_table.push(room_info);
        self.reverse_room_table[pos.id() as usize] = self.room_table.len() as RoomIndex;
        self.debug_log(&format!("Room table index: {}, reverse table index: {}", self.room_table.len() - 1, pos.id()));
    }

    pub fn search(&mut self, origin: Position, goals: &[Position]) -> Option<Vec<Position>> {
        self.debug_log(&format!("Starting search from {:?} to {:?}", origin, goals));
        let origin_pos = WorldPosition::from_position(origin);
        let origin_index = self.index_from_pos(origin_pos)?;

        // Initialize search
        self.open_set.clear();
        self.closed_set.clear();
        self.push_node(origin_index, origin_pos, 0, goals);

        let mut ops_remaining = self.max_ops;
        let mut min_node = None;
        let mut min_node_h_cost = Cost::MAX;
        let mut min_node_g_cost = Cost::MAX;

        // Store parent pointers for path reconstruction
        let mut parent_map = std::collections::HashMap::new();

        // Main search loop
        while let Some(current) = self.open_set.pop() {
            if ops_remaining == 0 {
                self.debug_log("Search terminated: out of operations");
                break;
            }

            let current_pos = current.position;
            let current_index = self.index_from_pos(current_pos)?;
            self.closed_set.insert(current_index);

            if let Some(parent) = current.parent {
                parent_map.insert(current_index, parent);
            }

            let h_cost = self.heuristic(current_pos, goals);
            let g_cost = current.g_score;

            self.debug_log(&format!(
                "Exploring pos: xx={}, yy={}, h_cost={}, g_cost={}, f_cost={}",
                current_pos.xx, current_pos.yy, h_cost, g_cost, h_cost + g_cost
            ));

            // Check if we've reached a goal
            if h_cost == 0 || (self.flee && h_cost >= 1) {
                self.debug_log("Found goal!");
                min_node = Some(current);
                break;
            } else if h_cost < min_node_h_cost || (self.flee && h_cost > min_node_h_cost) {
                min_node = Some(current);
                min_node_h_cost = h_cost;
                min_node_g_cost = g_cost;
            }

            if g_cost + h_cost > self.max_cost {
                self.debug_log("Search terminated: exceeded max cost");
                break;
            }

            // Add neighbors
            let pos = current.position;
            let index = current_index;

            // Add natural neighbors
            for dir in &[Direction::Top, Direction::Right, Direction::Bottom, Direction::Left] {
                if let Some(next) = pos.checked_add_direction(*dir) {
                    let n_cost = self.get_cost(next);
                    if n_cost != OBSTACLE_COST {
                        self.push_node(index, next, g_cost + n_cost, goals);
                    }
                }
            }

            // Add diagonal neighbors
            for dir in &[Direction::TopRight, Direction::BottomRight, Direction::BottomLeft, Direction::TopLeft] {
                if let Some(next) = pos.checked_add_direction(*dir) {
                    let n_cost = self.get_cost(next);
                    if n_cost != OBSTACLE_COST {
                        self.push_node(index, next, g_cost + n_cost, goals);
                    }
                }
            }

            ops_remaining -= 1;
        }

        self.debug_log(&format!("Search completed with {} operations remaining", ops_remaining));
        self.debug_log(&format!("Parent map size: {}", parent_map.len()));

        // Reconstruct path
        if let Some(end_node) = min_node {
            let mut path = Vec::new();
            let mut current = end_node;
            path.push(current.position.to_position());

            self.debug_log(&format!(
                "Starting path reconstruction from xx={}, yy={}",
                current.position.xx, current.position.yy
            ));

            // Keep track of visited positions to avoid cycles
            let mut visited = HashSet::new();
            visited.insert(current.position);

            while let Some(parent_index) = current.parent {
                let parent_pos = self.pos_from_index(parent_index);
                
                self.debug_log(&format!(
                    "Found parent at xx={}, yy={}",
                    parent_pos.xx, parent_pos.yy
                ));

                // Interpolate path if jump point is more than 1 tile away
                if current.position.range_to(parent_pos) > 1 {
                    let mut pos = current.position;
                    while pos.range_to(parent_pos) > 1 {
                        let dx = (parent_pos.xx as i32 - pos.xx as i32).signum();
                        let dy = (parent_pos.yy as i32 - pos.yy as i32).signum();
                        pos = WorldPosition::new(
                            (pos.xx as i32 + dx) as u32,
                            (pos.yy as i32 + dy) as u32,
                        );
                        if !visited.insert(pos) {
                            self.debug_log("Cycle detected during interpolation");
                            return Some(path);
                        }
                        path.push(pos.to_position());
                    }
                }

                if !visited.insert(parent_pos) {
                    self.debug_log("Cycle detected at parent");
                    return Some(path);
                }
                path.push(parent_pos.to_position());

                // Look up the next parent from our parent map
                if let Some(&next_parent) = parent_map.get(&parent_index) {
                    current = PathFinderState {
                        position: parent_pos,
                        parent: Some(next_parent),
                        f_score: 0,   // Not needed for reconstruction
                        g_score: 0,   // Not needed for reconstruction
                    };
                } else {
                    self.debug_log("No more parents found in parent map");
                    break;
                }
            }

            path.reverse();
            self.debug_log(&format!("Final path length: {}", path.len()));
            Some(path)
        } else {
            self.debug_log("No path found");
            None
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }
}

fn is_border_pos(val: u32) -> bool {
    (val + 1) % 50 < 2
}

fn is_near_border_pos(val: u32) -> bool {
    (val + 2) % 50 < 4
}

#[wasm_bindgen]
pub struct JsPathFinder(PathFinder);

#[derive(Serialize, Deserialize, Clone)]
struct JsPosition {
    x: u32,
    y: u32,
    roomName: String,
}

impl From<Position> for JsPosition {
    fn from(pos: Position) -> Self {
        Self {
            x: pos.x().u8() as u32,
            y: pos.y().u8() as u32,
            roomName: pos.room_name().to_string(),
        }
    }
}

impl TryFrom<JsPosition> for Position {
    type Error = &'static str;

    fn try_from(pos: JsPosition) -> Result<Self, Self::Error> {
        let room_name = RoomName::new(&pos.roomName).map_err(|_| "Invalid room name")?;
        let x_coord = RoomCoordinate::new(pos.x as u8).map_err(|_| "Invalid x coordinate")?;
        let y_coord = RoomCoordinate::new(pos.y as u8).map_err(|_| "Invalid y coordinate")?;
        let xy = RoomXY::new(x_coord, y_coord);
        Ok(Position::new(x_coord, y_coord, room_name))
    }
}

#[wasm_bindgen]
impl JsPathFinder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        plain_cost: u32,
        swamp_cost: u32,
        max_rooms: u8,
        max_ops: u32,
        max_cost: u32,
        flee: bool,
        heuristic_weight: f64,
    ) -> Self {
        Self(PathFinder::new(
            plain_cost,
            swamp_cost,
            max_rooms,
            max_ops,
            max_cost,
            flee,
            heuristic_weight,
        ))
    }

    #[wasm_bindgen]
    pub fn set_debug(&mut self, debug: bool) {
        self.0.set_debug(debug);
    }

    #[wasm_bindgen]
    pub fn search(
        &mut self,
        origin: &JsValue,
        goals: &JsValue,
        room_callback: &js_sys::Function,
    ) -> Option<js_sys::Array> {
        self.0.debug_log(&format!("Searching for path: {:?}", origin));
        // Convert JsValue to Position
        let origin: JsPosition = serde_wasm_bindgen::from_value(origin.clone()).ok()?;
        let origin = Position::try_from(origin).ok()?;
        
        let goals: Vec<JsPosition> = serde_wasm_bindgen::from_value(goals.clone()).ok()?;
        let goals: Vec<Position> = goals.into_iter()
            .filter_map(|p| Position::try_from(p).ok())
            .collect();

        // Clear previous room data
        self.0.room_table.clear();
        for i in 0..self.0.reverse_room_table.len() {
            self.0.reverse_room_table[i] = 0;
        }
        self.0.blocked_rooms.clear();
        self.0.debug_log("Cleared room data and blocked rooms");

        // Get unique rooms that need to be loaded
        let mut rooms = HashSet::new();
        rooms.insert(origin.room_name());
        for goal in goals.iter() {
            rooms.insert(goal.room_name());
        }

        self.0.debug_log(&format!("Loading room data for rooms: {:?}", rooms));
        // Load room data for each room
        for room_name in rooms {
            let this = JsValue::null();
            let room_arg = JsValue::from_str(&room_name.to_string());
            
            let result = match room_callback.call1(&this, &room_arg) {
                Ok(val) => val,
                Err(_) => return None,
            };

            if result.is_undefined() || result.is_null() {
                // Room is inaccessible
                self.0.blocked_rooms.insert(MapPosition::from_room_name(room_name));
                continue;
            }

            // Extract terrain and cost matrix data
            let obj = js_sys::Object::from(result);
            
            // Get terrain data
            let terrain = match js_sys::Reflect::get(&obj, &JsValue::from_str("terrain")) {
                Ok(val) => js_sys::Uint8Array::new(&val),
                Err(_) => return None,
            };
            let mut terrain_data = vec![0; terrain.length() as usize];
            terrain.copy_to(&mut terrain_data);
            let terrain_data = convert_terrain_data(&terrain_data);

            // Get optional cost matrix
            let cost_matrix = if let Ok(matrix) = js_sys::Reflect::get(&obj, &JsValue::from_str("cost_matrix")) {
                if !matrix.is_undefined() && !matrix.is_null() {
                    let matrix = js_sys::Uint8Array::new(&matrix);
                    let mut matrix_data = vec![0; matrix.length() as usize];
                    matrix.copy_to(&mut matrix_data);
                    Some(matrix_data)
                } else {
                    None
                }
            } else {
                None
            };

            self.0.load_room_data(room_name, terrain_data, cost_matrix);
        }

        self.0.debug_log("Loaded room data for rooms, doing search");

        // Perform pathfinding
        let path = self.0.search(origin, &goals);
        self.0.debug_log(&format!("Path found: {:?}", path));
        if let Some(path) = path {
            // Convert path to JS array
            let result = js_sys::Array::new();
            for pos in path {
                let js_pos = JsPosition::from(pos);
                let js_val = serde_wasm_bindgen::to_value(&js_pos).ok()?;
                result.push(&js_val);
            }
            return Some(result);
        }
        None
    }
}

// Helper function to convert terrain data from JavaScript
fn convert_terrain_data(terrain_data: &[u8]) -> Vec<u8> {
    let mut result = vec![0; (terrain_data.len() + 3) / 4];
    for (i, chunk) in terrain_data.chunks(4).enumerate() {
        let mut byte = 0u8;
        for (j, &val) in chunk.iter().enumerate() {
            // Screeps terrain values:
            // 0 = plain
            // 1 = wall
            // 2 = swamp
            byte |= (val & 0x03) << (j * 2);
        }
        result[i] = byte;
    }
    result
}

#[wasm_bindgen]
pub struct PathFinderResult {
    path: Vec<JsPosition>,
    ops: u32,
    cost: u32,
    incomplete: bool,
}

#[wasm_bindgen]
impl PathFinderResult {
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> js_sys::Array {
        let result = js_sys::Array::new();
        for pos in &self.path {
            if let Ok(js_val) = serde_wasm_bindgen::to_value(pos) {
                result.push(&js_val);
            }
        }
        result
    }
}
