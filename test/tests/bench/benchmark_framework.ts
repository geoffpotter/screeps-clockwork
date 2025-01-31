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
        total: number;
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
        iterations: 3  // Default to 3 iterations if not set
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

    constructor(suite: BenchmarkSuite<ResultT, ArgsT>, tickCpuLimit: number = Game.cpu.limit * 0.8) {
        this.suite = suite;
        this.results = new Map();
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
            this.suite.implementations.forEach((_, name) => {
                const metrics = new Map();
                // Initialize CPU metric
                metrics.set('CPU Time', {
                    value: 0,
                    wins: 0,
                    beatReference: 0,
                    total: 0
                });
                // Initialize extra metrics
                for (const metric of this.suite.extraMetrics) {
                    metrics.set(metric.name, {
                        value: 0,
                        wins: 0,
                        beatReference: 0,
                        total: 0
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
        const [referenceResult, referenceTime] = this.runImplementation(this.suite.reference, currentArgs);
        
        // Calculate reference metrics
        const referenceMetrics = new Map<string, number>();
        referenceMetrics.set('CPU Time', referenceTime);
        
        // Track reference performance
        if (referenceTime < bestPerformers.get('CPU Time')!.value) {
            bestPerformers.set('CPU Time', { value: referenceTime, impls: ['reference'] });
        } else if (referenceTime === bestPerformers.get('CPU Time')!.value) {
            bestPerformers.get('CPU Time')!.impls.push('reference');
        }

        for (const metric of this.suite.extraMetrics) {
            const value = metric.fn(referenceResult, currentArgs, referenceResult);
            referenceMetrics.set(metric.name, value);
            
            const isBetter = metric.mode === 'maximize' 
                ? value > bestPerformers.get(metric.name)!.value
                : value < bestPerformers.get(metric.name)!.value;
            const isEqual = value === bestPerformers.get(metric.name)!.value;

            if (isBetter) {
                bestPerformers.set(metric.name, { value, impls: ['reference'] });
            } else if (isEqual) {
                bestPerformers.get(metric.name)!.impls.push('reference');
            }
        }

        // Run all other implementations
        for (const [implName, implFn] of this.suite.implementations.entries()) {
            const result = caseResults.get(implName)!;
            
            // Run implementation
            const [implResult, cpuUsed] = this.runImplementation(implFn, currentArgs);
            
            // Store results
            result.results.push(implResult);
            result.totalCpuTime += cpuUsed;
            result.avgCpuTime = result.totalCpuTime / (this.currentArg + 1);

            // Update CPU metric
            const cpuMetric = result.metrics.get('CPU Time')!;
            cpuMetric.value += cpuUsed;
            cpuMetric.total++;

            if (cpuUsed < bestPerformers.get('CPU Time')!.value) {
                bestPerformers.set('CPU Time', { value: cpuUsed, impls: [implName] });
            } else if (cpuUsed === bestPerformers.get('CPU Time')!.value) {
                bestPerformers.get('CPU Time')!.impls.push(implName);
            }

            if (cpuUsed < referenceMetrics.get('CPU Time')!) {
                cpuMetric.beatReference++;
            }

            // Validate result
            if (this.suite.validate) {
                const validationResult = this.suite.validate(implResult, currentArgs, referenceResult);
                if (validationResult !== true) {
                    result.failures.push(`Failed at arg ${this.currentArg}: ${validationResult}`);
                }
            }

            // Calculate and track extra metrics
            for (const metric of this.suite.extraMetrics) {
                const value = metric.fn(implResult, currentArgs, referenceResult);
                const metricResult = result.metrics.get(metric.name)!;
                metricResult.value += value;
                metricResult.total++;

                const isBetter = metric.mode === 'maximize' 
                    ? value > bestPerformers.get(metric.name)!.value
                    : value < bestPerformers.get(metric.name)!.value;
                const isEqual = value === bestPerformers.get(metric.name)!.value;

                if (isBetter) {
                    bestPerformers.set(metric.name, { value, impls: [implName] });
                } else if (isEqual) {
                    bestPerformers.get(metric.name)!.impls.push(implName);
                }

                const referenceValue = referenceMetrics.get(metric.name)!;
                const beatReference = metric.mode === 'maximize'
                    ? value > referenceValue
                    : value < referenceValue;

                if (beatReference) {
                    metricResult.beatReference++;
                }
            }
        }

        // Record contest winners for all metrics
        for (const [metricName, best] of bestPerformers) {
            // Award wins to all implementations that tied for best
            for (const winner of best.impls) {
                if (winner === 'reference') {
                    // Update reference wins in all implementations' metrics
                    for (const result of caseResults.values()) {
                        const metric = result.metrics.get(metricName)!;
                        metric.total++;
                    }
                } else {
                    const winnerResult = caseResults.get(winner)!;
                    winnerResult.metrics.get(metricName)!.wins++;
                }
            }
            // Always increment total count
            for (const result of caseResults.values()) {
                const metric = result.metrics.get(metricName)!;
                if (!best.impls.includes('reference') && !best.impls.includes(result.implementationName)) {
                    metric.total++;
                }
            }
        }

        this.currentArg++;
        return false;
    }

    private runImplementation(impl: TupleToFunction<ArgsT, ResultT>, args: ArgsT): [ResultT, number] {
        let result: ResultT;
        const cpuUsed = cpuTime(() => {
            try {
                this.suite.beforeEach?.(args);
                result = impl(...args);
                this.suite.afterEach?.(args);
            } catch (e) {
                console.log("error in implementation", e);
                result = undefined as any;
            }
        }, this.suite.iterations) / this.suite.iterations;
        
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
            
            // Create reference result entry
            let referenceResult: BenchmarkResult<ResultT> | undefined;
            
            // Run reference implementation on first test case to get metrics
            if (this.currentArg > 0) {
                const firstArgs = this.args[0];
                const [_, refCpuTime] = this.runImplementation(this.suite.reference, firstArgs);
                
                // Calculate reference metrics
                const metrics = new Map();
                metrics.set('CPU Time', {
                    value: refCpuTime,
                    wins: caseResults.values().next().value.metrics.get('CPU Time')!.total - 
                          Array.from(caseResults.values()).reduce((sum, r) => sum + r.metrics.get('CPU Time')!.wins, 0),
                    beatReference: 0,
                    total: caseResults.values().next().value.metrics.get('CPU Time')!.total
                });

                // Calculate reference extra metrics
                const [refResult] = this.runImplementation(this.suite.reference, firstArgs);
                for (const metric of this.suite.extraMetrics) {
                    const value = metric.fn(refResult, firstArgs, refResult);
                    metrics.set(metric.name, {
                        value,
                        wins: caseResults.values().next().value.metrics.get(metric.name)!.total - 
                              Array.from(caseResults.values()).reduce((sum, r) => sum + r.metrics.get(metric.name)!.wins, 0),
                        beatReference: 0,
                        total: caseResults.values().next().value.metrics.get(metric.name)!.total
                    });
                }

                referenceResult = {
                    implementationName: this.suite.name,
                    results: [],
                    avgCpuTime: refCpuTime,
                    totalCpuTime: refCpuTime,
                    failures: [],
                    metrics
                };
            }
            
            // Sort all implementations including reference by average CPU time
            const sortedResults = Array.from(caseResults.entries())
                .map(([name, result]) => result);
            if (referenceResult) {
                sortedResults.push(referenceResult);
            }
            sortedResults.sort((a, b) => a.avgCpuTime - b.avgCpuTime);
            
            // Print all implementations
            for (const result of sortedResults) {
                const isReference = result === referenceResult;
                const status = result.failures.length === 0 ? '✓' : `✗ (${result.failures.length} failures)`;
                const cpuMetric = result.metrics.get('CPU Time')!;
                const avgCpu = result.avgCpuTime.toFixed(3);
                const cpuWinRate = ((cpuMetric.wins / cpuMetric.total) * 100).toFixed(1);
                
                let line = `${result.implementationName.padEnd(nameWidth)} | ` +
                          `${avgCpu.padStart(numWidth)}cpu | ` +
                          `${cpuWinRate.padStart(numWidth)}% | ` +
                          `${isReference ? 'baseline'.padStart(numWidth) : ((cpuMetric.beatReference / cpuMetric.total) * 100).toFixed(1).padStart(numWidth) + '%'} | `;

                // Add extra metrics
                for (const metric of this.suite.extraMetrics) {
                    const metricResult = result.metrics.get(metric.name)!;
                    const avg = (metricResult.value / metricResult.total).toFixed(2);
                    const winRate = ((metricResult.wins / metricResult.total) * 100).toFixed(1);
                    
                    line += `${avg.padStart(numWidth)} | ` +
                           `${winRate.padStart(numWidth)}% | ` +
                           `${isReference ? 'baseline'.padStart(numWidth) : ((metricResult.beatReference / metricResult.total) * 100).toFixed(1).padStart(numWidth) + '%'} | `;
                }

                line += status;
                console.log(line);

                if (result.failures.length > 0) {
                    failureText += `${result.implementationName} Failures:\n`;
                    result.failures.forEach(failure => failureText += `    - ${failure}\n`);
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

    console.log("running", pendingBenchmarks.length, "benchmarks");
    for (const [name, suite] of pendingBenchmarks) {
        console.log("running benchmark", name);
        const runner = new BenchmarkRunner(suite, tickCpuLimit);
        if (!runner.run()) {
            allComplete = false;
            break;  // Stop at first incomplete benchmark
        } else {
            runner.displayResults();
            completedBenchmarks.add(name);
        }
    }
    return allComplete;
}
