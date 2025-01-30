use crate::datatypes::{
    CustomCostMatrix, PositionIndex, Path
};
use crate::log;
use screeps::{Direction, RoomName, Position};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;
use js_sys::Function;

const LOGGING_ENABLED: bool = false;

#[derive(Copy, Clone, Eq, PartialEq)]
struct Key {
    k1: usize,
    k2: usize,
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.k1.cmp(&other.k1) {
            Ordering::Equal => self.k2.cmp(&other.k2),
            other => other,
        }
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct QueueState {
    k: Key,
    pos: PositionIndex,
}

impl Ord for QueueState {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for max-heap to act as min-heap
        other.k.cmp(&self.k)
    }
}

impl PartialOrd for QueueState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Copy, Clone)]
struct NodeState {
    g: usize,
    rhs: usize,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            g: usize::MAX,
            rhs: usize::MAX,
        }
    }
}

struct DStarLite {
    states: HashMap<PositionIndex, NodeState>,
    queue: BinaryHeap<QueueState>,
    k_m: usize,
    start: PositionIndex,
    goal: PositionIndex,
}

impl DStarLite {
    fn new(start: PositionIndex, goal: PositionIndex) -> Self {
        let mut dstar = Self {
            states: HashMap::new(),
            queue: BinaryHeap::new(),
            k_m: 0,
            start,
            goal,
        };
        
        let mut goal_state = NodeState::default();
        goal_state.rhs = 0;
        dstar.states.insert(goal, goal_state);
        
        let key = dstar.calculate_key(goal);
        dstar.queue.push(QueueState { pos: goal, k: key });
        
        dstar
    }
    
    fn heuristic(&self, pos: PositionIndex) -> usize {
        // Manhattan distance between rooms plus Manhattan distance within room
        let (start_x, start_y) = pos.room().room_xy();
        let (goal_x, goal_y) = self.goal.room().room_xy();
        let room_distance = ((start_x as i32 - goal_x as i32).abs() + 
                           (start_y as i32 - goal_y as i32).abs()) as usize;
        
        let local_distance = ((pos.local().x() as i32 - self.goal.local().x() as i32).abs() +
                            (pos.local().y() as i32 - self.goal.local().y() as i32).abs()) as usize;
        
        room_distance * 50 + local_distance
    }
    
    fn calculate_key(&self, pos: PositionIndex) -> Key {
        let state = self.states.get(&pos).copied().unwrap_or_default();
        let min_g_rhs = state.g.min(state.rhs);
        Key {
            k1: min_g_rhs + self.heuristic(pos) + self.k_m,
            k2: min_g_rhs,
        }
    }
    
    fn update_vertex(&mut self, pos: PositionIndex, get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>) {
        if LOGGING_ENABLED {
            log(&format!("Updating vertex at {:?}", pos));
        }
        if pos != self.goal {
            let mut min_rhs = usize::MAX;
            let directions = [
                Direction::Top,
                Direction::TopRight,
                Direction::Right,
                Direction::BottomRight,
                Direction::Bottom,
                Direction::BottomLeft,
                Direction::Left,
                Direction::TopLeft,
            ];
            
            for &dir in &directions {
                if let Some(next) = pos.r#move(dir) {
                    if let Some(cost_matrix) = get_cost_matrix(next.room_name()) {
                        let edge_cost = cost_matrix.get_local(next.local()) as usize;
                        if edge_cost < 255 {
                            let next_state = self.states.entry(next).or_insert_with(NodeState::default);
                            if next_state.g != usize::MAX {
                                let total_cost = edge_cost + next_state.g;
                                if total_cost < min_rhs {
                                    min_rhs = total_cost;
                                    if LOGGING_ENABLED {
                                        log(&format!("Found better path through {:?} with cost {} (edge={}, g={})", 
                                            next, total_cost, edge_cost, next_state.g));
                                    }
                                }
                            } else if LOGGING_ENABLED {
                                log(&format!("Skipping path through {:?} as its g value is infinity", next));
                            }
                        }
                    }
                }
            }

            let old_rhs = self.states.get(&pos).map(|s| s.rhs).unwrap_or(usize::MAX);
            let state = self.states.entry(pos).or_insert_with(NodeState::default);
            state.rhs = min_rhs;
            if LOGGING_ENABLED {
                log(&format!("Updating rhs from {} to {} for pos {:?}", old_rhs, min_rhs, pos));
            }
        }
        
