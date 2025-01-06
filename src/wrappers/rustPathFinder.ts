import { JsPathFinder } from "../wasm/screeps_clockwork";


export class RustPathFinder {
  private pathFinder: JsPathFinder;

  constructor(
    plainCost: number,
    swampCost: number,
    maxRooms: number,
    maxOps: number,
    maxCost: number,
    flee: boolean,
    heuristicWeight: number
  ) {
    this.pathFinder = new JsPathFinder(
      plainCost,
      swampCost,
      maxRooms,
      maxOps,
      maxCost,
      flee,
      heuristicWeight
    );
  }

  setDebug(debug: boolean): void {
    this.pathFinder.set_debug(debug);
  }

  search(
    origin: RoomPosition,
    goals: RoomPosition[],
    roomCallback: (roomName: string) => { terrain: Uint8Array; cost_matrix?: Uint8Array | null } | null
  ): RoomPosition[] | null {
    return this.pathFinder.search(origin, goals, roomCallback) || null;
  }
} 