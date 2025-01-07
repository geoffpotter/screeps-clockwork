use screeps::{Position, RoomName, Direction, RoomXY, RoomCoordinate, xy_to_linear_index, game::cpu};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet, HashMap};
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use js_sys;

use crate::log;
use crate::utils::PROFILER;

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
    #[inline]
    fn new(xx: u32, yy: u32) -> Self {
        Self { xx, yy }
    }

    #[inline]
    fn null() -> Self {
        Self { xx: 0, yy: 0 }
    }

    #[inline]
    fn is_null(&self) -> bool {
        self.xx == 0 && self.yy == 0
    }

    #[inline]
    fn from_position(pos: Position) -> Self {
        let x = pos.room_name().x_coord();
        let y = pos.room_name().y_coord();
        let base_x = (x + 128) * 50;
        let base_y = (y + 128) * 50;
        Self {
            xx: (base_x + pos.x().u8() as i32) as u32,
            yy: (base_y + pos.y().u8() as i32) as u32,
        }
    }

    #[inline]
    fn to_position(&self) -> Position {
        let room_x = (self.xx / 50) as i32;
        let room_y = (self.yy / 50) as i32;
        let x = -128 + room_x;
        let y = -128 + room_y;
        let packed_room = (((x + 128) as u16) << 8) | ((y + 128) as u16);
        let room_name = RoomName::from_packed(packed_room);
        let x_coord = RoomCoordinate::new((self.xx % 50) as u8).unwrap();
        let y_coord = RoomCoordinate::new((self.yy % 50) as u8).unwrap();
        Position::new(x_coord, y_coord, room_name)
    }

    #[inline]
    fn range_to(&self, other: WorldPosition) -> u32 {
        let dx = if other.xx > self.xx { other.xx - self.xx } else { self.xx - other.xx };
        let dy = if other.yy > self.yy { other.yy - self.yy } else { self.yy - other.yy };
        dx.max(dy)
    }

    #[inline]
    fn map_position(&self) -> MapPosition {
        MapPosition::new((self.xx / 50) as u8, (self.yy / 50) as u8)
    }

    #[inline]
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
    open_nodes: HashMap<PosIndex, Cost>,
    closed_set: HashSet<PosIndex>,
    plain_cost: Cost,
    swamp_cost: Cost,
    max_rooms: u8,
    max_ops: u32,
    max_cost: u32,
    flee: bool,
    heuristic_weight: f64,
    debug: bool,
    goals: Vec<Position>,
    goal_positions: Vec<WorldPosition>,
    parents: HashMap<PosIndex, PosIndex>,
    current_room_info: Option<(MapPosition, &'static RoomInfo)>,
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
            reverse_room_table: vec![0; 65536],
            blocked_rooms: HashSet::new(),
            open_set: BinaryHeap::new(),
            open_nodes: HashMap::new(),
            closed_set: HashSet::new(),
            plain_cost,
            swamp_cost,
            max_rooms,
            max_ops,
            max_cost,
            flee,
            heuristic_weight,
            debug: false,
            goals: Vec::new(),
            goal_positions: Vec::new(),
            parents: HashMap::new(),
            current_room_info: None,
        }
    }

    fn debug_log(&self, msg: &str) {
        if self.debug {
            log(msg);
        }
    }

    #[inline]
    fn get_cost(&self, pos: WorldPosition) -> Cost {
        // return 1;
        let map_pos = pos.map_position();
        let room_index = self.reverse_room_table[map_pos.id() as usize];
        if room_index == 0 {
            return OBSTACLE_COST;
        }
        // return 1;
        let room_info = &self.room_table[(room_index - 1) as usize];
        let x = (pos.xx % 50) as u8;
        let y = (pos.yy % 50) as u8;
        let terrain_cost = room_info.get_cost(x, y);
        // Use a lookup table for cost conversion
        match terrain_cost {
            0 => self.plain_cost,
            2 => self.swamp_cost,
            _ => OBSTACLE_COST,
        }
    }

    #[inline]
    fn get_room_info(&self, map_pos: MapPosition) -> Option<&RoomInfo> {
        let room_index = self.reverse_room_table[map_pos.id() as usize];
        if room_index == 0 {
            None
        } else {
            Some(&self.room_table[(room_index - 1) as usize])
        }
    }

    #[inline]
    fn heuristic(&self, pos: WorldPosition) -> Cost {
        let mut min_cost = Cost::MAX;
        for goal_pos in &self.goal_positions {
            let cost = pos.range_to(*goal_pos);
            if (!self.flee && cost < min_cost) || (self.flee && cost > min_cost) {
                min_cost = cost;
            }
        }
        ((min_cost as f64) * self.heuristic_weight) as Cost
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

    fn push_node(&mut self, parent_index: PosIndex, node: WorldPosition, g_cost: Cost) {
        let node_index = match self.index_from_pos(node) {
            Some(index) => index,
            None => return,
        };
        // PROFILER.start_call("push_node");

        if self.closed_set.contains(&node_index) {
            // PROFILER.end_call("push_node");
            return;
        }

        let h_cost = self.heuristic(node);
        let f_cost = g_cost + h_cost;

        // Check if node is already in open set with a better score
        if let Some(&existing_f_cost) = self.open_nodes.get(&node_index) {
            if existing_f_cost <= f_cost {
                // PROFILER.end_call("push_node");
                return;
            }
        }

        // Add or update node
        let state = PathFinderState {
            f_score: f_cost,
            g_score: g_cost,
            position: node,
            parent: Some(parent_index),
        };

        self.open_nodes.insert(node_index, f_cost);
        self.open_set.push(state);
        // PROFILER.end_call("push_node");
    }

    fn jump_neighbor(&mut self, parent_index: PosIndex, pos: WorldPosition, neighbor: WorldPosition, g_cost: Cost, cost: Cost, n_cost: Cost) {
        if n_cost != cost || is_border_pos(neighbor.xx) || is_border_pos(neighbor.yy) {
            if n_cost == OBSTACLE_COST {
                return;
            }
            self.push_node(parent_index, neighbor, g_cost + n_cost);
        } else {
            let dx = (neighbor.xx as i32 - pos.xx as i32).signum();
            let dy = (neighbor.yy as i32 - pos.yy as i32).signum();
            let jump_point = self.jump(n_cost, neighbor, dx, dy);
            
            if !jump_point.is_null() {
                let jump_cost = n_cost * (pos.range_to(jump_point) - 1) + self.get_cost(jump_point);
                self.push_node(parent_index, jump_point, g_cost + jump_cost);
            }
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
        let start_cpu = cpu::get_used();
        let PROFILING_ENABLED = false;
        if PROFILING_ENABLED {
            PROFILER.start_call("init");
        }
        self.debug_log(&format!("Starting search from {:?} to {:?}", origin, goals));
        let origin_pos = WorldPosition::from_position(origin);
        let origin_index = self.index_from_pos(origin_pos)?;

        // Initialize search
        self.open_set.clear();
        self.open_nodes.clear();
        self.closed_set.clear();
        self.goals.clear();
        self.goals.extend_from_slice(goals);
        self.goal_positions.clear();
        self.goal_positions.extend(goals.iter().map(|&pos| WorldPosition::from_position(pos)));
        self.parents.clear();

        // Initial A* setup - we need to add all valid neighbors of the origin
        let origin_cost = self.get_cost(origin_pos);
        if origin_cost == OBSTACLE_COST {
            return None;
        }

        self.expand_astar(origin_pos, origin_index, 0);

        let mut ops_remaining = self.max_ops;
        let mut min_node = None;
        let mut min_node_h_cost = Cost::MAX;
        let mut min_node_g_cost = Cost::MAX;
        let mut total_ops = 0;
        
        if PROFILING_ENABLED {
            PROFILER.end_call("init");
        }
        // Main search loop
        while let Some(current) = self.open_set.pop() {
            if PROFILING_ENABLED {
                PROFILER.start_call("op_init");
            }
            if ops_remaining == 0 {
                self.debug_log("Search terminated: out of operations");
                if PROFILING_ENABLED {
                    PROFILER.end_call("op_init");
                }
                break;
            }

            let current_pos = current.position;
            let current_index = self.index_from_pos(current_pos)?;

            // Skip if already closed
            if self.closed_set.contains(&current_index) {
                if PROFILING_ENABLED {
                    PROFILER.end_call("op_init");
                }
                continue;
            }

            self.closed_set.insert(current_index);

            if let Some(parent) = current.parent {
                self.parents.insert(current_index, parent);
            }

            if PROFILING_ENABLED {
                PROFILER.end_call("op_init");
                PROFILER.start_call("op_score");
            }

            let h_cost = self.heuristic(current_pos);
            let g_cost = current.g_score;

            self.debug_log(&format!("Exploring node at {:?} with h_cost={}, g_cost={}", current_pos, h_cost, g_cost));

            // Check if we've reached a goal
            if h_cost == 0 || (self.flee && h_cost >= 1) {
                min_node = Some(current);
                min_node_h_cost = 0;
                min_node_g_cost = g_cost;
                if PROFILING_ENABLED {
                    PROFILER.end_call("op_score");
                }
                break;
            } else if h_cost < min_node_h_cost || (self.flee && h_cost > min_node_h_cost) {
                min_node = Some(current);
                min_node_h_cost = h_cost;
                min_node_g_cost = g_cost;
            }

            if g_cost + h_cost > self.max_cost {
                self.debug_log("Search terminated: exceeded max cost");
                if PROFILING_ENABLED {
                    PROFILER.end_call("op_score");
                }
                break;
            }
            if PROFILING_ENABLED {
                PROFILER.end_call("op_score");
            }

            // Add next neighbors using JPS
            if PROFILING_ENABLED {
                PROFILER.start_call("op_expand");
            }
            // self.expand_astar(current_pos, current_index, g_cost);
            self.jps(current_index, current_pos, g_cost);
            ops_remaining -= 1;
            total_ops += 1;
            if PROFILING_ENABLED {
                PROFILER.end_call("op_expand");
            }
        }
        let cpu_used = cpu::get_used() - start_cpu;
        log(&format!("Search completed with {} total ops, {} CPU used", total_ops, cpu_used));

        if PROFILING_ENABLED {
            PROFILER.start_call("path_reconstruction");
        }
        // Reconstruct path
        if let Some(end_node) = min_node {
            // Pre-allocate path with estimated capacity
            let mut path = Vec::with_capacity(50);  // Most paths are under 50 steps
            let mut current_pos = end_node.position;
            let mut current_index = self.index_from_pos(current_pos)?;

            // Add the end position
            path.push(current_pos.to_position());

            // Follow parent pointers and interpolate directly
            while let Some(&parent_index) = self.parents.get(&current_index) {
                let parent_pos = self.pos_from_index(parent_index);
                
                // If points are adjacent, just add the parent
                if current_pos.range_to(parent_pos) <= 1 {
                    if parent_pos != origin_pos {
                        path.push(parent_pos.to_position());
                    }
                } else {
                    // Interpolate between points
                    let dx = (parent_pos.xx as i32 - current_pos.xx as i32).signum();
                    let dy = (parent_pos.yy as i32 - current_pos.yy as i32).signum();
                    let mut pos = current_pos;
                    
                    // Calculate number of steps needed
                    let steps = current_pos.range_to(parent_pos) - 1;
                    for _ in 0..steps {
                        pos = WorldPosition::new(
                            (pos.xx as i32 + dx) as u32,
                            (pos.yy as i32 + dy) as u32
                        );
                        path.push(pos.to_position());
                    }
                }

                if parent_pos == origin_pos {
                    break;
                }
                current_pos = parent_pos;
                current_index = parent_index;
            }
            path.reverse();
            if PROFILING_ENABLED {
                PROFILER.end_call("path_reconstruction");
            }
            PROFILER.print_results();
            let total_cpu = cpu::get_used() - start_cpu;
            log(&format!("Path found in {} CPU", total_cpu));
            Some(path)
        } else {
            None
        }
    }

    fn expand_astar(&mut self, origin_pos: WorldPosition, origin_index: u32, g_cost: Cost) {
        // Add initial neighbors in all 8 directions
        for dx in [-1, 0, 1].iter() {
            for dy in [-1, 0, 1].iter() {
                if *dx == 0 && *dy == 0 {
                    continue;
                }
                let neighbor = WorldPosition::new(
                    (origin_pos.xx as i32 + dx) as u32,
                    (origin_pos.yy as i32 + dy) as u32
                );
                let n_cost = self.get_cost(neighbor);
                if n_cost != OBSTACLE_COST {
                    self.push_node(origin_index, neighbor, g_cost + n_cost);
                }
            }
        }
    }
    
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    fn jump_x(&self, cost: Cost, mut pos: WorldPosition, dx: i32) -> WorldPosition {
        let mut prev_cost_u = self.get_cost(WorldPosition::new(pos.xx, pos.yy.wrapping_sub(1)));
        let mut prev_cost_d = self.get_cost(WorldPosition::new(pos.xx, pos.yy + 1));
        
        loop {
            if self.heuristic(pos) == 0 || is_near_border_pos(pos.xx) {
                break;
            }

            let cost_u = self.get_cost(WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy.wrapping_sub(1)));
            let cost_d = self.get_cost(WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy + 1));
            
            if (cost_u != OBSTACLE_COST && prev_cost_u != cost) ||
               (cost_d != OBSTACLE_COST && prev_cost_d != cost) {
                break;
            }
            
            prev_cost_u = cost_u;
            prev_cost_d = cost_d;
            pos.xx = (pos.xx as i32 + dx) as u32;

            let jump_cost = self.get_cost(pos);
            if jump_cost == OBSTACLE_COST {
                pos = WorldPosition::null();
                break;
            } else if jump_cost != cost {
                break;
            }
        }
        pos
    }

    fn jump_y(&self, cost: Cost, mut pos: WorldPosition, dy: i32) -> WorldPosition {
        let mut prev_cost_l = self.get_cost(WorldPosition::new(pos.xx.wrapping_sub(1), pos.yy));
        let mut prev_cost_r = self.get_cost(WorldPosition::new(pos.xx + 1, pos.yy));
        
        loop {
            if self.heuristic(pos) == 0 || is_near_border_pos(pos.yy) {
                break;
            }

            let cost_l = self.get_cost(WorldPosition::new(pos.xx.wrapping_sub(1), (pos.yy as i32 + dy) as u32));
            let cost_r = self.get_cost(WorldPosition::new(pos.xx + 1, (pos.yy as i32 + dy) as u32));
            
            if (cost_l != OBSTACLE_COST && prev_cost_l != cost) ||
               (cost_r != OBSTACLE_COST && prev_cost_r != cost) {
                break;
            }
            
            prev_cost_l = cost_l;
            prev_cost_r = cost_r;
            pos.yy = (pos.yy as i32 + dy) as u32;

            let jump_cost = self.get_cost(pos);
            if jump_cost == OBSTACLE_COST {
                pos = WorldPosition::null();
                break;
            } else if jump_cost != cost {
                break;
            }
        }
        pos
    }

    fn jump_xy(&self, cost: Cost, mut pos: WorldPosition, dx: i32, dy: i32) -> WorldPosition {
        let mut prev_cost_x = self.get_cost(WorldPosition::new(pos.xx.wrapping_sub(dx as u32), pos.yy));
        let mut prev_cost_y = self.get_cost(WorldPosition::new(pos.xx, pos.yy.wrapping_sub(dy as u32)));
        
        loop {
            if self.heuristic(pos) == 0 || is_near_border_pos(pos.xx) || is_near_border_pos(pos.yy) {
                break;
            }

            if (self.get_cost(WorldPosition::new(pos.xx.wrapping_sub(dx as u32), (pos.yy as i32 + dy) as u32)) != OBSTACLE_COST && prev_cost_x != cost) ||
               (self.get_cost(WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy.wrapping_sub(dy as u32))) != OBSTACLE_COST && prev_cost_y != cost) {
                break;
            }

            prev_cost_x = self.get_cost(WorldPosition::new(pos.xx, (pos.yy as i32 + dy) as u32));
            prev_cost_y = self.get_cost(WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy));
            
            if (prev_cost_y != OBSTACLE_COST && !self.jump_x(cost, WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy), dx).is_null()) ||
               (prev_cost_x != OBSTACLE_COST && !self.jump_y(cost, WorldPosition::new(pos.xx, (pos.yy as i32 + dy) as u32), dy).is_null()) {
                break;
            }

            pos.xx = (pos.xx as i32 + dx) as u32;
            pos.yy = (pos.yy as i32 + dy) as u32;

            let jump_cost = self.get_cost(pos);
            if jump_cost == OBSTACLE_COST {
                pos = WorldPosition::null();
                break;
            } else if jump_cost != cost {
                break;
            }
        }
        pos
    }

    fn jump(&self, cost: Cost, pos: WorldPosition, dx: i32, dy: i32) -> WorldPosition {
        if dx != 0 {
            if dy != 0 {
                self.jump_xy(cost, pos, dx, dy)
            } else {
                self.jump_x(cost, pos, dx)
            }
        } else {
            self.jump_y(cost, pos, dy)
        }
    }

    fn jps(&mut self, index: PosIndex, pos: WorldPosition, g_cost: Cost) {
        let parent = if let Some(&parent_index) = self.parents.get(&index) {
            self.pos_from_index(parent_index)
        } else {
            return;  // No parent found
        };

        let dx = if pos.xx > parent.xx { 1 } else if pos.xx < parent.xx { -1 } else { 0 };
        let dy = if pos.yy > parent.yy { 1 } else if pos.yy < parent.yy { -1 } else { 0 };

        // First check to see if we're jumping to/from a border, options are limited in this case
        let mut neighbors = Vec::with_capacity(3);
        if pos.xx % 50 == 0 {
            if dx == -1 {
                neighbors.push(WorldPosition::new(pos.xx.wrapping_sub(1), pos.yy));
            } else if dx == 1 {
                neighbors.push(WorldPosition::new(pos.xx + 1, pos.yy.wrapping_sub(1)));
                neighbors.push(WorldPosition::new(pos.xx + 1, pos.yy));
                neighbors.push(WorldPosition::new(pos.xx + 1, pos.yy + 1));
            }
        } else if pos.xx % 50 == 49 {
            if dx == 1 {
                neighbors.push(WorldPosition::new(pos.xx + 1, pos.yy));
            } else if dx == -1 {
                neighbors.push(WorldPosition::new(pos.xx.wrapping_sub(1), pos.yy.wrapping_sub(1)));
                neighbors.push(WorldPosition::new(pos.xx.wrapping_sub(1), pos.yy));
                neighbors.push(WorldPosition::new(pos.xx.wrapping_sub(1), pos.yy + 1));
            }
        } else if pos.yy % 50 == 0 {
            if dy == -1 {
                neighbors.push(WorldPosition::new(pos.xx, pos.yy.wrapping_sub(1)));
            } else if dy == 1 {
                neighbors.push(WorldPosition::new(pos.xx.wrapping_sub(1), pos.yy + 1));
                neighbors.push(WorldPosition::new(pos.xx, pos.yy + 1));
                neighbors.push(WorldPosition::new(pos.xx + 1, pos.yy + 1));
            }
        } else if pos.yy % 50 == 49 {
            if dy == 1 {
                neighbors.push(WorldPosition::new(pos.xx, pos.yy + 1));
            } else if dy == -1 {
                neighbors.push(WorldPosition::new(pos.xx.wrapping_sub(1), pos.yy.wrapping_sub(1)));
                neighbors.push(WorldPosition::new(pos.xx, pos.yy.wrapping_sub(1)));
                neighbors.push(WorldPosition::new(pos.xx + 1, pos.yy.wrapping_sub(1)));
            }
        }

        // Add special nodes from the above blocks to the heap
        if !neighbors.is_empty() {
            for neighbor in neighbors {
                let n_cost = self.get_cost(neighbor);
                if n_cost != OBSTACLE_COST {
                    self.push_node(index, neighbor, g_cost + n_cost);
                }
            }
            return;
        }

        // Regular JPS iteration follows

        // First check to see if we're close to borders
        let border_dx = if pos.xx % 50 == 1 {
            -1
        } else if pos.xx % 50 == 48 {
            1
        } else {
            0
        };

        let border_dy = if pos.yy % 50 == 1 {
            -1
        } else if pos.yy % 50 == 48 {
            1
        } else {
            0
        };

        // Now execute the logic that is shared between diagonal and straight jumps
        let cost = self.get_cost(pos);
        if dx != 0 {
            let neighbor = WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy);
            let n_cost = self.get_cost(neighbor);
            if n_cost != OBSTACLE_COST {
                if border_dy == 0 {
                    self.jump_neighbor(index, pos, neighbor, g_cost, cost, n_cost);
                } else {
                    self.push_node(index, neighbor, g_cost + n_cost);
                }
            }
        }
        if dy != 0 {
            let neighbor = WorldPosition::new(pos.xx, (pos.yy as i32 + dy) as u32);
            let n_cost = self.get_cost(neighbor);
            if n_cost != OBSTACLE_COST {
                if border_dx == 0 {
                    self.jump_neighbor(index, pos, neighbor, g_cost, cost, n_cost);
                } else {
                    self.push_node(index, neighbor, g_cost + n_cost);
                }
            }
        }

        // Forced neighbor rules
        if dx != 0 {
            if dy != 0 { // Jumping diagonally
                let neighbor = WorldPosition::new((pos.xx as i32 + dx) as u32, (pos.yy as i32 + dy) as u32);
                let n_cost = self.get_cost(neighbor);
                if n_cost != OBSTACLE_COST {
                    self.jump_neighbor(index, pos, neighbor, g_cost, cost, n_cost);
                }
                
                if self.get_cost(WorldPosition::new(pos.xx.wrapping_sub(dx as u32), pos.yy)) != cost {
                    let forced = WorldPosition::new(pos.xx.wrapping_sub(dx as u32), (pos.yy as i32 + dy) as u32);
                    self.jump_neighbor(index, pos, forced, g_cost, cost, self.get_cost(forced));
                }
                if self.get_cost(WorldPosition::new(pos.xx, pos.yy.wrapping_sub(dy as u32))) != cost {
                    let forced = WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy.wrapping_sub(dy as u32));
                    self.jump_neighbor(index, pos, forced, g_cost, cost, self.get_cost(forced));
                }
            } else { // Jumping left / right
                if border_dy == 1 || self.get_cost(WorldPosition::new(pos.xx, pos.yy + 1)) != cost {
                    let forced = WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy + 1);
                    self.jump_neighbor(index, pos, forced, g_cost, cost, self.get_cost(forced));
                }
                if border_dy == -1 || self.get_cost(WorldPosition::new(pos.xx, pos.yy.wrapping_sub(1))) != cost {
                    let forced = WorldPosition::new((pos.xx as i32 + dx) as u32, pos.yy.wrapping_sub(1));
                    self.jump_neighbor(index, pos, forced, g_cost, cost, self.get_cost(forced));
                }
            }
        } else { // Jumping up / down
            if border_dx == 1 || self.get_cost(WorldPosition::new(pos.xx + 1, pos.yy)) != cost {
                let forced = WorldPosition::new(pos.xx + 1, (pos.yy as i32 + dy) as u32);
                self.jump_neighbor(index, pos, forced, g_cost, cost, self.get_cost(forced));
            }
            if border_dx == -1 || self.get_cost(WorldPosition::new(pos.xx.wrapping_sub(1), pos.yy)) != cost {
                let forced = WorldPosition::new(pos.xx.wrapping_sub(1), (pos.yy as i32 + dy) as u32);
                self.jump_neighbor(index, pos, forced, g_cost, cost, self.get_cost(forced));
            }
        }
    }
}

#[inline]
fn is_border_pos(val: u32) -> bool {
    (val + 1) % 50 < 2
}

#[inline]
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
