import { MetricCalculator } from './metrics';
import { describe, expect, it } from '../helpers';

describe('MetricCalculator', () => {
    describe('beats', () => {
        it('should handle minimize mode correctly', () => {
            expect(MetricCalculator.beats(1, 2, 'minimize')).toBe(true);
            expect(MetricCalculator.beats(2, 1, 'minimize')).toBe(false);
            expect(MetricCalculator.beats(1, 1, 'minimize')).toBe(false);
        });

        it('should handle maximize mode correctly', () => {
            expect(MetricCalculator.beats(2, 1, 'maximize')).toBe(true);
            expect(MetricCalculator.beats(1, 2, 'maximize')).toBe(false);
            expect(MetricCalculator.beats(1, 1, 'maximize')).toBe(false);
        });

        it('should handle match mode correctly when values are different', () => {
            const refValue = 10;
            // 9 is closer to 10 (difference of 1) than 8 (difference of 2)
            expect(MetricCalculator.beats(9, 8, 'match', refValue)).toBe(true);
            expect(MetricCalculator.beats(8, 9, 'match', refValue)).toBe(false);
        });

        it('should not declare a winner when differences are equal in match mode', () => {
            const refValue = 10;
            // Both 9 and 11 are 1 away from 10; neither beats the other.
            expect(MetricCalculator.beats(9, 11, 'match', refValue)).toBe(false);
            expect(MetricCalculator.beats(11, 9, 'match', refValue)).toBe(false);
        });

        it('should require a reference value for match mode', () => {
            let threw = false;
            try {
                MetricCalculator.beats(1, 2, 'match');
            } catch (e) {
                threw = true;
            }
            expect(threw).toBe(true);
        });
    });

    describe('equals', () => {
        it('should consider equal numbers exactly equal', () => {
            expect(MetricCalculator.equals(1, 1)).toBe(true);
        });

        it('should not consider different numbers equal', () => {
            expect(MetricCalculator.equals(1, 1.0000001)).toBe(false);
            expect(MetricCalculator.equals(1, 0.9999999)).toBe(false);
        });
    });

    describe('findWinners', () => {
        it('should find winners in minimize mode', () => {
            const implementations = [
                { name: 'impl1', value: 1 },
                { name: 'impl2', value: 2 },
                { name: 'impl3', value: 1 }
            ];
            const winners = MetricCalculator.findWinners(implementations, 'minimize');
            expect(winners).toEqual(['impl1', 'impl3']);
        });

        it('should find winners in maximize mode', () => {
            const implementations = [
                { name: 'impl1', value: 1 },
                { name: 'impl2', value: 2 },
                { name: 'impl3', value: 1 }
            ];
            const winners = MetricCalculator.findWinners(implementations, 'maximize');
            expect(winners).toEqual(['impl2']);
        });

        it('should find winners in match mode when one candidate exactly matches the reference', () => {
            const implementations = [
                { name: 'impl1', value: 1 },
                { name: 'impl2', value: 1.5 },
                { name: 'impl3', value: 2 }
            ];
            // Only impl2 exactly equals the reference 1.5.
            const winners = MetricCalculator.findWinners(implementations, 'match', 1.5);
            expect(winners).toEqual(['impl2']);
        });

        it('should find winners in match mode when none exactly match the reference', () => {
            // In this case, absolute differences:
            // impl1: |1 - 1.5| = 0.5, impl2: |2 - 1.5| = 0.5, impl3: |1 - 1.5| = 0.5.
            // All implementations are equally close and should all be returned.
            const implementations = [
                { name: 'impl1', value: 1 },
                { name: 'impl2', value: 2 },
                { name: 'impl3', value: 1 }
            ];
            const winners = MetricCalculator.findWinners(implementations, 'match', 1.5);
            expect(winners).toEqual(['impl1', 'impl2', 'impl3']);
        });

        it('should handle empty implementations', () => {
            expect(MetricCalculator.findWinners([], 'minimize')).toEqual([]);
        });

        it('should require a reference value for match mode', () => {
            const implementations = [
                { name: 'impl1', value: 1 },
                { name: 'impl2', value: 2 }
            ];
            let threw = false;
            try {
                MetricCalculator.findWinners(implementations, 'match');
            } catch (e) {
                threw = true;
            }
            expect(threw).toBe(true);
        });

        it('should show beating reference when implementation matches exactly', () => {
            // Add test case where implementation matches reference
            const implementations = [
                { name: 'ref', value: 10 },
                { name: 'impl', value: 10 }
            ];
            const winners = MetricCalculator.findWinners(implementations, 'match', 10);
            expect(winners).toEqual(['ref', 'impl']);
        });
    });

    describe('beatReference', () => {
        it('should handle minimize mode correctly', () => {
            expect(MetricCalculator.beatReference(1, 2, 'minimize')).toBe(true);
            expect(MetricCalculator.beatReference(2, 1, 'minimize')).toBe(false);
        });

        it('should handle maximize mode correctly', () => {
            expect(MetricCalculator.beatReference(2, 1, 'maximize')).toBe(true);
            expect(MetricCalculator.beatReference(1, 2, 'maximize')).toBe(false);
        });

        it('should handle match mode correctly by only beating the reference when equal', () => {
            // For match mode, only a value that exactly equals the reference beats it.
            expect(MetricCalculator.beatReference(10, 10, 'match')).toBe(true);
            expect(MetricCalculator.beatReference(9, 10, 'match')).toBe(false);
            expect(MetricCalculator.beatReference(11, 10, 'match')).toBe(false);
        });

        it('should require a reference value for match mode in beatReference', () => {
            let threw = false;
            try {
                MetricCalculator.beatReference(1, 0, 'match');
            } catch (e) {
                threw = true;
            }
            expect(threw).toBe(true);
        });
    });
}); 