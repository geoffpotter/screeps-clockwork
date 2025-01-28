use crate::datatypes::{
    ClockworkCostMatrix, CustomCostMatrix, LocalIndex, MultiroomGenericMap, OptionalCache, PositionIndex, RoomIndex, Path
};
use crate::log;
use lazy_static::lazy_static;
use screeps::{Direction, RoomName};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use wasm_bindgen::prelude::*;
use js_sys::Function;

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

#[derive(Copy, Clone)]
struct State {
    g: usize,
    rhs: usize,
    key: Key,
}

impl Default for State {
    fn default() -> Self {
        Self {
            g: usize::MAX,
            rhs: usize::MAX,
            key: Key { k1: 0, k2: 0 },
        }
    }
}

struct DStarLite {
    states: HashMap<PositionIndex, State>,
    queue: BinaryHeap<(Key, PositionIndex)>,
    k_m: usize,
    start: PositionIndex,
    goal: PositionIndex,
    last_pos: PositionIndex,
}

impl DStarLite {
    fn new(start: PositionIndex, goal: PositionIndex) -> Self {
        let mut dstar = Self {
            states: HashMap::new(),
            queue: BinaryHeap::new(),
            k_m: 0,
            start,
            goal,
            last_pos: start,
        };
        
        dstar.states.insert(goal, State {
            g: usize::MAX,
            rhs: 0,
            key: Key { k1: 0, k2: 0 },
        });
        
        dstar.queue.push((
            dstar.calculate_key(goal),
            goal
        ));
        
        dstar
    }
    
    fn heuristic(&self, pos: PositionIndex) -> usize {
        let dx = (pos.room.x as i32 - self.start.room.x as i32).abs() as usize * 50 +
                (pos.local.x as i32 - self.start.local.x as i32).abs() as usize;
        let dy = (pos.room.y as i32 - self.start.room.y as i32).abs() as usize * 50 +
                (pos.local.y as i32 - self.start.local.y as i32).abs() as usize;
        dx.max(dy)
    }
    
    fn calculate_key(&self, pos: PositionIndex) -> Key {
        let state = self.states.get(&pos).unwrap_or(&State::default());
        let min_g_rhs = state.g.min(state.rhs);
        Key {
            k1: min_g_rhs + self.heuristic(pos) + self.k_m,
            k2: min_g_rhs,
        }
    }
    
    fn update_vertex(&mut self, pos: PositionIndex, get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>) {
        if pos != self.goal {
            let mut min_rhs = usize::MAX;
            for direction in Direction::all() {
                if let Some(next) = pos.step(direction) {
                    if let Some(cost_matrix) = get_cost_matrix(next.room.to_room_name()) {
                        let cost = cost_matrix.get_cost(next.local) as usize;
                        if cost < 255 {
                            let next_state = self.states.get(&next).unwrap_or(&State::default());
                            min_rhs = min_rhs.min(cost + next_state.g);
                        }
                    }
                }
            }
            let state = self.states.entry(pos).or_default();
            state.rhs = min_rhs;
        }
        
        let state = self.states.get(&pos).unwrap();
        if state.g != state.rhs {
            self.queue.push((self.calculate_key(pos), pos));
        }
    }
    
    fn compute_shortest_path(
        &mut self,
        get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>,
        max_ops: &mut usize,
    ) -> bool {
        while let Some((k_old, pos)) = self.queue.pop() {
            if *max_ops == 0 {
                return false;
            }
            *max_ops -= 1;
            
            let k_new = self.calculate_key(pos);
            if k_old > k_new {
                self.queue.push((k_new, pos));
            } else {
                let state = self.states.get(&pos).unwrap();
                if state.g > state.rhs {
                    let new_g = state.rhs;
                    self.states.get_mut(&pos).unwrap().g = new_g;
                    
                    for direction in Direction::all() {
                        if let Some(next) = pos.step(direction) {
                            self.update_vertex(next, get_cost_matrix);
                        }
                    }
                } else {
                    let old_g = state.g;
                    self.states.get_mut(&pos).unwrap().g = usize::MAX;
                    
                    let mut to_update = vec![pos];
                    for direction in Direction::all() {
                        if let Some(next) = pos.step(direction) {
                            if let Some(next_state) = self.states.get(&next) {
                                if next_state.rhs == old_g + 1 {
                                    to_update.push(next);
                                }
                            }
                        }
                    }
                    
                    for update_pos in to_update {
                        self.update_vertex(update_pos, get_cost_matrix);
                    }
                }
            }
        }
        
        true
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
    
    let mut path = Vec::new();
    let mut current = start;
    path.push(current);
    
    while current != goal && path.len() < max_path_length {
        if remaining_ops == 0 {
            return None;
        }
        remaining_ops -= 1;
        
        let mut min_cost = usize::MAX;
        let mut next_pos = None;
        
        for direction in Direction::all() {
            if let Some(next) = current.step(direction) {
                if let Some(cost_matrix) = get_cost_matrix(next.room.to_room_name()) {
                    let cost = cost_matrix.get_cost(next.local) as usize;
                    if cost < 255 {
                        if let Some(state) = dstar.states.get(&next) {
                            let total_cost = cost + state.g;
                            if total_cost < min_cost {
                                min_cost = total_cost;
                                next_pos = Some(next);
                            }
                        }
                    }
                }
            }
        }
        
        if let Some(next) = next_pos {
            current = next;
            path.push(current);
        } else {
            break;
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
    let start = PositionIndex::from(start_packed);
    let goal = PositionIndex::from(goal_packed);
    
    let get_cost_matrix = move |room_name: RoomName| -> Option<CustomCostMatrix> {
        let result = get_cost_matrix
            .call1(&JsValue::NULL, &JsValue::from(room_name.to_string()))
            .ok()?;
        if result.is_null() || result.is_undefined() {
            return None;
        }
        Some(CustomCostMatrix::try_from(result).ok()?)
    };
    
    dstar_lite_path(
        start,
        goal,
        get_cost_matrix,
        max_ops as usize,
        max_path_length as usize,
    ).map(|positions| Path::new(positions))
}
