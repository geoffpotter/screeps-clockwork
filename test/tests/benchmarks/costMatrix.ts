import { bench } from '../helpers/benchmark';
import { ClockworkCostMatrix, ephemeral, getTerrainCostMatrix } from '../../../src/index';

//console.log("in costmatrix benchmark");

// Benchmark CostMatrix get/set operations
bench<void, []>(
    'costmatrix-operations',
    () => {},
    (mark) => {
        // Set number of iterations for more accurate averaging
        mark.iterations(100);

        // Define test cases
        mark.test('set operations', () => {
            return [[]];
        });

        mark.test('get operations', () => {
            return [[]];
        });

        // Add implementations to test
        mark.implement('clockwork', () => {
            const matrix = ephemeral(new ClockworkCostMatrix());
            // Set operations
            for (let x = 0; x < 50; x++) {
                for (let y = 0; y < 50; y++) {
                    matrix.set(x, y, 1);
                }
            }
            // Get operations
            for (let x = 0; x < 50; x++) {
                for (let y = 0; y < 50; y++) {
                    matrix.get(x, y);
                }
            }
        });

        mark.implement('screeps baseline', () => {
            const matrix = new PathFinder.CostMatrix();
            // Set operations
            for (let x = 0; x < 50; x++) {
                for (let y = 0; y < 50; y++) {
                    matrix.set(x, y, 1);
                }
            }
            // Get operations
            for (let x = 0; x < 50; x++) {
                for (let y = 0; y < 50; y++) {
                    matrix.get(x, y);
                }
            }
        });

        // Validate results
        mark.validate(() => true);
    }
);

// Benchmark terrain data filling
bench<CostMatrix | ClockworkCostMatrix, [string]>(
    'terrain-filling',
    (roomName) => {
        const matrix = new PathFinder.CostMatrix();
        const terrain = new Room.Terrain(roomName);
        for (let y = 0; y < 50; y++) {
            for (let x = 0; x < 50; x++) {
                const tile = terrain.get(x, y);
                if (tile === TERRAIN_MASK_WALL) {
                    matrix.set(x, y, 255);
                } else if (tile === TERRAIN_MASK_SWAMP) {
                    matrix.set(x, y, 5);
                } else {
                    matrix.set(x, y, 1);
                }
            }
        }
        return matrix;
    },
    (mark) => {
        // Set number of iterations for more accurate averaging
        mark.iterations(10);

        // Define test cases
        mark.test('single room', () => {
            return [['W1N1']];
        });

        // Add implementations to test
        mark.implement('clockwork', (roomName) => {
            return ephemeral(getTerrainCostMatrix(roomName, { plainCost: 1, swampCost: 5, wallCost: 255 }));
        });

        mark.implement('screeps baseline', (roomName) => {
            const matrix = new PathFinder.CostMatrix();
            const terrain = new Room.Terrain(roomName);
            for (let y = 0; y < 50; y++) {
                for (let x = 0; x < 50; x++) {
                    const tile = terrain.get(x, y);
                    if (tile === TERRAIN_MASK_WALL) {
                        matrix.set(x, y, 255);
                    } else if (tile === TERRAIN_MASK_SWAMP) {
                        matrix.set(x, y, 5);
                    } else {
                        matrix.set(x, y, 1);
                    }
                }
            }
            return matrix;
        });

        // Validate results match reference
        mark.validate((result, args, reference) => {
            const [roomName] = args;
            for (let x = 0; x < 50; x++) {
                for (let y = 0; y < 50; y++) {
                    if (result.get(x, y) !== reference.get(x, y)) {
                        return `Mismatch at (${x},${y}) in room ${roomName}`;
                    }
                }
            }
            return true;
        });
    }
); 