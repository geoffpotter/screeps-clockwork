import { cpuTime } from '../../utils/cpuTime';

// Helper type to convert tuple type to function parameters
type TupleToFunction<T extends any[], R> = (...args: T) => R;

interface BenchmarkResult<ResultT> {
    implementationName: string;
    results: ResultT[];
    avgCpuTime: number;
    totalCpuTime: number;
    failures: string[];
    metrics: Map<string, {
        value: number;
        wins: number;
        beatReference: number;
        validCount: number;  // Used for both averaging and win percentage calculation
    }>;
}

interface BenchmarkSuite<ResultT, ArgsT extends any[]> {
    name: string;
    reference: TupleToFunction<ArgsT, ResultT>;
    implementations: Map<string, TupleToFunction<ArgsT, ResultT>>;
    beforeEach?: (args: ArgsT) => void;
    afterEach?: (args: ArgsT) => void;
    validate?: (result: ResultT, args: ArgsT, referenceResult: ResultT) => true | string;
    iterations: number;  // Number of times to run each implementation for averaging
    cases: {
        name: string;
        setup_args: () => ArgsT[];
    }[];
    extraMetrics: {
        name: string;
        mode: 'maximize' | 'minimize' | 'match';
        fn: (result: ResultT, args: ArgsT, referenceResult: ResultT) => number;
    }[];
}

interface BenchmarkSetup<ResultT, ArgsT extends any[]> {
    test: (name: string, fn: () => ArgsT[]) => void;
    implement: (name: string, fn: TupleToFunction<ArgsT, ResultT>) => void;
    validate: (fn: (result: ResultT, args: ArgsT, referenceResult: ResultT) => true | string) => void;
    beforeEach: (fn: (args: ArgsT) => void) => void;
    afterEach: (fn: (args: ArgsT) => void) => void;
    maximize: (name: string, fn: (result: ResultT, args: ArgsT, referenceResult: ResultT) => number) => void;
    minimize: (name: string, fn: (result: ResultT, args: ArgsT, referenceResult: ResultT) => number) => void;
    match: (name: string, fn: (result: ResultT, args: ArgsT, referenceResult: ResultT) => number) => void;
    iterations: (count: number) => void;
}

// Module-level state
const suites = new Map<string, BenchmarkSuite<unknown, unknown[]>>();
let currentSuite: BenchmarkSuite<unknown, unknown[]> | null = null;

// Track active benchmark runners
const activeRunners = new Map<string, BenchmarkRunner<unknown, unknown[]>>();

function makeSetup<Result, Args extends any[]>(suite: BenchmarkSuite<Result, Args>): BenchmarkSetup<Result, Args> {
    return {
        test: (name: string, fn: () => Args[]) => {
            suite.cases.push({ name, setup_args: fn });
        },
        implement: (name: string, fn: TupleToFunction<Args, Result>) => {
            suite.implementations.set(name, fn);
        },
        validate: (fn: (result: Result, args: Args, referenceResult: Result) => true | string) => {
            suite.validate = fn;
        },
        beforeEach: (fn: (args: Args) => void) => {
            suite.beforeEach = fn;
        },
        afterEach: (fn: (args: Args) => void) => {
            suite.afterEach = fn;
        },
        maximize: (name: string, fn: (result: Result, args: Args, referenceResult: Result) => number) => {
            suite.extraMetrics.push({ name, mode: 'maximize', fn });
        },
        minimize: (name: string, fn: (result: Result, args: Args, referenceResult: Result) => number) => {
            suite.extraMetrics.push({ name, mode: 'minimize', fn });
        },
        match: (name: string, fn: (result: Result, args: Args, referenceResult: Result) => number) => {
            suite.extraMetrics.push({ name, mode: 'match', fn });
        },
        iterations: (count: number) => {
            suite.iterations = count;
        }
    };
}

