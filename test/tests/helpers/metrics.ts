export type MetricMode = 'minimize' | 'maximize' | 'match';

export interface MetricResult {
    value: number;
    wins: number;
    beatReference: number;
    validCount: number;
}

export class MetricCalculator {
    /**
     * Determines if one value beats another based on the metric mode.
     * For "match" mode, the candidate wins if its absolute difference from the reference is strictly less than the other's.
     */
    static beats(value: number, otherValue: number, mode: MetricMode, referenceValue?: number): boolean {
        if (mode === 'minimize') {
            return value < otherValue;
        } else if (mode === 'maximize') {
            return value > otherValue;
        } else { // match
            if (referenceValue === undefined) throw new Error('Reference value required for match mode');
            const valueDiff = Math.abs(value - referenceValue);
            const otherDiff = Math.abs(otherValue - referenceValue);
            return valueDiff < otherDiff;
        }
    }

    /**
     * Determines if two values are exactly equal.
     */
    static equals(value: number, otherValue: number): boolean {
        return value === otherValue;
    }

    /**
     * Determines winners among a set of implementations for a given metric.
     */
    static findWinners(implementations: { name: string, value: number }[], mode: MetricMode, referenceValue?: number): string[] {
        if (implementations.length === 0) return [];

        if (mode === 'match') {
            if (referenceValue === undefined) throw new Error('Reference value required for match mode');
            // For match mode, find the implementation(s) closest to reference value (using exact differences)
            const diffs = implementations.map(impl => ({
                name: impl.name,
                diff: Math.abs(impl.value - referenceValue)
            }));
            const bestDiff = Math.min(...diffs.map(d => d.diff));
            return diffs.filter(d => d.diff === bestDiff).map(d => d.name);
        } else {
            // For minimize/maximize, find the best value and return all implementations that exactly match it.
            const values = implementations.map(i => i.value);
            const bestValue = mode === 'minimize' ? Math.min(...values) : Math.max(...values);
            return implementations.filter(impl => impl.value === bestValue).map(impl => impl.name);
        }
    }

    /**
     * Determines if an implementation beats the reference implementation.
     * In match mode only an exact value equal to the reference is considered to beat it.
     */
    static beatReference(value: number, referenceValue: number | undefined, mode: MetricMode): boolean {
        if (mode === 'match') {
            if (referenceValue === undefined || referenceValue === 0) {
                throw new Error('Reference value required for match mode');
            }
            return value === referenceValue;
        } else {
            return this.beats(value, referenceValue as number, mode);
        }
    }
} 