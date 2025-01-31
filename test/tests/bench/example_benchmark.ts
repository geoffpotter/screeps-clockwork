import { bench } from './benchmark_framework';
import { getRange } from '../../../src/index';
import { referenceGetRange } from '../referenceAlgorithms/getRange';

function generateRandomRoomName(): string {
    const ns = Math.random() < 0.5 ? 'N' : 'S';
    const ew = Math.random() < 0.5 ? 'W' : 'E';
    return `${ew}${Math.floor(Math.random() * 10)}${ns}${Math.floor(Math.random() * 10)}`;
}

function generateRandomPosition(): RoomPosition {
    return new RoomPosition(
        Math.floor(Math.random() * 50),
        Math.floor(Math.random() * 50),
        generateRandomRoomName()
    );
}

console.log("in example benchmark");
bench<number, [RoomPosition, RoomPosition]>(
    'getRange',
    referenceGetRange,
    (mark) => {
        // Set number of iterations for more accurate averaging
        mark.iterations(10);

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
            const pairs: [RoomPosition, RoomPosition][] = [];
            // Generate 1000 random position pairs
            for (let i = 0; i < 1000; i++) {
                pairs.push([generateRandomPosition(), generateRandomPosition()]);
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