export function bench<Result, Args extends any[]>(
    name: string,
    referenceFn: TupleToFunction<Args, Result>,
    fn: (mark: BenchmarkSetup<Result, Args>) => void
) {
    const suite: BenchmarkSuite<Result, Args> = {
        name,
        reference: referenceFn,
        implementations: new Map(),
        cases: [],
        extraMetrics: [],
        iterations: 1  // Default to 1 iteration if not set
    };
    
    currentSuite = suite as unknown as BenchmarkSuite<unknown, unknown[]>;
    suites.set(name, currentSuite);
    const setup = makeSetup(suite);
    fn(setup);
    currentSuite = null;
}

export class BenchmarkRunner<ResultT, ArgsT extends any[]> {
    private suite: BenchmarkSuite<ResultT, ArgsT>;
    private results: Map<string, Map<string, BenchmarkResult<ResultT>>>;
    private currentArg: number;
    private currentCase: number;
    private args: ArgsT[];
    private state: 'running' | 'complete';
    private tickCpuLimit: number;
    private caseTestCounts: Map<string, number>;
    private referenceResults: Map<string, BenchmarkResult<ResultT>>;  // Track reference results per case

    constructor(suite: BenchmarkSuite<ResultT, ArgsT>, tickCpuLimit: number = Game.cpu.limit * 0.8) {
        this.suite = suite;
        this.results = new Map();
        this.referenceResults = new Map();
        this.currentArg = 0;
        this.currentCase = 0;
        this.state = 'running';
        this.tickCpuLimit = tickCpuLimit;
        this.caseTestCounts = new Map();
        
        // Initialize first case
        const firstCase = this.getCurrentCase();
        this.args = firstCase.setup_args();
        this.caseTestCounts.set(firstCase.name, this.args.length);
        
        // Initialize results
        this.initializeResults();
    }

    private initializeResults() {
        const caseName = this.getCurrentCase().name;
        if (!this.results.has(caseName)) {
            const caseResults = new Map<string, BenchmarkResult<ResultT>>();
            
            // Initialize reference result
            const referenceMetrics = new Map();
            referenceMetrics.set('CPU Time', {
                value: 0,
                wins: 0,
                beatReference: 0,
                validCount: 0
            });
            for (const metric of this.suite.extraMetrics) {
                referenceMetrics.set(metric.name, {
                    value: 0,
                    wins: 0,
                    beatReference: 0,
                    validCount: 0
                });
            }
            this.referenceResults.set(caseName, {
                implementationName: 'reference',
                results: [],
                avgCpuTime: 0,
                totalCpuTime: 0,
                failures: [],
                metrics: referenceMetrics
            });

            // Initialize implementation results
            this.suite.implementations.forEach((_, name) => {
                const metrics = new Map();
                metrics.set('CPU Time', {
                    value: 0,
                    wins: 0,
                    beatReference: 0,
                    validCount: 0
                });
                for (const metric of this.suite.extraMetrics) {
                    metrics.set(metric.name, {
                        value: 0,
                        wins: 0,
                        beatReference: 0,
                        validCount: 0
                    });
                }
                
                caseResults.set(name, {
                    implementationName: name,
                    results: [],
                    avgCpuTime: 0,
                    totalCpuTime: 0,
                    failures: [],
                    metrics
                });
            });
            this.results.set(caseName, caseResults);
        }
    }

    private getCurrentCase() {
        return this.suite.cases[this.currentCase];
    }

    public run(): boolean {
        if (this.state === 'complete') return true;

        while (Game.cpu.getUsed() < this.tickCpuLimit) {
            if (this.runContest()) {
                return true;
            }
        }

        console.log(`Done with tick - Case: ${this.getCurrentCase().name}, Arg: ${this.currentArg + 1}/${this.args.length}`);
        return false;
    }

