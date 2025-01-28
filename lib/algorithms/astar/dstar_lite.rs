use crate::datatypes::{
    CustomCostMatrix, PositionIndex, Path
};
use screeps::{Direction, RoomName, Position};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::convert::TryFrom;
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
                            let cost = edge_cost + self.states.get(&next).copied().unwrap_or_default().g;
                            min_rhs = min_rhs.min(cost);
                        }
                    }
                }
            }
            self.states.get_mut(&pos).unwrap().rhs = min_rhs;
        }
        
        let key = self.calculate_key(pos);
        self.queue.push(QueueState { pos, k: key });
    }
    
    fn compute_shortest_path(
        &mut self,
        get_cost_matrix: &impl Fn(RoomName) -> Option<CustomCostMatrix>,
        max_ops: &mut usize,
    ) -> bool {
        while let Some(top) = self.queue.peek() {
            let start_key = self.calculate_key(self.start);
            let start_state = self.states.get(&self.start).copied().unwrap_or_default();
            
            if !(top.k < start_key || start_state.rhs != start_state.g) || *max_ops == 0 {
                break;
            }
            
            *max_ops -= 1;
            let u = self.queue.pop().unwrap().pos;
            let k_old = self.calculate_key(u);
            let k_new = self.calculate_key(u);

            if k_old < k_new {
                self.queue.push(QueueState { pos: u, k: k_new });
            } else {
                let state = self.states.get(&u).copied().unwrap_or_default();
                if state.g > state.rhs {
                    self.states.get_mut(&u).unwrap().g = state.rhs;
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
                        if let Some(next) = u.r#move(dir) {
                            self.update_vertex(next, get_cost_matrix);
                            let key = self.calculate_key(next);
                            self.queue.push(QueueState { pos: next, k: key });
                        }
                    }
                } else {
                    let mut state = self.states.get(&u).copied().unwrap_or_default();
                    state.g = usize::MAX;
                    self.states.insert(u, state);
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
                        if let Some(next) = u.r#move(dir) {
                            self.update_vertex(next, get_cost_matrix);
                            let key = self.calculate_key(next);
                            self.queue.push(QueueState { pos: next, k: key });
                        }
                    }
                    self.update_vertex(u, get_cost_matrix);
                    let key = self.calculate_key(u);
                    self.queue.push(QueueState { pos: u, k: key });
                }
            }
        }

        *max_ops > 0
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
            if let Some(next) = current.r#move(dir) {
                if let Some(cost_matrix) = get_cost_matrix(next.room_name()) {
                    let cost = cost_matrix.get_local(next.local()) as usize;
                    if cost < 255 {
                        if let Some(g) = dstar.states.get(&next).map(|s| s.g) {
                            let total_cost = cost + g;
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
