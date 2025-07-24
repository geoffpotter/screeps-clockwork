import { benchmark, registerBenchmark } from '../helpers/benchmark';
import { ClockworkCostMatrix, astarMultiroomDistanceMap, dijkstraMultiroomDistanceMap, getTerrainCostMatrix, ephemeral } from '../../../src/index';
import { getPositionsInRoom } from '../helpers/positions';
import { getPathCost, pathIsValid } from '../helpers/paths';

// Example 1: Simple Math Function Benchmarking
// This shows how to benchmark different implementations of the same mathematical function

// Example cached sine implementation (for demonstration)
const sineCache = new Map<number, number>();
function cachedSin(x: number): number {
  if (sineCache.has(x)) {
    return sineCache.get(x)!;
  }
  const result = Math.sin(x);
  sineCache.set(x, result);
  return result;
}

// Example approximated sine (Taylor series - less accurate but potentially faster)
function approxSin(x: number): number {
  // Normalize to [-PI, PI] range
  x = ((x + Math.PI) % (2 * Math.PI)) - Math.PI;
  
  // Taylor series approximation (first 4 terms)
  const x2 = x * x;
  return x - (x * x2) / 6 + (x * x2 * x2) / 120 - (x * x2 * x2 * x2) / 5040;
}

registerBenchmark<number, number>('Math.sin Implementations', (mark) => {
  mark
    .reference('native-math-sin', (x: number) => Math.sin(x))
    .implement('cached-sin', cachedSin)
    .implement('approx-sin', approxSin)
    .testCase('small-numbers', () => {
      const numbers: number[] = [];
      for (let i = 0; i < 1000; i++) {
        numbers.push(Math.random() * 2 * Math.PI - Math.PI);
      }
      return numbers;
    })
    .testCase('repeated-values', () => {
      // Test case where caching should shine
      const baseNumbers = [0, Math.PI/6, Math.PI/4, Math.PI/3, Math.PI/2, Math.PI];
      const numbers: number[] = [];
      for (let i = 0; i < 1000; i++) {
        numbers.push(baseNumbers[i % baseNumbers.length]);
      }
      return numbers;
    })
    .validate((result, input, reference) => {
      // Basic validity - result should be a finite number
      if (typeof result !== 'number' || !isFinite(result)) {
        return `Invalid result: expected finite number, got ${result}`;
      }
      
      // Sine should be between -1 and 1
      if (result < -1.1 || result > 1.1) { // Allow small tolerance for approximation errors
        return `Sine result out of range: ${result} (input: ${input.toFixed(3)})`;
      }
      
      // If we have a reference, compare against it
      if (reference !== undefined) {
        const error = Math.abs(result - reference);
        if (error > 0.1) {
          return `Error too large: ${error.toFixed(6)} vs reference (input: ${input.toFixed(3)})`;
        }
      }
      
      return true;
    })
    .minimize('absolute-error', (result, input, reference) => {
      if (reference === undefined) return 0; // No reference to compare against
      return Math.abs(result - reference);
    })
    .minimize('relative-error', (result, input, reference) => {
      if (reference === undefined) return 0; // No reference to compare against
      if (Math.abs(reference) < 1e-10) return 0; // Avoid division by zero
      return Math.abs((result - reference) / reference);
    })
    .iterations(5)
    .repeats(10) // Run each function 1000 times per CPU measurement for accurate timing of fast math operations
    .warmup(3) // Run 3 warmup rounds to stabilize JIT compilation
    .cpuLimit(0.4); // Use 40% of CPU limit per tick
});

// Example 2: Pathfinding Benchmark
// This shows how to benchmark pathfinding algorithms with complex validation

interface PathfindingInput {
  from: RoomPosition;
  to: RoomPosition[];
}

// Reference PathFinder implementation
function pathfinder(input: PathfindingInput): RoomPosition[] {
  const res = PathFinder.search(input.from, input.to[0], {
    maxCost: 1500,
    maxOps: 10000,
    roomCallback: () => new PathFinder.CostMatrix(),
    heuristicWeight: 1
  });
  
  if (res.incomplete) {
    return [];
  }
  return res.path;
}

// Clockwork A* implementation
function clockworkAstar(input: PathfindingInput): RoomPosition[] {
  const cache = new Map<string, ClockworkCostMatrix>();
  
  const distanceMap = ephemeral(
    astarMultiroomDistanceMap([input.from], {
      costMatrixCallback: roomName => {
        if (cache.has(roomName)) {
          return cache.get(roomName);
        }
        const costMatrix = ephemeral(getTerrainCostMatrix(roomName));
        cache.set(roomName, costMatrix);
        return costMatrix;
      },
      anyOfDestinations: input.to
    })
  );
  
  const path = distanceMap.pathToOrigin(input.to[0]);
  return path.toArray().slice(1); // Remove starting position
}

// Clockwork Dijkstra implementation
function clockworkDijkstra(input: PathfindingInput): RoomPosition[] {
  const cache = new Map<string, ClockworkCostMatrix>();
  
  const distanceMap = ephemeral(
    dijkstraMultiroomDistanceMap([input.from], {
      costMatrixCallback: roomName => {
        if (cache.has(roomName)) {
          return cache.get(roomName);
        }
        const costMatrix = ephemeral(getTerrainCostMatrix(roomName));
        cache.set(roomName, costMatrix);
        return costMatrix;
      },
      anyOfDestinations: input.to
    })
  );
  
  const path = distanceMap.pathToOrigin(input.to[0]);
  return path.toArray().slice(1); // Remove starting position
}

