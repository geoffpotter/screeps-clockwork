import { registerBenchmark } from '../helpers/benchmark';
import { 
  astarMultiroomDistanceMap, 
  dijkstraMultiroomDistanceMap,
  bfsMultiroomDistanceMap,
  getTerrainCostMatrix, 
  ephemeral,
  getRange 
} from '../../../src/index';
import { getPositionsInRoom } from '../helpers/positions';
import { getPathCost, pathIsValid } from '../helpers/paths';
import { referenceGetRange } from '../referenceAlgorithms/getRange';
import { referenceDijkstraDistanceMap } from '../referenceAlgorithms/dijkstraDistanceMap';

interface PathfindingInput {
  from: RoomPosition;
  to: RoomPosition;
  maxOps?: number;
}

interface PathfindingResult {
  path: RoomPosition[];
  pathCost: number;
  pathLength: number;
  incomplete: boolean;
  ops: number;
  cpu: number;
}

interface RangeInput {
  positions: RoomPosition[];
  target: RoomPosition;
}

interface RangeResult {
  ranges: number[];
  averageRange: number;
  totalCalculations: number;
  cpu: number;
}

// Helper function to get room coordinates
function getRoomCoords(roomName: string): { x: number; y: number } {
  const match = roomName.match(/^([WE])([0-9]+)([NS])([0-9]+)$/);
  if (!match) throw new Error('Invalid room name');
  const [, h, wx, v, wy] = match;
  return {
    x: h === 'W' ? -Number(wx) - 1 : Number(wx),
    y: v === 'N' ? -Number(wy) - 1 : Number(wy)
  };
}

// Helper function to get room at distance
function getRoomAtDistance(baseRoom: string, distance: number): string {
  const base = getRoomCoords(baseRoom);
  // Instead of diagonal, let's go in the negative X direction for more predictable room selection
  const targetX = base.x - distance;
  const targetY = base.y;
  
  const xChar = targetX < 0 ? 'W' : 'E';
  const yChar = targetY < 0 ? 'N' : 'S';
  const xNum = targetX < 0 ? Math.abs(targetX) - 1 : targetX;
  const yNum = targetY < 0 ? Math.abs(targetY) - 1 : targetY;
  
  return `${xChar}${xNum}${yChar}${yNum}`;
}

// Number of test values generated per benchmark scenario. Adjust this to change sample size.
const NUM_TEST_VALUES = 500;
// Number of iterations each benchmark should run
const NUM_ITERATIONS = 100;

// CPU limits for the benchmarks
const CPU_LIMIT_PATHFINDING = 0.9;
const CPU_LIMIT_RANGE = 0.9;

