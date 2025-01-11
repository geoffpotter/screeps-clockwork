use std::{collections::HashMap, ops::{Index, IndexMut}, convert::TryFrom};
use screeps::{Position, RoomName, RoomXY, Terrain, LocalRoomTerrain, RoomCoordinate};

const DIR_NONE: (i8, i8, usize) = (0, 0, 0);
const DIR_TOP: (i8, i8, usize) = (0, -1, 1);
const DIR_TOP_RIGHT: (i8, i8, usize) = (1, -1, 2);
const DIR_RIGHT: (i8, i8, usize) = (1, 0, 3);
const DIR_BOTTOM_RIGHT: (i8, i8, usize) = (1, 1, 4);
const DIR_BOTTOM: (i8, i8, usize) = (0, 1, 5);
const DIR_BOTTOM_LEFT: (i8, i8, usize) = (-1, 1, 6);
const DIR_LEFT: (i8, i8, usize) = (-1, 0, 7);
const DIR_TOP_LEFT: (i8, i8, usize) = (-1, -1, 8);

const DIRECTION_LOOKUP: &[(usize, [(i8, i8, usize); 8])] = &[
	// 0: Any direction
	(8, [DIR_TOP, DIR_TOP_RIGHT, DIR_RIGHT, DIR_BOTTOM_RIGHT, DIR_BOTTOM, DIR_BOTTOM_LEFT, DIR_LEFT, DIR_TOP_LEFT]),
	// 1: Top
	(3, [DIR_TOP, DIR_TOP_LEFT, DIR_TOP_RIGHT, DIR_NONE, DIR_NONE, DIR_NONE, DIR_NONE, DIR_NONE]),
	// 2: TopRight
	(5, [DIR_BOTTOM_RIGHT, DIR_TOP_LEFT, DIR_TOP, DIR_RIGHT, DIR_TOP_RIGHT, DIR_NONE, DIR_NONE, DIR_NONE]),
	// 3: Right
	(3, [DIR_RIGHT, DIR_TOP_RIGHT, DIR_BOTTOM_RIGHT, DIR_NONE, DIR_NONE, DIR_NONE, DIR_NONE, DIR_NONE]),
	// 4: BottomRight
	(5, [DIR_TOP_RIGHT, DIR_BOTTOM_LEFT, DIR_RIGHT, DIR_BOTTOM, DIR_BOTTOM_RIGHT, DIR_NONE, DIR_NONE, DIR_NONE]),
	// 5: Bottom
	(3, [DIR_BOTTOM, DIR_BOTTOM_RIGHT, DIR_BOTTOM_LEFT, DIR_NONE, DIR_NONE, DIR_NONE, DIR_NONE, DIR_NONE]),
	// 6: BottomLeft
	(5, [DIR_TOP_LEFT, DIR_BOTTOM_RIGHT, DIR_LEFT, DIR_BOTTOM, DIR_BOTTOM_LEFT, DIR_NONE, DIR_NONE, DIR_NONE]),
	// 7: Left
	(3, [DIR_LEFT, DIR_BOTTOM_LEFT, DIR_TOP_LEFT, DIR_NONE, DIR_NONE, DIR_NONE, DIR_NONE, DIR_NONE]),
	// 8: TopLeft
	(5, [DIR_BOTTOM_LEFT, DIR_TOP_RIGHT, DIR_TOP, DIR_LEFT, DIR_TOP_LEFT, DIR_NONE, DIR_NONE, DIR_NONE]),
];

const ROOM_AREA: usize = 2500;
#[derive(Debug, Clone, Copy)]
struct RoomAndXY {
	room: RoomName,
	xy: RoomXY,
}

impl RoomAndXY {
	fn from_pos(pos: Position) -> Self {
		Self { room: pos.room_name(), xy: pos.xy() }
	}

	fn to_pos(self) -> Position {
		Position::new(self.xy.x, self.xy.y, self.room)
	}

