import {
  astarMultiroomDistanceMap,
  ClockworkCostMatrix,
  getTerrainCostMatrix as clockworkGetTerrainCostMatrix,
  ClockworkPath,
  ephemeral,
  jpsDistanceMap,
  jpsPath,
  rust_pathfinder,
  PathfinderResult,
  jasper_star,
  astar_path
} from '../../src/index';
import { CustomCostMatrix, Path } from '../../src/wasm/screeps_clockwork';

import { cpuTime } from '../utils/cpuTime';
import { FlagVisualizer } from './helpers/FlagVisualizer';
import { visualizeDistanceMap } from './helpers/visualizeDistanceMap';
import { visualizePath } from './helpers/visualizePath';

interface PathResult {
  path: RoomPosition[];
  ops: number;
  cost: number;
  incomplete: boolean;
  cpu: number;
}

interface ComparisonResult {
  clockwork: PathResult;
  pathfinder: PathResult;
}

type CpuComparisonVisualizer = FlagVisualizer & {
  positionQueue: Array<{x: number, y: number}>;
  results: Map<string, ComparisonResult>;
  initialized: boolean;
};

function getTerrainCostMatrix(room: string, { plainCost, swampCost, wallCost }: { plainCost?: number; swampCost?: number; wallCost?: number; } = {}) {
  return ephemeral(clockworkGetTerrainCostMatrix(room, { plainCost, swampCost, wallCost }));
}


let avg_cw_time = 0;
let avg_pf_time = 0;

const cache = new Map<string, ClockworkCostMatrix>();

