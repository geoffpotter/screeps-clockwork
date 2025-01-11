import { fromPacked } from '../utils/fromPacked';
import { js_jasper_star } from '../wasm/screeps_clockwork';

export type PathfinderResult = {
  readonly path: RoomPosition[];
  readonly ops: number;
  readonly cost: number;
  readonly incomplete: boolean;
};

export function jasper_star(
  origin: RoomPosition,
  goals: RoomPosition[]
): PathfinderResult | null {
  if (!goals?.length) {
    throw new Error('At least one destination must be set');
  }

  const startPacked = origin.__packedPos;
  const destinationsPacked = new Uint32Array(goals.map(pos => pos.__packedPos));
  const result = js_jasper_star(startPacked, destinationsPacked, 0, 1, 5, 10000);

  if (!result.length) {
    return null;
  }

  // Unpack metadata from start of array: [ops, cost, incomplete, ...path]
  const [ops, cost, incomplete, ...pathPacked] = result;

  return {
    path: pathPacked.map(pos => fromPacked(pos)),
    ops,
    cost,
    incomplete: incomplete === 1
  };
}