// Pathfinding benchmarks: All algorithms comparison
registerBenchmark<PathfindingResult, PathfindingInput>('Pathfinding: All Algorithms Comparison', (mark) => {
  
  mark
    .reference('screeps-pathfinder', (input: PathfindingInput) => {
      const startCpu = Game.cpu.getUsed();
      try {
        const result = PathFinder.search(input.from, { pos: input.to, range: 0 }, {
          maxCost: 2000, // Increased for long-distance pathfinding
          maxOps: input.maxOps || 4000,
          roomCallback: (roomName: string) => {
            const terrain = new Room.Terrain(roomName);
            const matrix = new PathFinder.CostMatrix();
            for (let x = 0; x < 50; x++) {
              for (let y = 0; y < 50; y++) {
                const terrainType = terrain.get(x, y);
                if (terrainType === TERRAIN_MASK_WALL) {
                  matrix.set(x, y, 255);
                } else if (terrainType === TERRAIN_MASK_SWAMP) {
                  matrix.set(x, y, 5);
                } else {
                  matrix.set(x, y, 1);
                }
              }
            }
            return matrix;
          }
        });
        
        const cpu = Game.cpu.getUsed() - startCpu;
        return {
          path: result.path,
          pathCost: result.path.length > 0 ? getPathCost(result.path) : Infinity,
          pathLength: result.path.length,
          incomplete: result.incomplete,
          ops: result.ops,
          cpu: cpu
        };
      } catch (error) {
        return {
          path: [],
          pathCost: Infinity,
          pathLength: 0,
          incomplete: true,
          ops: 0,
          cpu: Game.cpu.getUsed() - startCpu
        };
      }
    })
    .implement('clockwork-astar', (input: PathfindingInput) => {
      const startCpu = Game.cpu.getUsed();
      const actualMaxOps = input.maxOps || 4000;
      try {
        const distanceMap = ephemeral(astarMultiroomDistanceMap([input.from], {
          costMatrixCallback: (roomName: string) => ephemeral(getTerrainCostMatrix(roomName)),
          maxRooms: 16,
          maxOps: actualMaxOps,
          anyOfDestinations: [input.to]
        }));
        const path = distanceMap.pathToOrigin(input.to);
        const fullPathArray = path.toArray();
        // Don't slice if we need to preserve the path - only slice if path starts at origin
        const pathArray = fullPathArray.length > 1 ? fullPathArray.slice(1) : fullPathArray;
        const cpu = Game.cpu.getUsed() - startCpu;
        return {
          path: pathArray,
          pathCost: pathArray.length > 0 ? getPathCost(pathArray) : Infinity,
          pathLength: pathArray.length,
          incomplete: pathArray.length === 0,
          ops: pathArray.length, // Approximate
          cpu: cpu
        };
      } catch (error) {
        // console.log(`DEBUG: Clockwork A* error:`, error);
        return {
          path: [],
          pathCost: Infinity,
          pathLength: 0,
          incomplete: true,
          ops: 0,
          cpu: Game.cpu.getUsed() - startCpu
        };
      }
    })
    .implement('clockwork-dijkstra', (input: PathfindingInput) => {
      const startCpu = Game.cpu.getUsed();
      try {
        const distanceMap = ephemeral(dijkstraMultiroomDistanceMap([input.from], {
          costMatrixCallback: (roomName: string) => ephemeral(getTerrainCostMatrix(roomName)),
          maxRooms: 128, // Higher limit for exhaustive algorithm
          maxOps: input.maxOps || 10000,
          anyOfDestinations: [input.to]
        }));
        const path = distanceMap.pathToOrigin(input.to);
        const fullPathArray = path.toArray();
        const pathArray = fullPathArray.length > 1 ? fullPathArray.slice(1) : fullPathArray;
        const cpu = Game.cpu.getUsed() - startCpu;
        return {
          path: pathArray,
          pathCost: pathArray.length > 0 ? getPathCost(pathArray) : Infinity,
          pathLength: pathArray.length,
          incomplete: pathArray.length === 0,
          ops: pathArray.length, // Approximate
          cpu: cpu
        };
      } catch (error) {
        // console.log(`DEBUG: Clockwork Dijkstra error:`, error);
        return {
          path: [],
          pathCost: Infinity,
          pathLength: 0,
          incomplete: true,
          ops: 0,
          cpu: Game.cpu.getUsed() - startCpu
        };
      }
    })
    .implement('clockwork-bfs', (input: PathfindingInput) => {
      const startCpu = Game.cpu.getUsed();
      try {
        const distanceMap = ephemeral(bfsMultiroomDistanceMap([input.from], {
          costMatrixCallback: (roomName: string) => ephemeral(getTerrainCostMatrix(roomName)),
          maxRooms: 128, // Higher limit for exhaustive algorithm  
          maxOps: input.maxOps || 10000,
          anyOfDestinations: [input.to]
        }));
        const path = distanceMap.pathToOrigin(input.to);
        const fullPathArray = path.toArray();
        const pathArray = fullPathArray.length > 1 ? fullPathArray.slice(1) : fullPathArray;
        const cpu = Game.cpu.getUsed() - startCpu;
        return {
          path: pathArray,
          pathCost: pathArray.length > 0 ? getPathCost(pathArray) : Infinity,
          pathLength: pathArray.length,
          incomplete: pathArray.length === 0,
          ops: pathArray.length, // Approximate
          cpu: cpu
        };
      } catch (error) {
        // console.log(`DEBUG: Clockwork BFS error:`, error);
        return {
          path: [],
          pathCost: Infinity,
          pathLength: 0,
          incomplete: true,
          ops: 0,
          cpu: Game.cpu.getUsed() - startCpu
        };
      }
    })
    .implement('clockwork-astar-low-ops', (input: PathfindingInput) => {
      const startCpu = Game.cpu.getUsed();
      const actualMaxOps = Math.min(input.maxOps || 4000, 2000);
      try {
        const distanceMap = ephemeral(astarMultiroomDistanceMap([input.from], {
          costMatrixCallback: (roomName: string) => ephemeral(getTerrainCostMatrix(roomName)),
          maxRooms: 16,
          maxOps: actualMaxOps, // Low ops limit but scale with input
          anyOfDestinations: [input.to]
        }));
        const path = distanceMap.pathToOrigin(input.to);
        const fullPathArray = path.toArray();
        const pathArray = fullPathArray.length > 1 ? fullPathArray.slice(1) : fullPathArray;
        const cpu = Game.cpu.getUsed() - startCpu;
        return {
          path: pathArray,
          pathCost: pathArray.length > 0 ? getPathCost(pathArray) : Infinity,
          pathLength: pathArray.length,
          incomplete: pathArray.length === 0,
          ops: pathArray.length, // Approximate
          cpu: cpu
        };
      } catch (error) {
        return {
          path: [],
          pathCost: Infinity,
          pathLength: 0,
          incomplete: true,
          ops: 0,
          cpu: Game.cpu.getUsed() - startCpu
        };
      }
    })
    .implement('clockwork-dijkstra-low-ops', (input: PathfindingInput) => {
      const startCpu = Game.cpu.getUsed();
      try {
        const distanceMap = ephemeral(dijkstraMultiroomDistanceMap([input.from], {
          costMatrixCallback: (roomName: string) => ephemeral(getTerrainCostMatrix(roomName)),
          maxRooms: 128, // Higher limit for exhaustive algorithm
          maxOps: input.maxOps || 5000, 
          anyOfDestinations: [input.to]
        }));
        const path = distanceMap.pathToOrigin(input.to);
        const fullPathArray = path.toArray();
        const pathArray = fullPathArray.length > 1 ? fullPathArray.slice(1) : fullPathArray;
        const cpu = Game.cpu.getUsed() - startCpu;
        return {
          path: pathArray,
          pathCost: pathArray.length > 0 ? getPathCost(pathArray) : Infinity,
          pathLength: pathArray.length,
          incomplete: pathArray.length === 0,
          ops: pathArray.length, // Approximate
          cpu: cpu
        };
      } catch (error) {
        return {
          path: [],
          pathCost: Infinity,
          pathLength: 0,
          incomplete: true,
          ops: 0,
          cpu: Game.cpu.getUsed() - startCpu
        };
      }
    })
    .testCase('same-room', () => {
      const room = 'W0N0';
      const positions = getPositionsInRoom(room, NUM_TEST_VALUES * 3, 'walkable'); // Fetch plenty of positions
      const inputs: PathfindingInput[] = [];
      
      for (let i = 0; i < positions.length - 1; i += 2) {
        const from = positions[i];
        const to = positions[i + 1];
        inputs.push({ from, to, maxOps: 10000 });
        if (inputs.length >= NUM_TEST_VALUES) break;
      }
      return inputs;
    })
    .testCase('adjacent-rooms', () => {
      const baseRoom = 'W0N0';
      let targetRoom = getRoomAtDistance(baseRoom, 1);
      console.log(`DEBUG: adjacent-rooms test case - base: ${baseRoom}, target: ${targetRoom}`);
      let fromPositions = getPositionsInRoom(baseRoom, NUM_TEST_VALUES * 3, 'walkable'); // Increased to ensure enough positions
      let toPositions = getPositionsInRoom(targetRoom, NUM_TEST_VALUES * 3, 'walkable'); // Increased to ensure enough positions
      console.log(`DEBUG: Found ${fromPositions.length} from positions and ${toPositions.length} to positions`);
      
      const inputs: PathfindingInput[] = [];
      for (let i = 0; i < Math.min(fromPositions.length, toPositions.length, NUM_TEST_VALUES); i++) {
        inputs.push({ from: fromPositions[i], to: toPositions[i], maxOps: 15000 });
      }
      return inputs;
    })
    .testCase('2-rooms-away', () => {
      const baseRoom = 'W0N0';
      const targetRoom = getRoomAtDistance(baseRoom, 2);
      const fromPositions = getPositionsInRoom(baseRoom, NUM_TEST_VALUES * 3, 'walkable'); // Increased
      const toPositions = getPositionsInRoom(targetRoom, NUM_TEST_VALUES * 3, 'walkable'); // Increased
      const inputs: PathfindingInput[] = [];
      
      for (let i = 0; i < Math.min(fromPositions.length, toPositions.length, 100); i++) { // Increased from 5 to 100
        inputs.push({ from: fromPositions[i], to: toPositions[i], maxOps: 4000 });
      }
      return inputs;
    })
    .testCase('5-rooms-away', () => {
      const baseRoom = 'W0N0';
      const targetRoom = getRoomAtDistance(baseRoom, 5);
      const fromPositions = getPositionsInRoom(baseRoom, NUM_TEST_VALUES * 3, 'walkable'); // Increased
      const toPositions = getPositionsInRoom(targetRoom, NUM_TEST_VALUES * 3, 'walkable'); // Increased
      const inputs: PathfindingInput[] = [];
      
      for (let i = 0; i < Math.min(fromPositions.length, toPositions.length, 100); i++) { // Increased from 10 to 100
        inputs.push({ from: fromPositions[i], to: toPositions[i], maxOps: 12000 });
      }
      return inputs;
    })
    .testCase('10-rooms-away', () => {
      const baseRoom = 'W0N0';
      const targetRoom = getRoomAtDistance(baseRoom, 10);
      const fromPositions = getPositionsInRoom(baseRoom, NUM_TEST_VALUES * 3, 'walkable'); // Increased
      const toPositions = getPositionsInRoom(targetRoom, NUM_TEST_VALUES * 3, 'walkable'); // Increased
      const inputs: PathfindingInput[] = [];
      
      for (let i = 0; i < Math.min(fromPositions.length, toPositions.length, 100); i++) { // Increased from 8 to 100
        inputs.push({ from: fromPositions[i], to: toPositions[i], maxOps: 20000 });
      }
      return inputs;
    })
    .validate((result, input, _reference) => {
      if (!result) {
        return 'No result returned for ' + input.from + " -> " + input.to;
      }
      
      if (result.pathLength === 0) {
        return "Path not found for " + input.from + " -> " + input.to;
      }
      
      const startGoal = { pos: input.from, range: 1 };
      const endGoals = [{ pos: input.to, range: 0 }];
      
      try {
        if (!pathIsValid(result.path, [startGoal], endGoals)) {
          return 'Path is not valid (start/end positions) for ' + input.from + " -> " + input.to;
        }
      } catch (error) {
        return `Path validation error: ${error} for ${input.from} -> ${input.to}`;
      }
      
      return true;
    })
    .minimize('path-cost', (result) => result.pathCost === Infinity ? 1000 : result.pathCost)
    .minimize('path-length', (result) => result.pathLength)
    .iterations(NUM_ITERATIONS) // Adjusted to variable
    // .warmup(2) // Added warmup (corrected method name)
    .cpuLimit(CPU_LIMIT_PATHFINDING);
});

