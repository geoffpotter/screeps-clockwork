import {
  astarMultiroomDistanceMap,
  getTerrainCostMatrix as clockworkGetTerrainCostMatrix,
  ephemeral,
  jpsPath
} from '../../src/index';
import { cpuTime } from '../utils/cpuTime';
import { FlagVisualizer } from './helpers/FlagVisualizer';
import { visualizeDistanceMap } from './helpers/visualizeDistanceMap';
import { visualizePath } from './helpers/visualizePath';

function getTerrainCostMatrix(room: string) {
  return ephemeral(clockworkGetTerrainCostMatrix(room));
}

// Extended FlagVisualizer type for CPU Usage Comparison Map
type CpuComparisonVisualizer = FlagVisualizer & {
  positionQueue: Array<{x: number, y: number}>;
  results: Map<string, string>;
  initialized: boolean;
};

let avg_pf_cpu = 0;
let avg_rust_pf_cpu = 0;

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
        astarMultiroomDistanceMap(
          [originFlag.pos],
          targetFlags.map(flag => flag.pos),
          {
            costMatrixCallback: getTerrainCostMatrix,
            maxTiles: 10000
          }
        )
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
        astarMultiroomDistanceMap([originFlag.pos], [targetFlag.pos], {
          costMatrixCallback: getTerrainCostMatrix,
          maxTiles: 10000
        })
      );

      const path = ephemeral(distanceMap.pathToOrigin(targetFlag.pos));
      const pathArray = path.toArray();
      visualizePath(pathArray);
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

      let start_cpu = Game.cpu.getUsed();
      let pathFinderPath: PathFinderPath;
      const visitedRooms = new Set<string>();
      pathFinderPath = PathFinder.search(
        targetFlag.pos,
        { pos: originFlag.pos, range: 0 },
        {
          maxCost: 1500,
          maxOps: 50000,
          maxRooms: 100,
          roomCallback: roomName => {
            visitedRooms.add(roomName);
            return new PathFinder.CostMatrix();
          },
          heuristicWeight: 1
        }
      );
      let end_cpu = Game.cpu.getUsed();
      let pf_cpu = end_cpu - start_cpu;
      let weight = 0.1;
      avg_pf_cpu = avg_pf_cpu * (1 - weight) + pf_cpu * weight;

      visualizePath(pathFinderPath!.path, 'red');
      start_cpu = Game.cpu.getUsed();
      const path = jpsPath(originFlag.pos, [targetFlag.pos]);
      end_cpu = Game.cpu.getUsed();
      let rust_pf_cpu = end_cpu - start_cpu;
      avg_rust_pf_cpu = avg_rust_pf_cpu * (1 - weight) + rust_pf_cpu * weight;
      console.log(`Clockwork CPU: ${rust_pf_cpu}, Avg Clockwork CPU: ${avg_rust_pf_cpu}`);
      console.log(
        `PathFinder CPU: ${pf_cpu}, \nAvg PathFinder CPU: ${avg_pf_cpu}, \nPathFinder Ops: ${pathFinderPath.ops}, \nlength: ${pathFinderPath.path.length}, \nCost: ${pathFinderPath.cost}, \nVisited Rooms: ${visitedRooms.size}, \nIncomplete: ${pathFinderPath.incomplete}`
      );
      visualizePath(path, 'green');
    }
  },
  {
    name: 'CPU Usage Comparison Map',
    color1: COLOR_YELLOW,
    color2: COLOR_YELLOW,
    // Queue to store positions that need processing
    positionQueue: [] as Array<{x: number, y: number}>,
    // Store results to persist across ticks
    results: new Map<string, string>(),
    // Flag to track if we've initialized the queue
    initialized: false,

    /**
     * Creates a visualization comparing CPU usage between PathFinder and Clockwork
     * for pathing from every position in the target flag's room back to the origin flag.
     * Red dots indicate PathFinder was faster, green dots indicate Clockwork was faster.
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


      // Initialize queue if not already done
      if (!this.initialized) {
        this.initialized = true;
        this.positionQueue = [];
        this.results.clear();

        // Add all walkable positions to queue
        for (let y = 0; y < 50; y++) {
          for (let x = 0; x < 50; x++) {
            if (terrain.get(x, y) !== TERRAIN_MASK_WALL) {
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
      
      while (this.positionQueue.length > 0 && (Game.cpu.getUsed() - startCpu) < cpuLimit) {
        const pos = this.positionQueue.pop()!;
        const from = new RoomPosition(pos.x, pos.y, targetRoom);
        const to = originFlag.pos;

        // Measure PathFinder time
        let pathFinderTime = cpuTime(() => {
          PathFinder.search(to, {pos: from, range: 0}, {
            maxCost: 1500,
            maxOps: 10000,
            roomCallback: roomName => new PathFinder.CostMatrix(),
            heuristicWeight: 1
          });
        }, 1);

        let clockworkTime = Infinity;
        try {
          // Measure Clockwork time
          clockworkTime = cpuTime(() => {
            jpsPath(originFlag.pos, [targetFlag.pos])
          }, 1);
        } catch (e) {
          console.log('Error at position', pos.x, pos.y, e);
        }

        // Store result
        const color = pathFinderTime < clockworkTime ? 'red' : 'green';
        this.results.set(`${pos.x},${pos.y}`, color);
      }

      // Draw all results we have so far
      for (const [posStr, color] of this.results) {
        const [x, y] = posStr.split(',').map(Number);
        viz.circle(x, y, {fill: color, radius: 0.3, opacity: 1});
      }

      // Show progress
      if (this.positionQueue.length > 0) {
        console.log(`CPU Usage Map: ${Math.floor((1 - this.positionQueue.length / 2500) * 100)}% complete, ${this.positionQueue.length} positions remaining`);
      } else {
        console.log('CPU Usage Map: Complete!');
      }
    }
  } as CpuComparisonVisualizer
] satisfies FlagVisualizer[];