	fn jump_edge(&self) -> Self {
		// If we're on a room edge, jump to the corresponding position in the next room
		let mut new_room = self.room;
		let mut new_xy = self.xy;

		if new_xy.x.u8() == 0 {
			let room_str = new_room.to_string();
			let (h, x, v, y) = parse_room_name(&room_str);
			let new_x = if h == 'W' { x + 1 } else { x - 1 };
			if let Ok(next_room) = RoomName::new(&format!("{}{}{}{}",
				if new_x < 0 { 'W' } else { 'E' },
				new_x.abs(),
				v,
				y
			)) {
				new_room = next_room;
				if let Ok(coord) = RoomCoordinate::new(49) {
					new_xy = RoomXY::new(coord, new_xy.y);
				}
			}
		} else if new_xy.x.u8() == 49 {
			let room_str = new_room.to_string();
			let (h, x, v, y) = parse_room_name(&room_str);
			let new_x = if h == 'W' { x - 1 } else { x + 1 };
			if let Ok(next_room) = RoomName::new(&format!("{}{}{}{}",
				if new_x < 0 { 'W' } else { 'E' },
				new_x.abs(),
				v,
				y
			)) {
				new_room = next_room;
				if let Ok(coord) = RoomCoordinate::new(0) {
					new_xy = RoomXY::new(coord, new_xy.y);
				}
			}
		}

		if new_xy.y.u8() == 0 {
			let room_str = new_room.to_string();
			let (h, x, v, y) = parse_room_name(&room_str);
			let new_y = if v == 'N' { y + 1 } else { y - 1 };
			if let Ok(next_room) = RoomName::new(&format!("{}{}{}{}",
				h,
				x,
				if new_y < 0 { 'N' } else { 'S' },
				new_y.abs()
			)) {
				new_room = next_room;
				if let Ok(coord) = RoomCoordinate::new(49) {
					new_xy = RoomXY::new(new_xy.x, coord);
				}
			}
		} else if new_xy.y.u8() == 49 {
			let room_str = new_room.to_string();
			let (h, x, v, y) = parse_room_name(&room_str);
			let new_y = if v == 'N' { y - 1 } else { y + 1 };
			if let Ok(next_room) = RoomName::new(&format!("{}{}{}{}",
				h,
				x,
				if new_y < 0 { 'N' } else { 'S' },
				new_y.abs()
			)) {
				new_room = next_room;
				if let Ok(coord) = RoomCoordinate::new(0) {
					new_xy = RoomXY::new(new_xy.x, coord);
				}
			}
		}

		Self { room: new_room, xy: new_xy }
	}
}

fn parse_room_name(name: &str) -> (char, i32, char, i32) {
	let mut chars = name.chars();
	let h = chars.next().unwrap();
	let mut x = String::new();
	let mut v = None;
	let mut y = String::new();
	
	for c in chars {
		if c.is_numeric() {
			if v.is_none() {
				x.push(c);
			} else {
				y.push(c);
			}
		} else {
			v = Some(c);
		}
	}
	
	(h, x.parse().unwrap(), v.unwrap(), y.parse().unwrap())
}

impl PartialEq for RoomAndXY {
	fn eq(&self, other: &Self) -> bool {
		self.room == other.room && self.xy == other.xy
	}
}

#[derive(Debug)]
pub struct TileMap<T> {
	pub map: [T; ROOM_AREA],
}

impl<T> TileMap<T> {
	pub fn new(map: [T; ROOM_AREA]) -> Self {
		Self { map }
	}
}

impl IndexMut<RoomXY> for TileMap<u8> {
	fn index_mut(&mut self, index: RoomXY) -> &mut Self::Output {
		let idx = index.y.u8() as usize * 50 + index.x.u8() as usize;
		&mut self.map[idx]
	}
}

impl Index<RoomXY> for TileMap<u8> {
	type Output = u8;

	fn index(&self, index: RoomXY) -> &Self::Output {
		let idx = index.y.u8() as usize * 50 + index.x.u8() as usize;
		&self.map[idx]
	}
}

impl IndexMut<RoomXY> for TileMap<(i32, i32, RoomAndXY)> {
	fn index_mut(&mut self, index: RoomXY) -> &mut Self::Output {
		let idx = index.y.u8() as usize * 50 + index.x.u8() as usize;
		&mut self.map[idx]
	}
}

impl Index<RoomXY> for TileMap<(i32, i32, RoomAndXY)> {
	type Output = (i32, i32, RoomAndXY);

	fn index(&self, index: RoomXY) -> &Self::Output {
		let idx = index.y.u8() as usize * 50 + index.x.u8() as usize;
		&self.map[idx]
	}
}

trait TerrainExt {
	fn is_plain(&self) -> bool;
	fn is_wall(&self) -> bool;
	fn is_swamp(&self) -> bool;
}

impl TerrainExt for Terrain {
	fn is_plain(&self) -> bool {
		matches!(self, Terrain::Plain)
	}

	fn is_wall(&self) -> bool {
		matches!(self, Terrain::Wall)
	}

	fn is_swamp(&self) -> bool {
		matches!(self, Terrain::Swamp)
	}
}