// Range calculation benchmarks: Clockwork vs Screeps
registerBenchmark<RangeResult, RangeInput>('Range Calculations: Clockwork vs Screeps', (mark) => {
  
  mark
    .reference('reference-get-range', (input: RangeInput) => {
      const startCpu = Game.cpu.getUsed();
      const ranges: number[] = [];
      for (const pos of input.positions) {
        const range = referenceGetRange(pos, input.target);
        ranges.push(range);
      }
      const cpu = Game.cpu.getUsed() - startCpu;
      const averageRange = ranges.reduce((sum, r) => sum + r, 0) / ranges.length;
      return {
        ranges,
        averageRange,
        totalCalculations: ranges.length,
        cpu
      };
    })
    .implement('clockwork-get-range', (input: RangeInput) => {
      const startCpu = Game.cpu.getUsed();
      const ranges: number[] = [];
      for (const pos of input.positions) {
        const range = getRange(pos, input.target);
        ranges.push(range);
      }
      const cpu = Game.cpu.getUsed() - startCpu;
      const averageRange = ranges.reduce((sum, r) => sum + r, 0) / ranges.length;
      return {
        ranges,
        averageRange,
        totalCalculations: ranges.length,
        cpu
      };
    })
    .testCase('same-room-positions', () => {
      const room = 'W0N0';
      const positions = getPositionsInRoom(room, NUM_TEST_VALUES, 'walkable');
      const target = positions[positions.length - 1];
      let cases = positions.map(pos => ({ positions: [pos], target }));
      return cases;
    })
    .testCase('cross-room-positions', () => {
      const positions1 = getPositionsInRoom('W0N0', NUM_TEST_VALUES, 'walkable');
      const positions2 = getPositionsInRoom('W0N1', 20, 'walkable');
      if (positions2.length > 0) {
        let cases = positions1.slice(0, NUM_TEST_VALUES).map(pos => ({ positions: [pos], target: positions2[0] }));
        return cases;
      }
      return [];
    })
    .validate((result, input, reference) => {
      if (!result || result.ranges.length === 0) {
        return 'No range calculations performed';
      }
      if (result.ranges.length !== input.positions.length) {
        return `Range count mismatch: ${result.ranges.length} vs ${input.positions.length}`;
      }
      if (reference) {
        for (let i = 0; i < result.ranges.length; i++) {
          if (result.ranges[i] !== reference.ranges[i]) {
            return `Range mismatch at index ${i}: ${result.ranges[i]} vs ${reference.ranges[i]}`;
          }
        }
      }
      return true;
    })
    .match('average-range', (result) => result.averageRange || Infinity)
    .iterations(NUM_ITERATIONS)
    .cpuLimit(CPU_LIMIT_RANGE);
});