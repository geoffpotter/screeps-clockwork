import { cpuTime } from '../../utils/cpuTime';
import './example_benchmark';

export interface BenchmarkImplementation<ResultT, ArgsT> {
    name: string;
    fn: (args: ArgsT) => ResultT;
}

export interface BenchmarkSuite<ResultT, ArgsT> {
    name: string;
    cases: BenchmarkCase<ResultT, ArgsT>[];
    implementations: BenchmarkImplementation<ResultT, ArgsT>[];
    /**
     * Validate the result of the benchmark against a reference result
     * @param result The result to validate
     * @param referenceResult The reference result to validate against
     * @param args The arguments that were used to generate this result
     * @returns true if the result is valid, or an error string describing why it's invalid
     */
    validate?: (result: ResultT, referenceResult: ResultT, args: ArgsT) => true | string;
}

export interface BenchmarkCase<ResultT, ArgsT> {
    benchmarkName: string;
    /**
     * Setup the benchmark cases, called once before the benchmark starts
     */
    setup_args: () => ArgsT[];
    /**
     * Teardown the benchmark, called once after the benchmark is complete
     */
    teardown?: () => void;
}

export interface BenchmarkResult<ResultT> {
    implementationName: string;
    results: ResultT[];
    avgCpuTime: number;
    totalCpuTime: number;
    failures: string[];
    contestWins: number;
    beatReference: number;
    totalContests: number;
}

export class Benchmark<ResultT, ArgsT> {
    public suite: BenchmarkSuite<ResultT, ArgsT>;
    private results: Map<string, Map<string, BenchmarkResult<ResultT>>>;
    private currentArg: number;
    private currentCase: number;
    private args: ArgsT[];
    private state: 'running' | 'complete';
    private tickCpuLimit: number;
    private iterations: number;

    constructor(suite: BenchmarkSuite<ResultT, ArgsT>, tickCpuLimit: number = Game.cpu.limit * 0.8, iterations: number = 3) {
        this.suite = suite;
        this.results = new Map();
        this.currentArg = 0;
        this.currentCase = 0;
        this.state = 'running';
        this.tickCpuLimit = tickCpuLimit;
        this.iterations = iterations;
        this.args = this.getCurrentCase().setup_args();
        
        // Initialize results for all implementations
        this.initializeResults();
    }

    private initializeResults() {
        const caseName = this.getCurrentCase().benchmarkName;
        if (!this.results.has(caseName)) {
            const caseResults = new Map<string, BenchmarkResult<ResultT>>();
            this.suite.implementations.forEach(impl => {
                caseResults.set(impl.name, {
                    implementationName: impl.name,
                    results: [],
                    avgCpuTime: 0,
                    totalCpuTime: 0,
                    failures: [],
                    contestWins: 0,
                    beatReference: 0,
                    totalContests: 0
                });
            });
            this.results.set(caseName, caseResults);
        }
    }

    private getCurrentCase(): BenchmarkCase<ResultT, ArgsT> {
        return this.suite.cases[this.currentCase];
    }

    public run(): boolean {
        if (this.state === 'complete') return true;

        while (Game.cpu.getUsed() < this.tickCpuLimit) {
            if (this.runContest()) {
                return true;
            }
        }

        console.log(`Done with tick - Case: ${this.getCurrentCase().benchmarkName}, Arg: ${this.currentArg + 1}/${this.args.length}`);
        return false;
    }