    private runContest(): boolean {
        if (this.currentArg >= this.args.length) {
            if (this.currentCase >= this.suite.cases.length - 1) {
                this.complete();
                return true;
            } else {
                this.currentCase++;
                const nextCase = this.getCurrentCase();
                this.args = nextCase.setup_args();
                this.caseTestCounts.set(nextCase.name, this.args.length);
                this.currentArg = 0;
                this.initializeResults();
                console.log("switching to next case", nextCase.name);
            }
        }

        const currentArgs = this.args[this.currentArg];
        const caseResults = this.getCaseResults();
        const referenceResult = this.referenceResults.get(this.getCurrentCase().name)!;
        
        // Track best performers for each metric
        const bestPerformers = new Map<string, { value: number; impls: string[] }>();
        bestPerformers.set('CPU Time', { value: Infinity, impls: [] });
        for (const metric of this.suite.extraMetrics) {
            bestPerformers.set(metric.name, { 
                value: metric.mode === 'maximize' ? -Infinity : Infinity, 
                impls: [] 
            });
        }

        // Run reference implementation first
        const [refResult, refTime] = this.runImplementation(this.suite.reference, currentArgs);
        let refValid = true;

        // Store reference result
        referenceResult.results.push(refResult);
        
        // Validate reference implementation
        if (this.suite.validate && refResult !== undefined) {
            const validationResult = this.suite.validate(refResult, currentArgs, refResult);
            if (validationResult !== true) {
                referenceResult.failures.push(`Failed at arg ${this.currentArg}: ${validationResult}`);
                refValid = false;
            }
        }
        
        // Update reference metrics
        if (refValid) {
            const cpuMetric = referenceResult.metrics.get('CPU Time')!;
            cpuMetric.value += refTime;
            cpuMetric.validCount++;
            referenceResult.totalCpuTime += refTime;
            referenceResult.avgCpuTime = referenceResult.totalCpuTime / cpuMetric.validCount;

            // Set reference as initial best for CPU time
            bestPerformers.set('CPU Time', { value: refTime, impls: ['reference'] });

            // Calculate reference extra metrics
            for (const metric of this.suite.extraMetrics) {
                const value = metric.fn(refResult, currentArgs, refResult);
                const metricResult = referenceResult.metrics.get(metric.name)!;
                
                if (isFinite(value)) {  // Allow zero values
                    metricResult.value += value;
                    metricResult.validCount++;

                    // Set reference as initial best for this metric
                    bestPerformers.set(metric.name, { value, impls: ['reference'] });
                }
            }
        }

        // Run all other implementations
        for (const [implName, implFn] of this.suite.implementations.entries()) {
            const result = caseResults.get(implName)!;
            
            // Run implementation
            const [implResult, cpuUsed] = this.runImplementation(implFn, currentArgs);
            
            // Validate result
            let isValid = true;
            if (this.suite.validate && implResult !== undefined) {
                const validationResult = this.suite.validate(implResult, currentArgs, refResult);
                if (validationResult !== true) {
                    result.failures.push(`Failed at arg ${this.currentArg}: ${validationResult}`);
                    isValid = false;
                }
            }

            // Store results and update metrics only if valid
            if (isValid) {
                result.results.push(implResult);
                
                // Update CPU metrics
                const cpuMetric = result.metrics.get('CPU Time')!;
                cpuMetric.value += cpuUsed;
                cpuMetric.validCount++;
                result.totalCpuTime = cpuMetric.value;  // Update totalCpuTime to match valid CPU measurements
                result.avgCpuTime = result.totalCpuTime / cpuMetric.validCount;  // Update avgCpuTime based on valid runs only

                // Track best performers for CPU time
                const currentBest = bestPerformers.get('CPU Time')!;
                if (cpuUsed < currentBest.value) {
                    bestPerformers.set('CPU Time', { value: cpuUsed, impls: [implName] });
                } else if (cpuUsed === currentBest.value) {
                    currentBest.impls.push(implName);
                }

                // Calculate and track extra metrics
                for (const metric of this.suite.extraMetrics) {
                    const value = metric.fn(implResult, currentArgs, refResult);
                    const metricResult = result.metrics.get(metric.name)!;
                    
                    // Only include non-infinite values in averages
                    if (isFinite(value)) {
                        metricResult.value += value;
                        metricResult.validCount++;

                        const currentBest = bestPerformers.get(metric.name)!;
                        if (metric.mode === 'maximize') {
                            if (value > currentBest.value) {
                                bestPerformers.set(metric.name, { value, impls: [implName] });
                            } else if (value === currentBest.value) {
                                currentBest.impls.push(implName);
                            }
                        } else if (metric.mode === 'minimize') {
                            if (value < currentBest.value) {
                                bestPerformers.set(metric.name, { value, impls: [implName] });
                            } else if (value === currentBest.value) {
                                currentBest.impls.push(implName);
                            }
                        } else if (metric.mode === 'match') {
                            if (value === currentBest.value) {
                                currentBest.impls.push(implName);
                            }
                        }
                    }
                }
            }
        }

        // Record contest winners and increment totals for all metrics
        for (const [metricName, best] of bestPerformers) {
            // If reference is valid, only award wins to implementations that are best
            // If reference is invalid, award wins to all valid implementations
            if (!refValid) {
                // Reference failed - all valid implementations get a win for this test case
                for (const [implName, result] of caseResults) {
                    const metric = result.metrics.get(metricName)!;
                    if (result.failures.length === 0) {
                        metric.wins++;
                        metric.beatReference++;
                    }
                }
            } else {
                // Reference is valid - award wins based on metric type
                const refMetric = referenceResult.metrics.get(metricName)!;

                // Get reference value and mode
                const refValue = refMetric.value / refMetric.validCount;
                let mode = this.suite.extraMetrics.find(m => m.name === metricName)?.mode;
                const isCpuOrMinimize = metricName === 'CPU Time' || mode === 'minimize';

                // If everyone has the same value, no one gets a win
                const allValues = new Set<number>();
                allValues.add(refValue);
                for (const [implName, result] of caseResults) {
                    const metric = result.metrics.get(metricName)!;
                    const implValue = metric.value / metric.validCount;
                    if (isFinite(implValue)) {
                        allValues.add(implValue);
                    }
                }
                const everyoneTied = allValues.size === 1;

                if (!everyoneTied) {
                    // Award wins to reference if it's among best performers
                    if (best.impls.includes('reference')) {
                        refMetric.wins++;
                    }

                    for (const [implName, result] of caseResults) {
                        const metric = result.metrics.get(metricName)!;
                        const implValue = metric.value / metric.validCount;
                        
                        // Only award wins if the implementation produced a valid result
                        if (isFinite(implValue)) {
                            // Award win if among best performers
                            if (best.impls.includes(implName)) {
                                metric.wins++;
                            }

                            // Beat reference calculations
                            if (isCpuOrMinimize) {
                                if (implValue < refValue) {
                                    metric.beatReference++;
                                }
                            } else if (mode === 'maximize') {
                                if (implValue > refValue) {
                                    metric.beatReference++;
                                }
                            } else if (mode === 'match' && implValue === refValue) {
                                metric.beatReference++;
                            }
                        }
                    }
                }
            }
        }

        this.currentArg++;
        return false;
    }

