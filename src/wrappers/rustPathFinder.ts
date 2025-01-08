import { fromPacked } from '../utils/fromPacked';
import { js_pathfinder } from '../wasm/screeps_clockwork';



export function rust_pathfinder(
  origin: RoomPosition,
  goals: RoomPosition[]
): RoomPosition[] | null {
  if (!goals?.length) {
    throw new Error('At least one destination must be set');
  }

  const startPacked = origin.__packedPos;
  const destinationsPacked = new Uint32Array(goals.map(pos => pos.__packedPos));
  const result = js_pathfinder(startPacked, destinationsPacked);

  const path = [];
  for (const pos of result) {
    path.push(fromPacked(pos));
  }

  return path;
}
