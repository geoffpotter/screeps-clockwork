import { fromPacked } from '../utils/fromPacked';
import { js_pathfinder } from '../wasm/screeps_clockwork';

export class RustPathFinder {

  constructor(
    plainCost: number,
    swampCost: number,
    maxRooms: number,
    maxOps: number,
    maxCost: number,
    flee: boolean,
    heuristicWeight: number
  ) {
  }

  setDebug(debug: boolean): void {
  }

  search(
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
} 