pub fn find_path(source: Position, target: Position, range: u8, mut get_cost_matrix: impl FnMut(RoomName) -> Option<TileMap<u8>>, plain_cost: u8, swamp_cost: u8, max_ops: usize) -> Option<Vec<Position>> {
	#[derive(Debug)]
	struct PathElement {
		pos: RoomAndXY,
		parent: RoomAndXY,
		dir: usize,

		cost: i32,
		steps: i32,
		heuristic: i32,
	}

	impl PathElement {
		fn get_idx(&self, min: i32) -> i32 {
			(self.cost + self.heuristic).max(min)
		}
	}

	if source == target { return Some(vec![]) };	// No movement needed for this.
	if source.in_range_to(target, 1) { return Some(vec![target]); }	// Handles other edge case, does break if target is in a wall but who cares about that, you're not pathing into a wall!

	let mut visited = 0;
	let mut open: Vec<Vec<PathElement>> = vec![Default::default()];
	let mut min_idx = 0usize;

	struct RoomData {
		cost_matrix: Option<TileMap<u8>>,
		// Cost, Steps, Parent
		parents: TileMap<(i32, i32, RoomAndXY)>,
		terrain: LocalRoomTerrain,
	}
	let mut room_data_map: HashMap<RoomName, RoomData> = Default::default();

	let default_position = RoomAndXY::from_pos(Position::from_packed(0));
	let target_room = target.room_name();
	let target_xy = target.xy();

	// Add source location
	open[0].push(PathElement { pos: RoomAndXY::from_pos(source), parent: RoomAndXY::from_pos(source), dir: 0, cost: 0, steps: 0, heuristic: 0 });

	let mut temp_room_data = RoomData {
		cost_matrix: Default::default(),
		parents: TileMap::new([(i32::MAX, 0, default_position); ROOM_AREA]),
		terrain: LocalRoomTerrain::from(screeps::RoomTerrain::new(source.room_name()).unwrap()),
	};

	loop {
		if min_idx >= open.len() { break; }	// Nothing was found.

		let mut room_data_room = RoomName::from_packed(0);
		let mut room_data = &mut temp_room_data;

		open[min_idx].reverse();	// Help find shorter step paths by exploring earlier nodes first (more likely having a lower step count)

		let min_idx_i32 = min_idx as i32;
		while let Some(search) = open[min_idx].pop() {
			visited += 1;
			if visited > max_ops { return None; }	// Too many!

			if search.pos.room == target_room && search.pos.xy.in_range_to(target_xy, range) && (range == 0 || !search.pos.xy.is_room_edge()) {
				let mut path = vec![];
				
				let mut current_pos = search.parent;

				path.push(search.pos.to_pos());
				path.push(current_pos.to_pos());

				loop {
					let Some(room_data) = room_data_map.get(&current_pos.room) else { break; };

					let (_, _, parent) = room_data.parents[current_pos.xy];
					if current_pos == parent {
						break;
					}	// Source pos!
					current_pos = parent;

					path.push(current_pos.to_pos());
				}

				path.pop();	// This is the source location, its not part of the path.

				path.reverse();
				return Some(path);
			}

			if room_data_room != search.pos.room {	// Cache room data from previous iteration so we don't do a hashmap lookup when we don't have to
				room_data_room = search.pos.room;
				room_data = room_data_map.entry(search.pos.room).or_insert_with(|| {
					RoomData {
						cost_matrix: get_cost_matrix(search.pos.room),
						parents: TileMap::new([(i32::MAX, 0, default_position); ROOM_AREA]),
						terrain: LocalRoomTerrain::from(screeps::RoomTerrain::new(search.pos.room).unwrap()),
					}
				});
			}

			let Some(cost_matrix) = &room_data.cost_matrix else { continue; };

			let parent = &mut room_data.parents[search.pos.xy];
			let has_explored = parent.0 != i32::MAX;

			if search.cost < parent.0 || (search.cost == parent.0 && search.steps < parent.1) {
				*parent = (search.cost, search.steps, search.parent);
			}

			if !has_explored {
				let base_heuristic = search.pos.to_pos().get_range_to(target) as i32;

				let (num_directions, directions) = DIRECTION_LOOKUP[search.dir];
				for (dx, dy, new_dir) in directions[0..num_directions].iter() {
					let Some(new_xy) = search.pos.xy.checked_add((*dx, *dy)) else { continue; };

					let new = RoomAndXY {
						room: search.pos.room,
						xy: new_xy,
					};

					let new_heuristic = new.to_pos().get_range_to(target) as i32;
					let heuristic_change = new_heuristic - base_heuristic;
					let mut extra_cost = cost_matrix[new_xy];
					if extra_cost == 0 {
						let terrain = room_data.terrain.get_xy(new_xy);
						if matches!(terrain, Terrain::Plain) {
							extra_cost = plain_cost;
						} else if matches!(terrain, Terrain::Wall) {
							continue;	// No can do
						} else {
							extra_cost = swamp_cost;	// Swamp
						}
					}
					if extra_cost == u8::MAX { continue; }	// Wall!

					let new = PathElement {
						pos: new.jump_edge(),
						parent: search.pos,
						dir: *new_dir,
						cost: search.cost + extra_cost as i32,
						steps: search.steps + 1,
						heuristic: search.heuristic + heuristic_change,
					};

					let new_idx = new.get_idx(min_idx_i32) as usize;

					while open.len() <= new_idx { open.push(Default::default()); };
					open[new_idx].push(new);
				}
			}
		}

		min_idx += 1;	// We have completed all tiles with this score
	}

	None
}