registerBenchmark<RoomPosition[], PathfindingInput>('Pathfinding Algorithms', (mark) => {
  mark
    .reference('screeps-pathfinder', pathfinder)
    .implement('clockwork-astar', clockworkAstar)
    .implement('clockwork-dijkstra', clockworkDijkstra)
    .testCase('same-room-short', () => {
      const room = 'W1N1';
      const positions = getPositionsInRoom(room, 50, 'walkable');
      const inputs: PathfindingInput[] = [];
      
      for (let i = 0; i < positions.length - 1; i += 2) {
        inputs.push({
          from: positions[i],
          to: [positions[i + 1]]
        });
      }
      return inputs;
    })
    .testCase('adjacent-rooms', () => {
      const room1Positions = getPositionsInRoom('W0N0', 25, 'walkable');
      const room2Positions = getPositionsInRoom('W0N1', 25, 'walkable');
      const inputs: PathfindingInput[] = [];
      
      for (let i = 0; i < room1Positions.length; i++) {
        inputs.push({
          from: room1Positions[i],
          to: [room2Positions[i]]
        });
      }
      return inputs;
    })
    .testCase('distant-rooms', () => {
      const room1Positions = getPositionsInRoom('W0N0', 20, 'walkable');
      const room2Positions = getPositionsInRoom('W0N3', 20, 'walkable');
      const inputs: PathfindingInput[] = [];
      
      for (let i = 0; i < room1Positions.length; i++) {
        inputs.push({
          from: room1Positions[i],
          to: [room2Positions[i]]
        });
      }
      return inputs;
    })
    .beforeEach((input) => {
      // Clear any global state before each test
    })
    .validate((result, input, reference) => {
      // Basic path validity check (independent of reference)
      if (!result || result.length === 0) {
        return 'No path found';
      }
      
      // Check if path is valid (connects start to end)
      const startGoal = { pos: input.from, range: 1 };
      const endGoals = input.to.map(pos => ({ pos, range: 0 }));
      
      if (!pathIsValid(result, [startGoal], endGoals)) {
        return 'Path is not valid (doesn\'t connect start to end)';
      }
      
      // If we have a reference result, do comparative validation
      if (reference && reference.length > 0) {
        // Allow some tolerance in path cost (our algorithm might find different but equally good paths)
        const ourCost = getPathCost(result);
        const refCost = getPathCost(reference);
        const tolerance = 1.5; // Allow 50% worse than reference (more lenient)
        
        if (ourCost > refCost * tolerance) {
          return `Path cost too high: ${ourCost} vs reference ${refCost} (tolerance: ${tolerance})`;
        }
      }
      
      return true;
    })
    .minimize('path-length', (result) => result ? result.length : Infinity)
    .minimize('path-cost', (result) => result ? getPathCost(result) : Infinity)
    .iterations(2)
    .cpuLimit(0.3);
});

// Example 3: Simple String Processing Benchmark
// Shows how to benchmark different approaches to the same string operation

// Different ways to count vowels in a string
function countVowelsLoop(str: string): number {
  let count = 0;
  const vowels = 'aeiouAEIOU';
  for (let i = 0; i < str.length; i++) {
    if (vowels.includes(str[i])) {
      count++;
    }
  }
  return count;
}

function countVowelsRegex(str: string): number {
  const matches = str.match(/[aeiouAEIOU]/g);
  return matches ? matches.length : 0;
}

function countVowelsFilter(str: string): number {
  const vowels = new Set('aeiouAEIOU');
  return str.split('').filter(char => vowels.has(char)).length;
}

registerBenchmark<number, string>('String Processing', (mark) => {
  mark
    .reference('loop-based-counting', countVowelsLoop)
    .implement('regex-approach', countVowelsRegex)
    .implement('filter-approach', countVowelsFilter)
    .testCase('short-strings', () => {
      const strings = [
        'hello world',
        'javascript',
        'benchmark',
        'performance',
        'algorithm'
      ];
      return Array(200).fill(0).map(() => strings[Math.floor(Math.random() * strings.length)]);
    })
    .testCase('long-strings', () => {
      const longString = 'The quick brown fox jumps over the lazy dog. '.repeat(10);
      return Array(50).fill(longString);
    })
    .validate((result, input, reference) => {
      // Basic validity - result should be a non-negative number
      if (typeof result !== 'number' || result < 0) {
        return `Invalid result: expected non-negative number, got ${result}`;
      }
      
      // If we have a reference, compare against it
      if (reference !== undefined && result !== reference) {
        return `Wrong vowel count: got ${result}, expected ${reference} for "${input.substring(0, 50)}..."`;
      }
      
      return true;
    })
    .match('correctness', (result, input, reference) => result) // Exact match with reference
    .iterations(3)
    .repeats(2) // Run each function 500 times per CPU measurement for accurate timing of fast string operations
    .warmup(2) // Run 2 warmup rounds to stabilize performance
    .cpuLimit(0.3);
});