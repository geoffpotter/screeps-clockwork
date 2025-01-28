import { Benchmark, BenchmarkResult, BenchmarkSuite } from ".";
import { toRoomPosition } from "./positions";
import { drawPath, PathfindingBenchmarkArgs } from "./paths";
import { getBenchmarkPositions } from "./positions";
import { ClockworkPath, ephemeral, getTerrainCostMatrix, rust_pathfinder } from "../../../src";
import { 
    js_astar_multiroom_distance_map2, 
    js_astar_multiroom_path2, 
    js_astar_multiroom_path3, 
    js_astar_path, 
    js_astar_path_heap, 
    js_astar_path_numeric, 
    js_astar_path_standard,
    js_path_to_multiroom_distance_map_origin,
    js_bidirectional_astar_path,
    js_theta_star_path,
    js_lazy_theta_star_path,
    js_dstar_lite_path,
    js_contraction_hierarchies_path
} from "../../../src/wasm/screeps_clockwork";
import { fromPackedRoomName } from "../../../src/utils/fromPacked";
import { js_astar_multiroom_distance_map } from "../../../src/wasm/screeps_clockwork";
import { referenceGetRange } from "../referenceAlgorithms/getRange";
import { visualizeDistanceMap } from "../../visualizations/helpers/visualizeDistanceMap";


