import { Benchmark, BenchmarkResult, BenchmarkSuite } from ".";
import { toRoomPosition } from "./positions";
import { drawPath, PathfindingBenchmarkArgs } from "./paths";
import { getBenchmarkPositions } from "./positions";
import { ClockworkPath, ephemeral, getTerrainCostMatrix, rust_pathfinder } from "../../../src";
import { js_path_to_multiroom_distance_map_origin } from "../../../src/wasm/screeps_clockwork";
import { fromPackedRoomName } from "../../../src/utils/fromPacked";
import { js_astar_multiroom_distance_map } from "../../../src/wasm/screeps_clockwork";



let suite: BenchmarkSuite<RoomPosition[], PathfindingBenchmarkArgs> = {
    name: "Pathfinding",
    cases: [
        {
            benchmarkName: "Same Room",
            setup_args: () => {
                let num_cases = 500;
                let positions = getBenchmarkPositions({
                    topLeftRoom: "W7N3",
                    bottomRightRoom: "W7N3",
                    positionsPerRoom: num_cases + 1
                });
                let cases: PathfindingBenchmarkArgs[] = [];
                for (let i = 0; i < num_cases; i++) {
                    cases.push({
                        origins: [toRoomPosition(positions.walkable[i])],
                        goals: [toRoomPosition(positions.walkable[i + 1])]
                    });
                }
                return cases;
            },
        },
        {
            benchmarkName: "5x5 Rooms",
            setup_args: () => {
                let num_cases = 1000;
                let positions = getBenchmarkPositions({
                    topLeftRoom: "W9N5",
                    bottomRightRoom: "W5N1",
                    positionsPerRoom: num_cases + 1
                });
                let cases: PathfindingBenchmarkArgs[] = [];
                for (let i = 0; i < num_cases; i++) {
                    cases.push({
                        origins: [toRoomPosition(positions.walkable[i])],
                        goals: [toRoomPosition(positions.walkable[i + 1])]
                    });
                }
                return cases;
            },
        }
    ],
    implementations: [
        {
            name: "Pathfinder",
            fn: ({ origins, goals }) => {
                let result = PathFinder.search(origins[0], goals.map(g => ({ pos: g, range: 0 })), {
                    plainCost: 1,
                    swampCost: 5,
                });
                console.log("pathfinder", result.path.length, result.cost);
                drawPath(result.path, "#00FF00");
                return result.path;
            }
        },
        {
            name: "Rust Pathfinder",
            fn: ({ origins, goals }) => {
                let result = rust_pathfinder(origins[0], goals);
                if (!result) {
                    return [];
                }
                console.log("rust", result.path.length, result.cost)
                drawPath(result.path, "#FF0000");
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
                    goalPacked
                );
                const path = ephemeral(new ClockworkPath(js_path_to_multiroom_distance_map_origin(goalPacked[0], distanceMap)));
                return path.toArray();
            }
        },
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
                    return drawPath(path);
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
                    10000,
                    50
                );
                if (path) {
                    return drawPath(path);
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
                    10000,
                    50
                );
                if (path) {
                    return drawPath(path);
                }
                return [];
            }
        },
        {
            name: "D* Lite",
            fn({ origins, goals }) {
                let origin = origins[0];
                let goal = goals[0];
                let start = ephemeral.wasm.pack_position(origin);
                let end = ephemeral.wasm.pack_position(goal);
                let path = ephemeral.wasm.js_dstar_lite_path(
                    start,
                    end,
                    getTerrainCostMatrix,
                    10000,
                    50
                );
                if (path) {
                    return drawPath(path);
                }
                return [];
            }
        },
        {
            name: "Contraction Hierarchies",
            fn({ origins, goals }) {
                let origin = origins[0];
                let goal = goals[0];
                let start = ephemeral.wasm.pack_position(origin);
                let end = ephemeral.wasm.pack_position(goal);
                let path = ephemeral.wasm.js_contraction_hierarchies_path(
                    start,
                    end,
                    getTerrainCostMatrix,
                    10000,
                    50
                );
                if (path) {
                    return drawPath(path);
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

        if (result.length < start_pos.getRangeTo(goal_pos)) { // path is too short
            return false;
        }

        let cost = 0;
        // check that path doesn't contain any walls
        for (let pos of result) {
            let terrain = Game.map.getRoomTerrain(pos.roomName);
            if (terrain.get(pos.x, pos.y) === TERRAIN_MASK_WALL) {
                return false;
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
                return false;
            }
        }

        // path should start within 1 tile of start and end within 0 tiles of goal
        return first_pos.inRangeTo(start_pos, 1) && last_pos.inRangeTo(goal_pos, 0);
    },
}

let benchmark = new Benchmark(suite);

export function runBenchmarks() {
    if (benchmark.run()) {
        benchmark.displayResults()
        // printBenchmarkResults(results);
    }
}