        let g = self.states.get(&pos).map(|s| s.g).unwrap_or(usize::MAX);
        let rhs = self.states.get(&pos).map(|s| s.rhs).unwrap_or(usize::MAX);
        if g != rhs {
            let key = self.calculate_key(pos);
            if LOGGING_ENABLED {
                log(&format!("State inconsistent (g={}, rhs={}), adding to queue with key ({}, {})", 
                    g, rhs, key.k1, key.k2));
            }
            self.queue.retain(|state| state.pos != pos);
            self.queue.push(QueueState { pos, k: key });
        } else if LOGGING_ENABLED {
            log(&format!("State consistent (g=rhs={}), not adding to queue", g));
        }
    }
    
    fn compute_shortest_path(
        &mut self,
        get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>,
        max_ops: &mut usize,
    ) -> bool {
        // Initialize start state if needed
        self.update_vertex(self.start, get_cost_matrix);
        
        while let Some(top) = self.queue.peek() {
            let start_key = self.calculate_key(self.start);
            let start_state = self.states.get(&self.start).copied().unwrap_or_default();
            
            if LOGGING_ENABLED {
                log(&format!("Top of queue: pos={:?}, k=({}, {})", top.pos, top.k.k1, top.k.k2));
                log(&format!("Start state: g={}, rhs={}", start_state.g, start_state.rhs));
            }
            
            // Only break if start is consistent AND its rhs value is not infinity
            if start_state.rhs == start_state.g && start_state.rhs != usize::MAX {
                if top.k >= start_key {
                    if LOGGING_ENABLED {
                        log(&format!("Breaking: start is consistent with valid cost (g=rhs={})", start_state.g));
                    }
                    break;
                }
            }
            
            if *max_ops == 0 {
                if LOGGING_ENABLED {
                    log("Breaking: max_ops reached 0");
                }
                break;
            }
            
            *max_ops -= 1;
            let u = self.queue.pop().unwrap();
            let k_new = self.calculate_key(u.pos);
            
            if LOGGING_ENABLED {
                log(&format!("Processing pos={:?}, old_k=({}, {}), new_k=({}, {})", 
                    u.pos, u.k.k1, u.k.k2, k_new.k1, k_new.k2));
            }

            if u.k < k_new {
                if LOGGING_ENABLED {
                    log("Key out of date, reinserting with new key");
                }
                self.queue.push(QueueState { pos: u.pos, k: k_new });
            } else {
                let state = self.states.get(&u.pos).copied().unwrap_or_default();
                if LOGGING_ENABLED {
                    log(&format!("Current state: g={}, rhs={}", state.g, state.rhs));
                }
                
                if state.g > state.rhs {
                    if LOGGING_ENABLED {
                        log("Node is overconsistent (g > rhs), making consistent");
                    }
                    // Make node consistent
                    if let Some(state) = self.states.get_mut(&u.pos) {
                        state.g = state.rhs;
                        if LOGGING_ENABLED {
                            log(&format!("Updated g to {}", state.g));
                        }
                    }
                    
                    // Update affected neighbors
                    let directions = [
                        Direction::Top,
                        Direction::TopRight,
                        Direction::Right,
                        Direction::BottomRight,
                        Direction::Bottom,
                        Direction::BottomLeft,
                        Direction::Left,
                        Direction::TopLeft,
                    ];
                    for &dir in &directions {
                        if let Some(next) = u.pos.r#move(dir) {
                            if let Some(cost_matrix) = get_cost_matrix(next.room_name()) {
                                let edge_cost = cost_matrix.get_local(next.local()) as usize;
                                if edge_cost < 255 {
                                    if LOGGING_ENABLED {
                                        log(&format!("Updating neighbor at {:?}", next));
                                    }
                                    self.update_vertex(next, get_cost_matrix);
                                }
                            }
                        }
                    }
                } else {
                    if LOGGING_ENABLED {
                        log("Node is underconsistent (g ≤ rhs), making overconsistent");
                    }
                    let old_g = state.g;
                    // Make node overconsistent
                    if let Some(state) = self.states.get_mut(&u.pos) {
                        state.g = usize::MAX;
                        if LOGGING_ENABLED {
                            log(&format!("Set g to infinity for pos {:?}", u.pos));
                        }
                    }
                    
                    // Update node and affected neighbors
                    let mut affected = vec![u.pos];
                    let directions = [
                        Direction::Top,
                        Direction::TopRight,
                        Direction::Right,
                        Direction::BottomRight,
                        Direction::Bottom,
                        Direction::BottomLeft,
                        Direction::Left,
                        Direction::TopLeft,
                    ];
                    for &dir in &directions {
                        if let Some(next) = u.pos.r#move(dir) {
                            if let Some(cost_matrix) = get_cost_matrix(next.room_name()) {
                                let edge_cost = cost_matrix.get_local(next.local()) as usize;
                                if edge_cost < 255 {
                                    if let Some(next_state) = self.states.get(&next) {
                                        if old_g != usize::MAX && next_state.rhs == old_g + edge_cost {
                                            if LOGGING_ENABLED {
                                                log(&format!("Found affected neighbor at {:?} (rhs={}, old_g={})", 
                                                    next, next_state.rhs, old_g));
                                            }
                                            affected.push(next);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    if LOGGING_ENABLED {
                        log(&format!("Updating {} affected vertices", affected.len()));
                    }
                    for pos in affected {
                        self.update_vertex(pos, get_cost_matrix);
                    }
                }
            }
            
            // Periodically update start vertex to ensure it gets proper costs
            if *max_ops % 100 == 0 {
                self.update_vertex(self.start, get_cost_matrix);
            }
        }

        // Return true only if we found a valid path (start has valid rhs)
        let start_state = self.states.get(&self.start).copied().unwrap_or_default();
        start_state.rhs != usize::MAX && *max_ops > 0
    }
}

pub fn dstar_lite_path(
    start: PositionIndex,
    goal: PositionIndex,
    get_cost_matrix: impl Fn(RoomName) -> Option<CustomCostMatrix>,
    max_ops: usize,
    max_path_length: usize,
) -> Option<Vec<PositionIndex>> {
    let mut dstar = DStarLite::new(start, goal);
    let mut remaining_ops = max_ops;
    
    if !dstar.compute_shortest_path(&get_cost_matrix, &mut remaining_ops) {
        return None;
    }
    
    // Extract path more efficiently
    let mut path = Vec::with_capacity(max_path_length);
    let mut current = start;
    path.push(current);
    
    while current != goal && path.len() < max_path_length {
        if remaining_ops == 0 {
            return None;
        }
        remaining_ops -= 1;
        
        let current_g = match dstar.states.get(&current) {
            Some(state) => state.g,
            None => return None
        };
        
        let mut best_next = None;
        let mut best_g = usize::MAX;
        
        let directions = [
            Direction::Top,
            Direction::TopRight,
            Direction::Right,
            Direction::BottomRight,
            Direction::Bottom,
            Direction::BottomLeft,
            Direction::Left,
            Direction::TopLeft,
        ];
        
        // First pass: find the minimum g-value among neighbors
        for &dir in &directions {
            if let Some(next) = current.r#move(dir) {
                if let Some(next_state) = dstar.states.get(&next) {
                    if next_state.g < best_g {
                        best_g = next_state.g;
                        best_next = Some(next);
                    }
                }
            }
        }
        
        match best_next {
            Some(next) => {
                // Verify this step actually makes progress
                if best_g >= current_g {
                    return None; // No progress possible
                }
                current = next;
                path.push(current);
            }
            None => return None
        }
    }
    
    if current == goal {
        Some(path)
    } else {
        None
    }
}

#[wasm_bindgen]
pub fn js_dstar_lite_path(
    start_packed: u32,
    goal_packed: u32,
    get_cost_matrix: Function,
    max_ops: u32,
    max_path_length: u32,
) -> Option<Path> {
    let start = Position::from_packed(start_packed);
    let goal = Position::from_packed(goal_packed);
    let start = PositionIndex::from(start);
    let goal = PositionIndex::from(goal);

    let get_cost_matrix = |room_name: RoomName| {
        let result = get_cost_matrix.call1(
            &JsValue::NULL,
            &JsValue::from_f64(room_name.packed_repr() as f64),
        );
        match result {
            Ok(value) => {
                if value.is_undefined() {
                    None
                } else {
                    CustomCostMatrix::try_from(value).ok()
                }
            }
            Err(_) => None,
        }
    };

    dstar_lite_path(
        start,
        goal,
        get_cost_matrix,
        max_ops as usize,
        max_path_length as usize,
    )
    .map(|positions| {
        let screeps_positions: Vec<Position> = positions.into_iter().map(Position::from).collect();
        Path::from(screeps_positions)
    })
}