    protected runContest(): boolean {
        if (this.currentArg >= this.args.length) {
            if (this.currentCase >= this.suite.cases.length - 1) {
                this.complete();
                return true;
            } else {
                this.currentCase++;
                this.args = this.getCurrentCase().setup_args();
                this.currentArg = 0;
                this.initializeResults();
                console.log("switching to next case", this.getCurrentCase().benchmarkName);
            }
        }

        const currentArgs = this.args[this.currentArg];
        const caseResults = this.getCaseResults();
        let bestTime = Infinity;
        let bestImpl = '';
        let referenceTime = 0;
        let referenceResult: ResultT | null = null;

        // Run all implementations for current arg
        for (const impl of this.suite.implementations) {
            const result = caseResults.get(impl.name)!;
            
            // Run implementation with iterations
            const [implResult, cpuUsed] = this.runImplementation(impl, currentArgs);
            
            // Store results
            result.results.push(implResult);
            result.totalCpuTime += cpuUsed;
            result.avgCpuTime = result.totalCpuTime / (this.currentArg + 1);
            result.totalContests++;

            // Track best performance
            if (cpuUsed < bestTime) {
                bestTime = cpuUsed;
                bestImpl = impl.name;
            }

            // Store reference result from first implementation
            if (impl === this.suite.implementations[0]) {
                referenceTime = cpuUsed;
                referenceResult = implResult;
            } else if (referenceResult && this.suite.validate) {
                const validationResult = this.suite.validate(implResult, referenceResult, currentArgs);
                if (validationResult !== true) {
                    result.failures.push(`Failed at arg ${this.currentArg} with args ${JSON.stringify(currentArgs)}: ${validationResult}`);
                }
            }

            // Check if beat reference
            if (impl !== this.suite.implementations[0] && cpuUsed < referenceTime) {
                result.beatReference++;
            }
        }

        // Record contest winner
        if (bestImpl) {
            const winnerResult = caseResults.get(bestImpl)!;
            winnerResult.contestWins++;
        }

        this.currentArg++;
        return false;
    }

    private runImplementation(impl: BenchmarkImplementation<ResultT, ArgsT>, args: ArgsT): [ResultT, number] {
        let result: ResultT;
        const cpuUsed = cpuTime(() => {
            try {
                result = impl.fn(args);
            } catch (e) {
                console.log("error in implementation", impl.name, e);
                result = [] as any;
            }
        }, this.iterations) / this.iterations;
        
        return [result!, cpuUsed];
    }

    private getCaseResults(): Map<string, BenchmarkResult<ResultT>> {
        return this.results.get(this.getCurrentCase().benchmarkName)!;
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

        console.log(`\nBenchmark Results for: ${this.suite.name}, ${this.suite.cases.length} cases, ${this.suite.implementations.length} implementations`);
        console.log('=====================================');
        let colors = ['red', 'green', 'blue', 'yellow', 'purple', 'orange', 'pink', 'gray', 'brown', 'black'];
        let caseIdx = 0;
        
        for (const [caseName, caseResults] of this.results) {
            console.log(`\nCase: ${caseName}`);
            // Header
            console.log(
                `${'Implementation'.padEnd(nameWidth)} | ` +
                `${'Avg CPU'.padStart(numWidth)} | ` +
                `${'Total CPU'.padStart(numWidth)} | ` +
                `${'Contests Won'.padStart(numWidth)} | ` +
                `${'Beat Ref'.padStart(numWidth)} | Status`
            );
            console.log('─'.repeat(nameWidth + numWidth * 4 + 30));
            
            // Sort implementations by average CPU time
            const sortedResults = Array.from(caseResults.entries())
                .sort((a, b) => a[1].avgCpuTime - b[1].avgCpuTime);
            
            for (const [implName, result] of sortedResults) {
                let raw_results = (result as BenchmarkResult<RoomPosition[]>).results;
                if (raw_results) {
                    for (let res of raw_results) {
                        let col = colors[caseIdx % colors.length];
                        try {
                            Game.map.visual.poly(res, {stroke: col});
                        } catch (e) {
                            // console.log(e);
                        }
                    }
                }
                
                const status = result.failures.length === 0 ? '✓' : `✗ (${result.failures.length} failures)`;
                const avgCpu = result.avgCpuTime.toFixed(3);
                const totalCpu = result.totalCpuTime.toFixed(3);
                const contestWinRate = ((result.contestWins / result.totalContests) * 100).toFixed(1);
                const beatRefRate = ((result.beatReference / result.totalContests) * 100).toFixed(1);
                
                console.log(
                    `${result.implementationName.padEnd(nameWidth)} | ` +
                    `${avgCpu.padStart(numWidth)}ms | ` +
                    `${totalCpu.padStart(numWidth)}ms | ` +
                    `${contestWinRate.padStart(numWidth)}% | ` +
                    `${beatRefRate.padStart(numWidth)}% | ` +
                    `${status}`
                );

                if (result.failures.length > 0) {
                    failureText += result.implementationName + ' Failures:\n';
                    result.failures.forEach(failure => failureText += `    - ${failure}\n`);
                }
            }
            caseIdx++;
        }

        if (failureText) {
            console.log('\n\nFailures:\n' + failureText);
        }
    }

    protected betweenImplementations() {
        // do nothing
    }
}



