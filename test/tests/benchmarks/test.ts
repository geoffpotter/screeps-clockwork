import { bench } from "../helpers/benchmark";

function wasteCPU(amt: number) {
    let start_cpu = Game.cpu.getUsed();
    let total = 0;
    while (Game.cpu.getUsed() - start_cpu < amt) {
        total++;
    }
}

type TestResult = [boolean, number];

// Test benchmark with predictable outcomes
bench<TestResult, [number]>('benchmark-test',
    (amt: number) => {
        wasteCPU(amt);
        return [true, amt];  // Always succeeds, returns input as secondary value
    },
    (mark) => {
        mark.iterations(1);  // Single iteration for predictable results
        
        // Test cases to verify different behaviors
        mark.test("single value", () => {
            return [[1]];  // Single test to verify basic win/loss counting
        });
        
        mark.test("ties with reference", () => {
            return [[1], [2]];  // Two tests where middle impl ties with reference
        });
        
        mark.test("ties between impls", () => {
            return [[1], [2], [3]];  // Three tests where impls tie with each other
        });

        mark.test("all fail", () => {
            return [[0]];  // Test where all implementations fail
        });

        mark.test("reference fails", () => {
            return [[-1]];  // Test where reference fails but impls succeed
        });

        // Each implementation has a clear pattern:
        // mark.implement("always-fastest", (amt: number) => {
        //     wasteCPU(amt * 0.5);  // Should win every CPU contest
        //     return [true, 0];  // Should win every extra value contest
        // });

        // mark.implement("always-middle", (amt: number) => {
        //     wasteCPU(amt);  // Should tie with reference
        //     return [true, 1];  // Should tie with reference
        // });

        mark.implement("always-slowest", (amt: number) => {
            wasteCPU(amt * 2);  // Should never win CPU contest
            return [true, 2];  // Should never win extra value contest
        });

        let testIndex = 0;
        mark.implement("sometimes-fails", (amt: number) => {
            wasteCPU(amt);
            // Fail on even numbered tests
            const shouldSucceed = (testIndex % 2) === 0;
            return [shouldSucceed, 1];
        });

        // mark.implement("ties-with-fastest", (amt: number) => {
        //     wasteCPU(amt * 0.5);  // Should tie with always-fastest
        //     return [true, 0];  // Should tie with always-fastest
        // });

        // Add metrics that are easy to verify
        mark.minimize("min", (result, args, referenceResult) => {
            return result[1];
        });

        mark.maximize("max", (result, args, referenceResult) => {
            return result[1];
        });

        mark.match("match", (result, args, referenceResult) => {
            return result[1];
        });

        mark.validate((result, args, referenceResult) => {
            // Fail all implementations in "all fail" case
            if (args[0] === 0) return "forced failure";
            
            // Fail reference in "reference fails" case
            if (args[0] === -1 && result === referenceResult) return "reference failure";
            
            if (result[0] === true) {
                return true;
            }
            return "failed";
        });
    }
);