import { bench } from '../helpers/benchmark';
import { getRange } from '../../../src/index';
import { getPositionsInArea } from '../helpers/positions';

console.log("in getRange benchmark");

bench<number, [RoomPosition, RoomPosition]>(
    'getRange',
    (pos1, pos2) => pos1.getRangeTo(pos2),
    (mark) => {
        // Set number of iterations for more accurate averaging
        mark.iterations(1000);

        // Define test cases
        mark.test('same room', () => {
            return [
                [new RoomPosition(25, 25, 'W1N1'), new RoomPosition(26, 25, 'W1N1')],
                [new RoomPosition(0, 0, 'W1N1'), new RoomPosition(49, 49, 'W1N1')],
                [new RoomPosition(10, 10, 'W1N1'), new RoomPosition(40, 40, 'W1N1')]
            ];
        });

        mark.test('adjacent rooms', () => {
            return [
                [new RoomPosition(25, 25, 'W1N1'), new RoomPosition(25, 25, 'W2N1')],
                [new RoomPosition(0, 25, 'W1N1'), new RoomPosition(49, 25, 'W2N1')],
                [new RoomPosition(25, 0, 'W1N1'), new RoomPosition(25, 49, 'W1N2')]
            ];
        });

        mark.test('random positions', () => {
            // Get positions from a 3x3 grid of rooms
            const positions = getPositionsInArea({
                topLeftRoom: 'W6N6',
                bottomRightRoom: 'W4N4',
                positionsPerRoom: 112
            });

            const pairs: [RoomPosition, RoomPosition][] = [];
            const allPositions = [...positions.walkable];
            
            // Generate 1000 random position pairs
            for (let i = 0; i < 1000; i++) {
                const pos1 = allPositions[Math.floor(Math.random() * allPositions.length)];
                const pos2 = allPositions[Math.floor(Math.random() * allPositions.length)];
                pairs.push([pos1, pos2]);
            }
            return pairs;
        });

        // Add implementations to test
        mark.implement('clockwork', (pos1, pos2) => getRange(pos1, pos2));
        mark.implement('screeps baseline', (pos1, pos2) => pos1.getRangeTo(pos2));

        // Validate results match reference
        mark.validate((result, args, reference) => {
            const [pos1, pos2] = args;
            if (result === reference) return true;
            return `Expected ${reference} but got ${result} for positions ${pos1} to ${pos2}`;
        });
    }
);
