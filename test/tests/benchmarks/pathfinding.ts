import { bench } from '../helpers/benchmark';
import { ClockworkCostMatrix, ClockworkPath, astarMultiroomDistanceMap, bfsMultiroomDistanceMap, dijkstraMultiroomDistanceMap, ephemeral, getRange, getTerrainCostMatrix } from '../../../src/index';
import { getPositionsInArea, getPositionsInRoom } from '../helpers/positions';
import { getPathCost, pathIsValid } from '../helpers/paths';

console.log("in pathfinding benchmark");

interface PathFinderPath {
    path: RoomPosition[];
    ops: number;
    cost: number;
    incomplete: boolean;
}

function pathfinder(from: RoomPosition, to: RoomPosition[]) {
    // let destinations = to.map(pos => ({pos: pos, range: 0}));


    const visitedRooms = new Set<string>();
    let res = PathFinder.search(from, to[0], {
        maxCost: 1500,
        maxOps: 10000,
        roomCallback: roomName => {
          visitedRooms.add(roomName);
          return new PathFinder.CostMatrix();
        },
        heuristicWeight: 1
    });
    if (res.incomplete) {
        console.log("pathfinder failed", from, to, res.path);
        return [];
    }
    return res.path;
}

// Combined benchmark for both single and multiple destination pathfinding
bench(
    'pathfinder',
    pathfinder,
    (mark) => {
        // Set number of iterations for more accurate averaging
        mark.iterations(5);

        // Define test cases for single destination
        mark.test('basic test', () => {
            const from = new RoomPosition(5, 5, 'W1N1');
            const to = new RoomPosition(45, 45, 'W1N1');
            return new Array(100).fill([from, [to]]);
        });

        // Define test cases for single destination
        mark.test('single-destination: same room', () => {
            let positions = getPositionsInRoom('W1N1', 300, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < positions.length - 1; i++) {
                pairs.push([positions[i], [positions[i+1]]]);
            }
            return pairs;
        });

        // mark.test('single-destination: adjacent rooms', () => {
        //     return [
        //         [new RoomPosition(5, 5, 'W1N1'), [new RoomPosition(5, 5, 'W2N1')]],
        //         [new RoomPosition(0, 25, 'W1N1'), [new RoomPosition(49, 25, 'W2N1')]],
        //         [new RoomPosition(25, 0, 'W1N1'), [new RoomPosition(25, 49, 'W1N2')]]
        //     ];
        // });

        mark.test('one room apart', () => {
            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N1', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]);
            }
            return pairs;
        });

        mark.test('two rooms apart', () => {
            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N2', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]);
            }
            return pairs;
        });

        mark.test('three rooms apart', () => {
            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N3', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]); 
            }
            return pairs;
        });

        mark.test('four rooms apart', () => {
            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N4', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]);
            }
            return pairs;
        });

        mark.test('five rooms apart', () => {
            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N5', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]); 
            }
            return pairs;
        });

        mark.test('six rooms apart', () => {
            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N6', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]);  
            }
            return pairs;
        });

        mark.test('seven rooms apart', () => {

            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N7', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]);  
            }
            return pairs;
        });

        mark.test('eight rooms apart', () => {
            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N8', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]);  
            }
            return pairs;
        });
        
        mark.test('nine rooms apart', () => {
            let room1 = getPositionsInRoom('W0N0', 100, 'walkable');
            let room2 = getPositionsInRoom('W0N9', 100, 'walkable');
            let pairs: [RoomPosition, RoomPosition[]][] = [];
            for (let i = 0; i < room1.length; i++) {
                pairs.push([room1[i], [room2[i]]]);  
            }
            return pairs;
        });
        // // Define test cases for multiple destinations
        // mark.test('multi-destination: same room', () => {
        //     return [
        //         [
        //             new RoomPosition(5, 5, 'W1N1'),
        //             [
        //                 new RoomPosition(45, 45, 'W1N1'),
        //                 new RoomPosition(5, 45, 'W1N1'),
        //                 new RoomPosition(45, 5, 'W1N1'),
        //                 new RoomPosition(5, 5, 'W1N1')
        //             ]
        //         ]
        //     ];
        // });

        // mark.test('multi-destination: multiple rooms', () => {
        //     return [
        //         [
        //             new RoomPosition(5, 5, 'W1N1'),
        //             [
        //                 new RoomPosition(45, 45, 'W1N1'),
        //                 new RoomPosition(5, 45, 'W1N1'),
        //                 new RoomPosition(45, 5, 'W1N2'),
        //                 new RoomPosition(5, 5, 'W1N2')
        //             ]
        //         ]
        //     ];
        // });

        let cache = new Map<string, ClockworkCostMatrix>();
        mark.beforeEach((args) => {
            cache = new Map<string, ClockworkCostMatrix>();
        });

        // mark.implement('pf2', pathfinder)
        // mark.implement('pf3', pathfinder)
        // mark.implement('pf4', pathfinder)
        // mark.implement('pf5', pathfinder)
        // Add implementations to test
        mark.implement('clockwork-astar', (from, to) => {
            const distanceMap = ephemeral(
                astarMultiroomDistanceMap([from], {
                    costMatrixCallback: roomName => {
                        if (cache.has(roomName)) {
                            return cache.get(roomName);
                        }
                        const costMatrix = ephemeral(getTerrainCostMatrix(roomName));
                        cache.set(roomName, costMatrix);
                        return costMatrix;
                    },
                    anyOfDestinations: Array.isArray(to) ? to : [to]
                })
            );
            let path = distanceMap.pathToOrigin(Array.isArray(to) ? to[0] : to);
            return path.toArray().slice(1);
        });

    // const cache = new Map<string, ClockworkCostMatrix>();
        mark.implement('clockwork-dijkstra', (from, to) => {
            const distanceMap = ephemeral(
                dijkstraMultiroomDistanceMap([from], {
                    costMatrixCallback: roomName => {
                        if (cache.has(roomName)) {
                            return cache.get(roomName);
                        }
                        const costMatrix = ephemeral(getTerrainCostMatrix(roomName));
                        cache.set(roomName, costMatrix);
                        return costMatrix;
                    },
                    anyOfDestinations: Array.isArray(to) ? to : [to]
                })
            );
            let path = distanceMap.pathToOrigin(Array.isArray(to) ? to[0] : to);
            return path.toArray().slice(1);
        });

        // const visitedRooms = new Set<string>();
        // mark.implement('pathfinder Multi', (from, to) => {
        //     let destinations = to.map(pos => ({pos: pos, range: 0}));


            
        //     let res = PathFinder.search(from, destinations, {
        //         maxCost: 1500,
        //         maxOps: 10000,
        //         heuristicWeight: 1
        //     });
        //     if (res.incomplete) {
        //         console.log("pathfinder failed", from, to, res.path);
        //         return [];
        //     }
        //     return res.path;
        // });

        // mark.implement('clockwork-bfs', (from, to) => {
        //     let distanceMap = bfsMultiroomDistanceMap([from], {
        //         costMatrixCallback: roomName => ephemeral(getTerrainCostMatrix(roomName)),
        //         anyOfDestinations: Array.isArray(to) ? to : [to]
        //     });
        //     let path = distanceMap.pathToOrigin(Array.isArray(to) ? to[0] : to);
        //     return path.toArray().slice(1);
        // });

        // Validate results match reference
        mark.validate((result, args, referenceResult) => {
            if (!result || result.length === 0) return 'No path found';
            
            if (referenceResult.length > 0) {
                let ourCost = getPathCost(result);
                let refCost = getPathCost(referenceResult);
                if (ourCost > refCost) {
                    console.log("us", ourCost, result);
                    console.log("ref", refCost, referenceResult);
                    return `path cost is greater than reference cost: ${ourCost} > ${refCost}`;
                }
            }

            let start_pos = args[0];
            let goals = args[1];

            if (!pathIsValid(result, [{pos: start_pos, range: 1}], goals.map(goal => ({pos: goal, range: 0})))) {
                if (pathIsValid(result, goals.map(goal => ({pos: goal, range: 0})), [{pos: start_pos, range: 1}])) {
                    return `path is backwards`;
                }
                return `path is not valid`;
            }

            return true;
        });

        // Add extra metrics
        mark.minimize('path length', (result) => {
            if (result === undefined) {
                return Infinity;
            }
            return result.length;
        });


    }
); 