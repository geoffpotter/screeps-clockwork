
use std::sync::Arc;
use screeps::{Position, RoomName};
use crate::datatypes::{ClockworkCostMatrix, OptionalCache};
use crate::utils::Profiler;

pub struct CostCache<'a> {
    current_room_cost_matrix: Option<ClockworkCostMatrix>,
    current_matrix_room: Option<RoomName>,
    cost_matrices: OptionalCache<'a, RoomName, ClockworkCostMatrix>,
    profiler: Arc<Profiler>,
}

impl<'a> CostCache<'a> {
    pub fn new<F>(get_cost_matrix: F, profiler: Arc<Profiler>) -> Self 
    where 
        F: Fn(RoomName) -> Option<ClockworkCostMatrix> + 'a
    {
        let cost_matrices = OptionalCache::new(get_cost_matrix);

        Self {
            current_room_cost_matrix: None,
            current_matrix_room: None,
            cost_matrices,
            profiler,
        }
    }
    #[inline(always)]
    pub fn get_cost(&mut self, position: Position) -> u8 {
        // self.profiler.start_call("get_cost");
        let room_name = position.room_name();

        // Fast path: same room
        if Some(room_name) == self.current_matrix_room {
            let result = self.current_room_cost_matrix
                .as_ref()
                .unwrap()
                .get(position.xy());
            // self.profiler.end_call("get_cost");
            return result
        }

        // Room change needed
        let next_matrix = self.cost_matrices.get_or_create(room_name);
        match next_matrix {
            Some(matrix) => {
                self.current_room_cost_matrix = Some(matrix);
                self.current_matrix_room = Some(room_name);
                let result = self.current_room_cost_matrix
                    .as_ref()
                    .unwrap()
                    .get(position.xy());
                // self.profiler.end_call("get_cost");
                result
            }
            None => {
                // self.profiler.end_call("get_cost");
                255
            }
        }
    }
} 