const cache2 = new Map<string, CustomCostMatrix>();
export default [
  {
    name: 'A* Multiroom Distance Map',
    color1: COLOR_GREEN,
    color2: COLOR_RED,
    /**
     * Visualization of a distance map, where each cell tracks the distance to
     * the nearest flag.
     */
    run(rooms) {
      const [originFlag, ...targetFlags] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || targetFlags.length === 0) {
        return;
      }
      const distanceMap = ephemeral(
        astarMultiroomDistanceMap([originFlag.pos], {
          costMatrixCallback: getTerrainCostMatrix,
          maxOps: 10000,
          allOfDestinations: targetFlags.map(flag => flag.pos)
        })
      );
      for (const room of distanceMap.getRooms()) {
        visualizeDistanceMap(room, distanceMap.getRoom(room)!);
      }
    }
  },
  {
    name: 'A* Multiroom Distance Map Path',
    color1: COLOR_GREEN,
    color2: COLOR_GREEN,
    /**
     * Visualization of a Dijkstra multiroom distance map-based path.
     */
    run(rooms) {
      const [originFlag, targetFlag, ...rest] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || !targetFlag) {
        return;
      }

      const distanceMap = ephemeral(
        astarMultiroomDistanceMap([originFlag.pos], {
          costMatrixCallback: getTerrainCostMatrix,
          maxOps: 10000,
          anyOfDestinations: [targetFlag.pos]
        })
      );

      const path = ephemeral(distanceMap.pathToOrigin(targetFlag.pos));
      const pathArray = path.toArray();
      visualizePath(pathArray);
    }
  },
  {
    name: 'JPS Multiroom Distance Map',
    color1: COLOR_YELLOW,
    color2: COLOR_RED,
    /**
     * Visualization of a distance map, where each cell tracks the distance to
     * the nearest flag.
     */
    run(rooms) {
      const [originFlag, ...targetFlags] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || targetFlags.length === 0) {
        return;
      }
      const distanceMap = ephemeral(
        jpsDistanceMap(
          [originFlag.pos],
          targetFlags.map(flag => flag.pos),
          {
            costMatrixCallback: getTerrainCostMatrix,
            maxOps: 10000
          }
        )
      );
      for (const room of distanceMap.getRooms()) {
        visualizeDistanceMap(room, distanceMap.getRoom(room)!);
      }
    }
  },
  {
    name: 'JPS Multiroom Distance Map Path',
    color1: COLOR_YELLOW,
    color2: COLOR_GREEN,
    /**
     * Visualization of a Dijkstra multiroom distance map-based path.
     */
    run(rooms) {
      const [originFlag, targetFlag, ...rest] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || !targetFlag) {
        return;
      }

      const distanceMap = ephemeral(
        jpsDistanceMap([originFlag.pos], [targetFlag.pos], {
          costMatrixCallback: getTerrainCostMatrix,
          maxOps: 10000
        })
      );

      const path = ephemeral(distanceMap.pathToOrigin(targetFlag.pos));
      const pathArray = path.toArray();
      visualizePath(pathArray);
    }
  },
  {
    name: 'JPS Distance Map',
    color1: COLOR_YELLOW,
    color2: COLOR_RED,
    /**
     * Visualization of a JPS path.
     */
    run(rooms) {
      const [originFlag, targetFlag, ...rest] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || !targetFlag) {
        return;
      }

      const distanceMap = ephemeral(
        jpsDistanceMap([originFlag.pos], [targetFlag.pos], {
          costMatrixCallback: getTerrainCostMatrix,
          maxOps: 10000
        })
      );
      for (const room of distanceMap.getRooms()) {
        visualizeDistanceMap(room, distanceMap.getRoom(room)!);
      }
    }
  },
  {
    name: 'JPS Path',
    color1: COLOR_YELLOW,
    color2: COLOR_GREEN,
    /**
     * Visualization of a JPS path.
     */
    run(rooms) {
      const [originFlag, targetFlag, ...rest] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || !targetFlag) {
        return;
      }
      let from = originFlag.pos;
      let to = targetFlag.pos;
      const iterations = 1;

      let pathFinderPath: PathFinderPath;
      const visitedRooms = new Set<string>();
      const pathFinderTime = cpuTime(() => {
        pathFinderPath = PathFinder.search(to, {pos: from, range: 0}, {
          maxCost: 1500,
          maxOps: 10000,
          roomCallback: roomName => {
            visitedRooms.add(roomName);
            return new PathFinder.CostMatrix();
          },
          heuristicWeight: 1
        });
      }, iterations);
  

      visualizePath(pathFinderPath!.path, "red");
      cache.clear();
      let clockworkPath: ClockworkPath;
      
      const clockworkTime = cpuTime(() => {
        clockworkPath = ephemeral(
          jpsPath([from], [to], {
            maxOps: Game.time % 20 + 10000,
            
            // maxOps: 10000,
            costMatrixCallback: roomName => {
              // let startCpu = Game.cpu.getUsed();
              if (Game.map.getRoomStatus(roomName).status != "normal") {
                console.log('Room not normal', roomName);
                return;
              }
              if (cache.has(roomName)) {
                // console.log('Cache hit', roomName);
                // let endCpu = Game.cpu.getUsed();
                // console.log('CM Cpu time', endCpu - startCpu);
                return cache.get(roomName);
              }
              // console.log('Cache miss', roomName);
              const costMatrix = ephemeral(getTerrainCostMatrix(roomName, { plainCost: 1, swampCost: 5, wallCost: 255 }));
              // let endCpu = Game.cpu.getUsed();
              // console.log('CM Cpu time', endCpu - startCpu);
              cache.set(roomName, costMatrix);
              return costMatrix;
            }
          })
        );
      }, iterations);
  

      let weight = 0.1;
      if (avg_cw_time === 0) {
        avg_cw_time = clockworkTime * 0.5;
      }
      if (avg_pf_time === 0) {
        avg_pf_time = pathFinderTime * 0.5;
      }
      avg_cw_time = (avg_cw_time * (1 - weight)) + (clockworkTime * weight);
      avg_pf_time = (avg_pf_time * (1 - weight)) + (pathFinderTime * weight);
      console.log(`Clockwork Time: ${avg_cw_time.toFixed(2)}, this tick: ${clockworkTime.toFixed(2)}`);
      console.log('Clockwork Path', clockworkPath!.length, "rooms opened", cache.size);
      console.log(`PathFinder Time: ${avg_pf_time.toFixed(2)}, this tick: ${pathFinderTime.toFixed(2)}`);
      console.log('PathFinder Path', pathFinderPath!.path.length, "rooms opened", visitedRooms.size, "ops", pathFinderPath!.ops);


      // const path = ephemeral(
      //   jpsPath([originFlag.pos], [targetFlag.pos], {
      //     costMatrixCallback: getTerrainCostMatrix,
      //     maxOps: 10000
      //   })
      // );
      // const pathArray = path.toArray();
      let pathArray = clockworkPath!.toArray();
      visualizePath(pathArray);
    }
  },
  {
    name: 'A* Path',
    color1: COLOR_YELLOW,
    color2: COLOR_PURPLE,
    run(rooms) {
      const [originFlag, targetFlag, ...rest] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || !targetFlag) {
        return;
      }
      let from = originFlag.pos;
      let to = targetFlag.pos;
      const iterations = 1;

      let pathFinderPath: PathFinderPath;
      const visitedRooms = new Set<string>();
      const pathFinderTime = cpuTime(() => {
        pathFinderPath = PathFinder.search(to, {pos: from, range: 0}, {
          maxCost: 1500,
          maxOps: 10000,
          roomCallback: roomName => {
            visitedRooms.add(roomName);
            return new PathFinder.CostMatrix();
          },
          heuristicWeight: 1
        });
      }, iterations);

      visualizePath(pathFinderPath!.path, "red");
      cache.clear();
      let temp_astarPath: ClockworkPath | null = null;
      
      const astarTime = cpuTime(() => {
        temp_astarPath =
          astar_path(from, to, {
            maxOps: Game.time % 20 + 1000000,
            costMatrixCallback: roomName => {
              // console.log('Getting cost matrix for room in js: ', roomName);
              if (Game.map.getRoomStatus(roomName).status != "normal") {
                // console.log('Room not normal', roomName);
                return;
              }
              if (cache2.has(roomName)) {
                // console.log('Cache hit', roomName);
                return cache2.get(roomName);
              }
              // console.log('Cache miss', roomName);
              const costMatrix = ephemeral(
                getTerrainCostMatrix(roomName, { plainCost: 1, swampCost: 5, wallCost: 255 }).toCustomCostMatrix()
              );
              // const costMatrix = new CustomCostMatrix();
              cache2.set(roomName, costMatrix);
              // console.log('Cost matrix set in cache', roomName, costMatrix);
              return costMatrix;
            }
          });
          if (!temp_astarPath) {
            console.log('A* Path not found');
          }
      }, iterations);
      // @ts-ignore
      let astarPath: ClockworkPath = temp_astarPath;
      let weight = 0.1;
      if (avg_cw_time === 0) {
        avg_cw_time = astarTime * 0.5;
      }
      if (avg_pf_time === 0) {
        avg_pf_time = pathFinderTime * 0.5;
      }
      avg_cw_time = (avg_cw_time * (1 - weight)) + (astarTime * weight);
      avg_pf_time = (avg_pf_time * (1 - weight)) + (pathFinderTime * weight);
      console.log(`PathFinder Time: ${avg_pf_time.toFixed(2)}, this tick: ${pathFinderTime.toFixed(2)}`);
      console.log('PathFinder Path', pathFinderPath!.path.length, "rooms opened", visitedRooms.size, "ops", pathFinderPath!.ops);

      let pathArray = astarPath;
      if (pathArray) {
        console.log(`A* Time: ${avg_cw_time.toFixed(2)}, this tick: ${astarTime.toFixed(2)}`);
        console.log('A* Path', astarPath!.length, "rooms opened", cache.size);
        visualizePath(pathArray.toArray());
      } else {
        console.log('A* Path not found');
      }
    }
  },
  {
    name: 'Rust PathFinder',
    color1: COLOR_YELLOW,
    color2: COLOR_CYAN,
    /**
     * Visualization of the Rust-based pathfinder implementation.
     */
    run(rooms) {
      const [originFlag, targetFlag, ...rest] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || !targetFlag) {
        return;
      }
      let from = originFlag.pos;
      let to = targetFlag.pos;
      const iterations = 1;

      // // Initialize Rust pathfinder
      // const rustPathFinder = new RustPathFinder(
      //   1,  // plain cost
      //   5,  // swamp cost
      //   16, // max rooms
      //   10000, // max operations
      //   1500000, // max cost
      //   false, // flee mode
      //   1.2  // heuristic weight
      // );

      let rustPath: RoomPosition[] = [];
      let rustResult: PathfinderResult | null = null;
      const visitedRooms = new Set<string>();
      const rustTime = cpuTime(() => {
        rustResult = rust_pathfinder(
          from,
          [to]
        );
        if (rustResult) {
          rustPath = rustResult.path;
        }
        // rustPath = rustPathFinder.search(
        //   from,
        //   [to]
          // (roomName: string) => {
          //   if (Game.map.getRoomStatus(roomName).status !== "normal") {
          //     return null;
          //   }
          //   const terrain = Game.map.getRoomTerrain(roomName);
          //   const terrainData = new Uint8Array(2500);
          //   for (let y = 0; y < 50; y++) {
          //     for (let x = 0; x < 50; x++) {
          //       const idx = y * 50 + x;
          //       terrainData[idx] = terrain.get(x, y);
          //     }
          //   }
          //   return {
          //     terrain: terrainData,
          //     cost_matrix: null
          //   };
          // }
        // ) || [];
      }, iterations);

      // Compare with built-in PathFinder
      let pathFinderPath: PathFinderPath;
      const pfVisitedRooms = new Set<string>();
      const pathFinderTime = cpuTime(() => {
        pathFinderPath = PathFinder.search(from, {pos: to, range: 0}, {
          maxCost: 1500,
          maxOps: 10000,
          roomCallback: roomName => {
            pfVisitedRooms.add(roomName);
            return new PathFinder.CostMatrix();
          },
          heuristicWeight: 1.2
        });
      }, iterations);

      // Visualize both paths
      visualizePath(pathFinderPath!.path, "red");
      visualizePath(rustPath, "blue");

      // Group positions by room for visualization
      const positionsByRoom = new Map<string, RoomPosition[]>();
      for (const pos of rustPath) {
        if (!positionsByRoom.has(pos.roomName)) {
          positionsByRoom.set(pos.roomName, []);
        }
        positionsByRoom.get(pos.roomName)!.push(pos);
      }

      // Visualize path in each room
      for (const [roomName, positions] of positionsByRoom) {
        const viz = new RoomVisual(roomName);
        // Draw start/end points in this room
        if (positions[0] === rustPath[0]) {
          viz.circle(positions[0].x, positions[0].y, {fill: 'red', radius: 0.5});
        }
        if (positions[positions.length - 1] === rustPath[rustPath.length - 1]) {
          viz.circle(positions[positions.length - 1].x, positions[positions.length - 1].y, {fill: 'green', radius: 0.5});
        }
        // Draw path segment in this room
        viz.poly(positions, {stroke: 'blue', strokeWidth: 0.15, opacity: 0.5});
      }

      // Log performance metrics
      let weight = 0.1;
      if (avg_cw_time === 0) {
        avg_cw_time = rustTime * 0.5;
      }
      if (avg_pf_time === 0) {
        avg_pf_time = pathFinderTime * 0.5;
      }
      avg_cw_time = (avg_cw_time * (1 - weight)) + (rustTime * weight);
      avg_pf_time = (avg_pf_time * (1 - weight)) + (pathFinderTime * weight);

      // console.log('Rust path:', rustPath.map(pos => `${pos.x},${pos.y}-${pos.roomName}`).join(','));
      // console.log('PathFinder path:', pathFinderPath!.path.map(pos => `${pos.x},${pos.y}-${pos.roomName}`).join(','));
      // @ts-ignore
      console.log(`Rust PathFinder Time: ${avg_cw_time.toFixed(2)}, this tick: ${rustTime.toFixed(2)} ops: ${rustResult?.ops}, cost: ${rustResult?.cost}, incomplete: ${rustResult?.incomplete}`);
      console.log('Rust Path Length:', rustPath.length, "rooms opened:", visitedRooms.size);
      console.log(`PathFinder Time: ${avg_pf_time.toFixed(2)}, this tick: ${pathFinderTime.toFixed(2)}`);
      console.log('PathFinder Path Length:', pathFinderPath!.path.length, "rooms opened:", pfVisitedRooms.size, "ops:", pathFinderPath!.ops, "cost:", pathFinderPath!.cost, "incomplete:", pathFinderPath!.incomplete);
    }
  },
  {
    name: 'CPU Usage Comparison Map',
    color1: COLOR_YELLOW,
    color2: COLOR_YELLOW,
    // Queue to store positions that need processing
    positionQueue: [] as Array<{x: number, y: number}>,
    // Store detailed results to persist across ticks
    results: new Map<string, ComparisonResult>(),
    // Flag to track if we've initialized the queue
    initialized: false,

    /**
     * Creates a visualization comparing PathFinder and Clockwork
     * for pathing from every position in the target flag's room back to the origin flag.
     * Shows detailed comparison of path length, cost, operations, and CPU usage.
     * Processes positions incrementally across ticks to stay within CPU limits.
     */
    run(rooms) {
      const [originFlag, targetFlag, ...rest] = Object.values(rooms).reduce((acc, flags) => [...acc, ...flags], []);
      if (!originFlag || !targetFlag) {
        this.initialized = false;
        this.positionQueue = [];
        this.results.clear();
        return;
      }

      const targetRoom = targetFlag.pos.roomName;
      const terrain = Game.map.getRoomTerrain(targetRoom);
      const viz = new RoomVisual(targetRoom);
      // viz.circle(originFlag.pos.x, originFlag.pos.y, {fill: 'blue', radius: 0.3, opacity: 1});

      // Initialize queue if not already done
      if (!this.initialized) {
        this.initialized = true;
        this.positionQueue = [];
        this.results.clear();

        // Add all walkable positions to queue
        for (let y = 0; y < 50; y++) {
          for (let x = 0; x < 50; x++) {
            if (terrain.get(x, y) !== TERRAIN_MASK_WALL && !(originFlag.pos.roomName === targetRoom && originFlag.pos.x === x && originFlag.pos.y === y)) {
              this.positionQueue.push({x, y});
            }
          }
        }
        // Shuffle queue for more interesting visualization
        for (let i = this.positionQueue.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [this.positionQueue[i], this.positionQueue[j]] = [this.positionQueue[j], this.positionQueue[i]];
        }
      }

      // Process positions until we hit CPU limit or queue is empty
      const startCpu = Game.cpu.getUsed();
      const cpuLimit = 100; // CPU limit per tick
      let iterations = 3;
      while (this.positionQueue.length > 0 && (Game.cpu.getUsed() - startCpu) < cpuLimit) {
        const pos = this.positionQueue.pop()!;
        const to = new RoomPosition(pos.x, pos.y, targetRoom);
        const from = originFlag.pos;

        // Measure PathFinder results
        let pathFinderResult;
        const pathFinderTime = cpuTime(() => {
          pathFinderResult = PathFinder.search(from, {pos: to, range: 0}, {
            maxCost: 1500,
            maxOps: 10000,
            roomCallback: roomName => new PathFinder.CostMatrix(),
            heuristicWeight: 1
          });
        }, iterations) / iterations;

        let clockworkResult;
        let clockworkTime = Infinity;
        try {
          // Measure Clockwork results
          clockworkTime = cpuTime(() => {
            const result = jpsPath([from], [to], {
              costMatrixCallback: getTerrainCostMatrix,
              maxOps: 10000
            });
            if (result) {
              // console.log('Clockwork ops', result.ops);
              clockworkResult = {
                path: result.toArray(),
                ops: 0,
                cost: 0,
                incomplete: false
              };
            }
          }, iterations) / iterations;
        } catch (e) {
          console.log('Error at position', pos.x, pos.y, e);
        }

        if (clockworkResult) {
          // @ts-ignore
          console.log('Clockwork ops', clockworkResult.ops, 'pathfinder ops', pathFinderResult?.ops);
          // if (clockworkResult.cost > pathFinderResult?.cost) {
          //   console.log('Clockwork cost', clockworkResult.cost, 'pathfinder cost', pathFinderResult?.cost);
          // }
          // Store detailed results
          const comparison: ComparisonResult = {
            clockwork: {
              // @ts-ignore
              path: clockworkResult.path,
              // @ts-ignore
              ops: clockworkResult.ops,
              // @ts-ignore
              cost: clockworkResult.cost,
              // @ts-ignore
              incomplete: clockworkResult.incomplete,
              cpu: clockworkTime
            },
            pathfinder: {
              // @ts-ignore
              path: [...pathFinderResult.path] as RoomPosition[],
              // @ts-ignore
              ops: pathFinderResult.ops,
              // @ts-ignore
              cost: pathFinderResult.cost,
              // @ts-ignore
              incomplete: pathFinderResult.incomplete,
              cpu: pathFinderTime
            }
          };
          this.results.set(`${pos.x},${pos.y}`, comparison);
        }
      }

      // Draw all results we have so far
      for (const [posStr, result] of this.results) {
        const [x, y] = posStr.split(',').map(Number);
        const { clockwork, pathfinder } = result;
        // console.log("posStr", posStr, "clockwork", clockwork.ops, "pathfinder", pathfinder.ops);
        // Compare path optimality
        if (false) { //(clockwork.cost !== pathfinder.cost || clockwork.path.length !== pathfinder.path.length) {
          // One algorithm found a better path
          // console.log('Clockwork cost', clockwork.cost, 'clockwork path length', clockwork.path.length, 'Pathfinder cost', pathfinder.cost, 'pathfinder path length', pathfinder.path.length);
          const clockworkWon = clockwork.cost <= pathfinder.cost && clockwork.path.length <= pathfinder.path.length;
          let cost = pathfinder.cost - clockwork.cost;
          let length = pathfinder.path.length - clockwork.path.length;
          let text = cost == 0 ? length : cost;
          viz.circle(x, y, {
            radius: 0.3,
            fill: cost >= 0 ? 'green' : 'red',
            stroke: length >= 0 ? 'green' : 'red',
            strokeWidth: 0.1,
            opacity: 0.8
          });
          // viz.text(
          //   cost + " " + length,
          //   x,
          //   y,
          //   {
          //     color: clockworkWon ? 'green' : 'red',
          //     font: 0.3,
          //     align: 'center',
          //     opacity: 1
          //   }
          // );
        } else {
          // Paths are equally optimal, compare ops and CPU
          const clockworkBetterCpu = clockwork.cpu <= pathfinder.cpu;
          const clockworkBetterOps = clockwork.ops <= pathfinder.ops;
          const cpu = (clockwork.cpu - pathfinder.cpu).toFixed(2); 
          const ops = (clockwork.ops - pathfinder.ops);
          if (ops === 0 || true) {
            viz.text(
              cpu,
              x,
              y + 0.1,
              {
                color: clockworkBetterCpu ? 'green' : 'red',
                font: 0.4,
                align: 'center',
                opacity: 1
              }
            );
          } else {
            viz.text(
              cpu,
              x,
              y,
              {
                color: clockworkBetterCpu ? 'green' : 'red',
                font: 0.3,
                align: 'center',
                opacity: 1
              }
            );
            viz.text(
              ops.toString(),
              x,
              y + 0.3,
              {
                color: clockworkBetterOps ? 'green' : 'red',
                font: 0.3,
                align: 'center',
                opacity: 1
              }
            );
          }

          // viz.circle(x, y, {
          //   radius: 0.3,
          //   fill: clockworkBetterOps ? 'green' : 'red',
          //   stroke: clockworkBetterCpu ? 'green' : 'red',
          //   strokeWidth: 0.1,
          //   opacity: 0.8
          // });
        }
      }

      // Show progress
      if (this.positionQueue.length > 0) {
        console.log(`CPU Usage Map: ${Math.floor((1 - this.positionQueue.length / 2500) * 100)}% complete, ${this.positionQueue.length} positions remaining`);
      } else {
        console.log('CPU Usage Map: Complete!');
        
        // Calculate and display overall statistics
        let totalPositions = this.results.size;
        let betterPaths = 0;
        let equalPaths = 0;
        let betterCpu = 0;
        let betterOps = 0;
        
        for (const result of this.results.values()) {
          const { clockwork, pathfinder } = result;
          if (clockwork.cost < pathfinder.cost || clockwork.path.length < pathfinder.path.length) {
            betterPaths++;
          } else if (clockwork.cost === pathfinder.cost && clockwork.path.length === pathfinder.path.length) {
            equalPaths++;
            if (clockwork.cpu < pathfinder.cpu) betterCpu++;
            if (clockwork.ops < pathfinder.ops) betterOps++;
          }
        }
        
        console.log(`
          Statistics:
          Total Positions: ${totalPositions}
          Better Paths: ${betterPaths} (${((betterPaths/totalPositions)*100).toFixed(1)}%)
          Equal Paths: ${equalPaths} (${((equalPaths/totalPositions)*100).toFixed(1)}%)
          Of Equal Paths:
            Better CPU: ${betterCpu} (${((betterCpu/equalPaths)*100).toFixed(1)}%)
            Better Ops: ${betterOps} (${((betterOps/equalPaths)*100).toFixed(1)}%)
        `.replace(/^ +/gm, ''));
      }
    }
  } as CpuComparisonVisualizer
] satisfies FlagVisualizer[];