let suite: BenchmarkSuite<RoomPosition[], PathfindingBenchmarkArgs> = {
    name: "Pathfinding",
    cases: [
        // {
        //     benchmarkName: "Same Room",
        //     setup_args: () => {
        //         let num_cases = 1000;
        //         let positions = getBenchmarkPositions({
        //             topLeftRoom: "W7N3",
        //             bottomRightRoom: "W7N3",
        //             positionsPerRoom: num_cases + 1
        //         });
        //         let cases: PathfindingBenchmarkArgs[] = [];
        //         for (let i = 0; i < num_cases; i++) {
        //             cases.push({
        //                 origins: [toRoomPosition(positions.walkable[(i) % positions.walkable.length])],
        //                 goals: [toRoomPosition(positions.walkable[(i + 1) % positions.walkable.length])]
        //             });
        //         }
        //         return cases;
        //     },
        // },
        // {
        //     benchmarkName: "2 Rooms",
        //     setup_args: () => {
        //         let num_cases = 1000;
        //         let positions = getBenchmarkPositions({
        //             topLeftRoom: "W8N3",
        //             bottomRightRoom: "W7N3",
        //             positionsPerRoom: num_cases + 1
        //         });
        //         let cases: PathfindingBenchmarkArgs[] = [];
        //         for (let i = 0; i < num_cases; i++) {
        //             cases.push({
        //                 origins: [toRoomPosition(positions.walkable[(i) % positions.walkable.length])],
        //                 goals: [toRoomPosition(positions.walkable[(i + 1) % positions.walkable.length])]
        //             });
        //         }
        //         return cases;
        //     },
        // },
        {
            benchmarkName: "2x2 Rooms",
            setup_args: () => {
                let num_cases = 50;
                let positions = getBenchmarkPositions({
                    topLeftRoom: "W8N4",
                    bottomRightRoom: "W7N3",
                    positionsPerRoom: num_cases + 1
                });
                let cases: PathfindingBenchmarkArgs[] = [];
                for (let i = 0; i < num_cases; i++) {
                    cases.push({
                        origins: [toRoomPosition(positions.walkable[(i) % positions.walkable.length])],
                        goals: [toRoomPosition(positions.walkable[(i + 1) % positions.walkable.length])]
                    });
                }
                return cases;
            },
        },
        // {
        //     benchmarkName: "3x3 Rooms",
        //     setup_args: () => {
        //         let num_cases = 100;
        //         let positions = getBenchmarkPositions({
        //             topLeftRoom: "W8N4",
        //             bottomRightRoom: "W6N2",
        //             positionsPerRoom: num_cases + 1
        //         });
        //         let cases: PathfindingBenchmarkArgs[] = [];
        //         for (let i = 0; i < num_cases; i++) {
        //             cases.push({
        //                 origins: [toRoomPosition(positions.walkable[(i) % positions.walkable.length])],
        //                 goals: [toRoomPosition(positions.walkable[(i + 1) % positions.walkable.length])]
        //             });
        //         }
        //         return cases;
        //     },
        // },
        // {
        //     benchmarkName: "5x5 Rooms",
        //     setup_args: () => {
        //         let num_cases = 20;
        //         let positions = getBenchmarkPositions({
        //             topLeftRoom: "W9N5",
        //             bottomRightRoom: "W5N1",
        //             positionsPerRoom: num_cases + 1
        //         });
        //         let cases: PathfindingBenchmarkArgs[] = [];
        //         for (let i = 0; i < num_cases; i++) {
        //             cases.push({
        //                 origins: [toRoomPosition(positions.walkable[(i) % positions.walkable.length])],
        //                 goals: [toRoomPosition(positions.walkable[(i + 1) % positions.walkable.length])]
        //             });
        //         }
        //         return cases;
        //     },
        // },
    ],
    implementations: [
        {
            name: "Pathfinder",
            fn: ({ origins, goals }) => {
                let result = PathFinder.search(origins[0], goals.map(g => ({ pos: g, range: 0 })), {
                    plainCost: 1,
                    swampCost: 5,
                    maxOps: 10_000,
                    maxRooms: 10_000,
                });
                // console.log("pathfinder", result.path.length, result.cost, result.incomplete);
                // drawPath(result.path, "#00FF00");
                if (result.incomplete) {
                    console.log("pathfinder", result.path.length, result.cost, result.incomplete);
                    return [];
                }
                return result.path;
            }
        },
        {
            name: "Rust Pathfinder",
            fn: ({ origins, goals }) => {
                let result = rust_pathfinder(origins[0], goals);
                if (!result || result.incomplete) {
                    console.log("rust", result?.path.length, result?.cost, result?.incomplete)
                    return [];
                }
                // console.log("rust", result.path.length, result.cost)
                // drawPath(result.path, "#FF0000");
                return result.path;
            }
        },
        {
            name: "A* Multiroom Distance Map",
            fn: ({ origins, goals }) => {
                // @ts-ignore
                const startPacked = new Uint32Array([origins[0].__packedPos]);
                // @ts-ignore
                const goalPacked = new Uint32Array([goals[0].__packedPos]);
                const distanceMap = js_astar_multiroom_distance_map(
                    startPacked,
                    (roomName: number) => getTerrainCostMatrix(fromPackedRoomName(roomName)),
                    10_000,
                    10_000,
                    10_000,
                    goalPacked
                  );
                  let path;
                  try {
                    path = ephemeral(new ClockworkPath(js_path_to_multiroom_distance_map_origin(goalPacked[0], distanceMap)));
                  } catch (e) {
                    distanceMap.get_rooms().forEach(room => {
                        let room_name = fromPackedRoomName(room);
                        visualizeDistanceMap(room_name, distanceMap.get_room(room)!);
                    });
                    console.log("error in path reconstruction", e);
                    return [];
                  }

                //   console.log("astar", path.length);
                //   drawPath(path.toArray(), "#0000FF");
                  return path.toArray().slice(1);
                
            }
        },
        // {
        //     name: "A* Multiroom Distance Map2",
        //     fn: ({ origins, goals }) => {
        //         // @ts-ignore
        //         const startPacked = new Uint32Array([origins[0].__packedPos]);
        //         // @ts-ignore
        //         const goalPacked = new Uint32Array([goals[0].__packedPos]);
        //         const raw_path = js_astar_multiroom_path2(
        //             startPacked,
        //             (roomName: number) => {
        //                 // console.log("roomName", roomName);
        //                 return getTerrainCostMatrix(fromPackedRoomName(roomName))
        //             },
        //             10_000,
        //             10_000,
        //             goalPacked
        //         );
        //         let path = ephemeral(new ClockworkPath(raw_path));
                
        //         // console.log("astar2", path.length);
        //         // drawPath(path.toArray(), "#0000FF");
        //         return path.toArray().slice(1);
        //     }
        // },
        // {
        //     name: "A* Multiroom Distance Map3",
        //     fn: ({ origins, goals }) => {
        //         // @ts-ignore
        //         const startPacked = new Uint32Array([origins[0].__packedPos]);
        //         // @ts-ignore
        //         const goalPacked = new Uint32Array([goals[0].__packedPos]);
        //         const raw_path = js_astar_multiroom_path3(
        //             startPacked,
        //             (roomName: number) => {
        //                 // console.log("roomName", roomName);
        //                 return getTerrainCostMatrix(fromPackedRoomName(roomName))
        //             },
        //             10_000,
        //             10_000,
        //             goalPacked
        //         );
        //         let path = ephemeral(new ClockworkPath(raw_path));
                
        //         // console.log("astar3", path.length);
        //         // drawPath(path.toArray(), "#0000FF");
        //         return path.toArray().slice(1);
        //     }
        // },
        // {
        //     name: "js_astar_path",
        //     fn: ({ origins, goals }) => {
        //         const raw_path = js_astar_path(
        //             // @ts-ignore
        //             origins[0].__packedPos,
        //             // @ts-ignore
        //             goals[0].__packedPos,
        //             (roomName: number) => {
        //                 // console.log("roomName", roomName);
        //                 return getTerrainCostMatrix(fromPackedRoomName(roomName))
        //             },
        //             10_000,
        //             10_000
        //         );
        //         if (!raw_path) {
        //             return [];
        //         }
        //         let path = ephemeral(new ClockworkPath(raw_path));
                
        //         // console.log("js_astar_path", origins[0], goals[0], path.toArrayReversed());
        //         // drawPath(path.toArrayReversed(), "#0000FF");
        //         return path.toArrayReversed();
        //     }
        // },
        // {
        //     name: "js_astar_path_heap",
        //     fn: ({ origins, goals }) => {
        //         const raw_path = js_astar_path_heap(
        //             // @ts-ignore
        //             origins[0].__packedPos,
        //             // @ts-ignore
        //             goals[0].__packedPos,
        //             (roomName: number) => {
        //                 // console.log("roomName", roomName);
        //                 return getTerrainCostMatrix(fromPackedRoomName(roomName))
        //             },
        //             10_000,
        //             10_000
        //         );
        //         if (!raw_path) {
        //             return [];
        //         }
        //         let path = ephemeral(new ClockworkPath(raw_path));
                
        //         // console.log("js_astar_path", origins[0], goals[0], path.toArrayReversed());
        //         // drawPath(path.toArrayReversed(), "#0000FF");
        //         return path.toArrayReversed();
        //     }
        // },
        // {
        //     name: "js_astar_path_numeric",
        //     fn: ({ origins, goals }) => {
        //         const raw_path = js_astar_path_numeric(
        //             // @ts-ignore
        //             origins[0].__packedPos,
        //             // @ts-ignore
        //             goals[0].__packedPos,
        //             (roomName: number) => {
        //                 // console.log("roomName", roomName);
        //                 return getTerrainCostMatrix(fromPackedRoomName(roomName))
        //             },
        //             10_000,
        //             10_000
        //         );
        //         if (!raw_path) {
        //             return [];
        //         }
        //         let path = ephemeral(new ClockworkPath(raw_path));
                
        //         // console.log("js_astar_path", origins[0], goals[0], path.toArrayReversed());
        //         // drawPath(path.toArrayReversed(), "#0000FF");
        //         return path.toArrayReversed();
        //     }
        // },
        // {
        //     name: "js_astar_path_standard",
        //     fn: ({ origins, goals }) => {
        //         const raw_path = js_astar_path_standard(
        //             // @ts-ignore
        //             origins[0].__packedPos,
        //             // @ts-ignore
        //             goals[0].__packedPos,
        //             (roomName: number) => {
        //                 // console.log("roomName", roomName);
        //                 return getTerrainCostMatrix(fromPackedRoomName(roomName))
        //             },
        //             10_000,
        //             10_000
        //         );
        //         if (!raw_path) {
        //             return [];
        //         }
        //         let path = ephemeral(new ClockworkPath(raw_path));
                
        //         // console.log("js_astar_path", origins[0], goals[0], path.toArrayReversed());
        //         // drawPath(path.toArrayReversed(), "#0000FF");
        //         return path.toArrayReversed();
        //     }
        // },

        {
            name: "Bidirectional A*",
            fn({ origins, goals }) {
                // @ts-ignore
                const startPacked = new Uint32Array([origins[0].__packedPos]);
                // @ts-ignore
                const goalPacked = new Uint32Array([goals[0].__packedPos]);
                let path = js_bidirectional_astar_path(
                    startPacked[0],
                    goalPacked[0],
                    getTerrainCostMatrix,
                    10000,
                    50
                );
                if (path) {
                    return path.to_array();
                }
                return [];
            }
        },
        {
            name: "Theta*",
            fn({ origins, goals }) {
                // @ts-ignore
                const startPacked = new Uint32Array([origins[0].__packedPos]);
                // @ts-ignore
                const goalPacked = new Uint32Array([goals[0].__packedPos]);
                let path = js_theta_star_path(
                    startPacked[0],
                    goalPacked[0],
                    getTerrainCostMatrix,
                    10000
                );
                if (path) {
                    return path.to_array();
                }
                return [];
            }
        },
        {
            name: "Lazy Theta*",
            fn({ origins, goals }) {
                // @ts-ignore
                const startPacked = new Uint32Array([origins[0].__packedPos]);
                // @ts-ignore
                const goalPacked = new Uint32Array([goals[0].__packedPos]);
                let path = js_lazy_theta_star_path(
                    startPacked[0],
                    goalPacked[0],
                    getTerrainCostMatrix,
                    10000
                );
                if (path) {
                    return path.to_array();
                }
                return [];
            }
        },
        {
            name: "D* Lite",
            fn({ origins, goals }) {
                let origin = origins[0];
                let goal = goals[0];
                // @ts-ignore
                let start = origin.__packedPos;
                // @ts-ignore
                let end = goal.__packedPos;
                let path = js_dstar_lite_path(
                    start,
                    end,
                    getTerrainCostMatrix,
                    10000,
                    50
                );
                if (path) {
                    return path.to_array();
                }
                return [];
            }
        },
        {
            name: "Contraction Hierarchies",
            fn({ origins, goals }) {
                let origin = origins[0];
                let goal = goals[0];
                // @ts-ignore
                let start = origin.__packedPos;
                // @ts-ignore
                let end = goal.__packedPos;
                let path = js_contraction_hierarchies_path(
                    start,
                    end,
                    getTerrainCostMatrix,
                    10000,
                    50
                );
                if (path) {
                    return path.to_array();
                }
                return [];
            }
        }
    ],
    validate(result, referenceResult, args) {
        // if (result.length > referenceResult.length) {
        //     return false;
        // }

        
        let first_pos = result[0];
        let last_pos = result[result.length - 1];
        
        let start_pos = args.origins[0];
        let goal_pos = args.goals[0];

        if (result.length < referenceGetRange(start_pos, goal_pos)) { // path is too short
            // console.log("start_pos", start_pos, "goal_pos", goal_pos, "range", referenceGetRange(start_pos, goal_pos));
            // console.log("path is too short", result.length, referenceGetRange(start_pos, goal_pos));
            return `path is too short: ${result.length} < ${referenceGetRange(start_pos, goal_pos)}`;
        }

        let cost = 0;
        // check that path doesn't contain any walls
        for (let pos of result) {
            let terrain = Game.map.getRoomTerrain(pos.roomName);
            if (terrain.get(pos.x, pos.y) === TERRAIN_MASK_WALL) {
                // console.log("wall in path", pos);
                return `path hits a wall: ${pos}`;
            }
            let terrain_type = terrain.get(pos.x, pos.y);
            cost += terrain_type === TERRAIN_MASK_SWAMP ? 5 : 1;

        }

        if (referenceResult.length > 0) {
            let reference_cost = 0;
            for (let pos of referenceResult) {
                let terrain = Game.map.getRoomTerrain(pos.roomName);
                let terrain_type = terrain.get(pos.x, pos.y);
                reference_cost += terrain_type === TERRAIN_MASK_SWAMP ? 5 : 1;
            }

            if (cost > reference_cost) {
                // console.log("cost is greater than reference cost", cost, reference_cost);
                // console.log("ref len", referenceResult.length, "path len", result.length);
                // console.log("origin", args.origins[0], "goal", args.goals[0]);
                // console.log("first pos", first_pos, "last pos", last_pos);
                return `path cost is greater than reference cost: ${cost} > ${reference_cost}`;
            }
        }

        let start_ok = referenceGetRange(first_pos, start_pos) <= 1;
        let end_ok = referenceGetRange(last_pos, goal_pos) <= 0;
        if (!start_ok) {
            // console.log("bad path.  start", start_pos, first_pos, start_ok, referenceGetRange(first_pos, start_pos), "end", goal_pos, last_pos, end_ok, referenceGetRange(last_pos, goal_pos));
            return `path is not within 1 tile of start: ${referenceGetRange(first_pos, start_pos)}`;
        }
        if (!end_ok) {
            return `path is not within 0 tiles of goal: ${referenceGetRange(last_pos, goal_pos)}`;
        }
        // path should start within 1 tile of start and end within 0 tiles of goal
        return true;
    },
}

let benchmark = new Benchmark(suite);

export function runBenchmarks() {
    if (benchmark.run()) {
        benchmark.displayResults()
        // printBenchmarkResults(results);
    }
}