    private runImplementation(impl: TupleToFunction<ArgsT, ResultT>, args: ArgsT): [ResultT, number] {
        let result: ResultT;
        let cpuUsed: number = 0;
        try {
            this.suite.beforeEach?.(args);
            cpuUsed = cpuTime(() => {
                result = impl(...args);
            }, this.suite.iterations) / this.suite.iterations;
            this.suite.afterEach?.(args);
        } catch (e) {
            console.log("error in implementation", this.suite.name, this.args[this.currentArg], e.stack);
            result = undefined as any;
        }
        
        return [result!, cpuUsed];
    }

    private getCaseResults(): Map<string, BenchmarkResult<ResultT>> {
        return this.results.get(this.getCurrentCase().name)!;
    }

    private complete() {
        this.state = 'complete';
    }

    public getResults(): Map<string, Map<string, BenchmarkResult<ResultT>>> {
        return this.results;
    }

    public displayResults(): void {
        const nameWidth = 50;
        const numWidth = 12;
        
        let failureText = '';

        console.log(`\nBenchmark Results for: ${this.suite.name}`);
        console.log('=====================================');
        
        for (const [caseName, caseResults] of this.results) {
            const numTestCases = this.caseTestCounts.get(caseName)!;
            const referenceResult = this.referenceResults.get(caseName)!;
            
            console.log(`\nCase: ${caseName} (${numTestCases} test cases, ${this.suite.iterations} iterations each)`);
            
            // Header
            let header = `${'Implementation'.padEnd(nameWidth)} | ` +
                        `${'Avg CPU'.padStart(numWidth)} | ` +
                        `${'CPU Wins'.padStart(numWidth)} | ` +
                        `${'Beat Ref'.padStart(numWidth)} | `;

            // Add extra metrics to header
            for (const metric of this.suite.extraMetrics) {
                header += `${metric.name.padStart(numWidth)} | ` +
                         `${'Wins'.padStart(numWidth)} | ` +
                         `${'Beat Ref'.padStart(numWidth)} | `;
            }

            header += 'Status';
            console.log(header);
            console.log('─'.repeat(header.length));
            
            // Sort all implementations by average CPU time
            const sortedResults = Array.from(caseResults.values());
            sortedResults.push(referenceResult);
            sortedResults.sort((a, b) => a.avgCpuTime - b.avgCpuTime);
            
            // Print all implementations
            for (const result of sortedResults) {
                const isReference = result === referenceResult;
                const status = result.failures.length === 0 ? '✓' : `✗ (${result.failures.length} failures)`;
                const cpuMetric = result.metrics.get('CPU Time')!;
                const avgCpu = cpuMetric.validCount > 0 ? (cpuMetric.value / cpuMetric.validCount).toFixed(3) : '0.000';
                const cpuWinRate = ((cpuMetric.wins / numTestCases) * 100).toFixed(1);
                
                let line = `${(isReference ? this.suite.name : result.implementationName).padEnd(nameWidth)} | ` +
                          `${avgCpu.padStart(numWidth)}cpu | ` +
                          `${cpuWinRate.padStart(numWidth)}% | ` +
                          `${isReference ? 'baseline'.padStart(numWidth) : ((cpuMetric.beatReference / numTestCases) * 100).toFixed(1).padStart(numWidth) + '%'} | `;

                // Add extra metrics
                for (const metric of this.suite.extraMetrics) {
                    const metricResult = result.metrics.get(metric.name)!;
                    const avg = metricResult.validCount > 0 ? (metricResult.value / metricResult.validCount).toFixed(3) : '0.000';
                    const winRate = ((metricResult.wins / numTestCases) * 100).toFixed(1);
                    
                    line += `${avg.padStart(numWidth)} | ` +
                           `${winRate.padStart(numWidth)}% | ` +
                           `${isReference ? 'baseline'.padStart(numWidth) : ((metricResult.beatReference / numTestCases) * 100).toFixed(1).padStart(numWidth) + '%'} | `;
                }

                line += status;
                console.log(line);

                if (result.failures.length > 0) {
                    failureText += `${result.implementationName} Failures:\n`;
                    result.failures.slice(0, 3).forEach(failure => failureText += `    - ${failure}\n`);
                }
            }
        }

        if (failureText) {
            console.log('\n\nFailures:\n' + failureText);
        }
    }
}

// Track which benchmarks have been run
const completedBenchmarks = new Set<string>();

// Export a function to run all registered benchmarks
export function runBenchmarks(tickCpuLimit: number = Game.cpu.limit * 0.8): boolean {
    let allComplete = true;
    const pendingBenchmarks = Array.from(suites.entries())
        .filter(([name]) => !completedBenchmarks.has(name));

    if (pendingBenchmarks.length === 0) {
        return true;
    }

    for (const [name, suite] of pendingBenchmarks) {
        // Get or create runner for this benchmark
        let runner = activeRunners.get(name) as BenchmarkRunner<unknown, unknown[]>;
        if (!runner) {
            console.log("creating new runner for benchmark", name);
            runner = new BenchmarkRunner(suite, tickCpuLimit);
            activeRunners.set(name, runner);
        }

        if (!runner.run()) {
            allComplete = false;
            break;  // Stop at first incomplete benchmark
        } else {
            runner.displayResults();
            completedBenchmarks.add(name);
            activeRunners.delete(name);  // Clean up completed runner
        }
    }

    return allComplete;
}

// Add a function to reset benchmark state (useful for testing)
export function resetBenchmarks() {
    completedBenchmarks.clear();
    activeRunners.clear();
}
