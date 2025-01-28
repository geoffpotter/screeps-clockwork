import { js_astar_multiroom_distance_map, js_astar_multiroom_distance_map2, js_astar_multiroom_distance_map3, js_astar_path_heap, js_astar_path_numeric, js_astar_path_standard } from '../wasm/screeps_clockwork';
import { fromPacked, fromPackedRoomName } from '../utils/fromPacked';
import { CustomCostMatrix, js_astar_path } from '../wasm/screeps_clockwork';
import { ClockworkCostMatrix, Path } from '../wasm/screeps_clockwork';
import { ClockworkPath } from './path';
import { getTerrainCostMatrix } from './getTerrainCostMatrix';

export type AstarPathOptions = {
  maxOps?: number;
  maxPathLength?: number;
  costMatrixCallback?: (roomName: string) => CustomCostMatrix | undefined;
};

export function astar_path(
  origin: RoomPosition,
  goal: RoomPosition,
  options: AstarPathOptions = {}
): ClockworkPath | null {
  const {
    maxOps = 50000,
    maxPathLength = 1000,
    costMatrixCallback = () => {
        console.log('Default cost matrix callback called');
        return new CustomCostMatrix();
    }
  } = options;

  let start_cpu = Game.cpu.getUsed();
  const result = js_astar_path_heap(
    origin.__packedPos,
    goal.__packedPos,
    (roomName: number) => costMatrixCallback(fromPackedRoomName(roomName)),
    maxOps,
    maxPathLength
  );
  let end_cpu = Game.cpu.getUsed();
  console.log(`js: A* Path (Heap) Cpu time: ${end_cpu - start_cpu}`);

  start_cpu = Game.cpu.getUsed();
  const result2 = js_astar_path(
    origin.__packedPos,
    goal.__packedPos,
    (roomName: number) => costMatrixCallback(fromPackedRoomName(roomName)),
    maxOps,
    maxPathLength
  );
  end_cpu = Game.cpu.getUsed();
  console.log(`js: A* Path (No Heap) Cpu time: ${end_cpu - start_cpu}`);

  start_cpu = Game.cpu.getUsed();
  const result3 = js_astar_path_numeric(
    origin.__packedPos,
    goal.__packedPos,
    (roomName: number) => costMatrixCallback(fromPackedRoomName(roomName)),
    maxOps,
    maxPathLength
  );
  end_cpu = Game.cpu.getUsed();
  console.log(`js: A* Path (Numeric) Cpu time: ${end_cpu - start_cpu}`);

  start_cpu = Game.cpu.getUsed();
  const result4 = js_astar_path_standard(
    origin.__packedPos,
    goal.__packedPos,
    (roomName: number) => costMatrixCallback(fromPackedRoomName(roomName)),
    maxOps,
    maxPathLength
  );
  end_cpu = Game.cpu.getUsed();
  console.log(`js: A* Path (Standard) Cpu time: ${end_cpu - start_cpu}`);

  const startPacked = new Uint32Array([origin.__packedPos]);
  const goalPacked = new Uint32Array([goal.__packedPos]);
  start_cpu = Game.cpu.getUsed();
  // const result5 = js_astar_multiroom_distance_map(
  //   startPacked,
  //   (roomName: number) => getTerrainCostMatrix(fromPackedRoomName(roomName)),
  //   maxOps,
  //   maxPathLength,
  //   goalPacked
  // );
  end_cpu = Game.cpu.getUsed();
  console.log(`js: Distance Map Cpu time: ${end_cpu - start_cpu}`);

  start_cpu = Game.cpu.getUsed();
  const result6 = js_astar_multiroom_distance_map2(
    startPacked,
    (roomName: number) => getTerrainCostMatrix(fromPackedRoomName(roomName)),
    maxOps,
    maxPathLength,
    goalPacked
  );
  end_cpu = Game.cpu.getUsed();
  console.log(`js: Distance Map 2 Cpu time: ${end_cpu - start_cpu}`);


  start_cpu = Game.cpu.getUsed();
  const result7 = js_astar_multiroom_distance_map3(
    startPacked,
    (roomName: number) => getTerrainCostMatrix(fromPackedRoomName(roomName)),
    maxOps,
    maxPathLength,
    goalPacked
  );
  end_cpu = Game.cpu.getUsed();
  console.log(`js: Distance Map 3 Cpu time: ${end_cpu - start_cpu}`);

  if (!result) {
    return null;
  }
  return new ClockworkPath(result);
} 