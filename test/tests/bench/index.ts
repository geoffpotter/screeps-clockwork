export interface BenchmarkImplementation<ResultT, ArgsT> {
    name: string;
    fn: (args: ArgsT) => ResultT;
}

export interface BenchmarkSuite<ResultT, ArgsT> {
    name: string;
    cases: BenchmarkCase<ResultT, ArgsT>[];
    implementations: BenchmarkImplementation<ResultT, ArgsT>[];
    /**
     * Validate the result of the benchmark, called once after the benchmark is complete, validates all results
     */
    validate?: (result: ResultT, referenceResult: ResultT, args: ArgsT) => boolean;
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
    valid: boolean;
}

export class Benchmark<ResultT, ArgsT> {
    public suite: BenchmarkSuite<ResultT, ArgsT>;
    private results: Map<string, Map<string, BenchmarkResult<ResultT>>>;
    private currentImpl: string;
    private currentCase: number;
    private referenceResults: ResultT[] | null = null;
    private args: ArgsT[];
    private state: 'running' | 'complete';
    private tickCpuLimit: number;
    private iterationsCompleted = 0;
    private currentResults: ResultT[] = [];
    private totalCpuUsed = 0;

    constructor(suite: BenchmarkSuite<ResultT, ArgsT>, tickCpuLimit: number = Game.cpu.limit * 0.8) {
        this.suite = suite;
        this.results = new Map();
        this.currentImpl = '';
        this.currentCase = 0;
        this.state = 'running';
        this.tickCpuLimit = tickCpuLimit;
        this.args = this.getCurrentCase().setup_args();
    }

    private getCurrentCase(): BenchmarkCase<ResultT, ArgsT> {
        return this.suite.cases[this.currentCase];
    }

    public run(): boolean {
        // Return true when complete
        if (this.state === 'complete') return true;

        const startCpu = Game.cpu.getUsed();

        while (Game.cpu.getUsed() - startCpu < this.tickCpuLimit) {
            if (this.runImplementation()) {
                return true;
            }
        }

        const impl = this.suite.implementations[this.currentImpl];
        if (impl) {
            console.log(`Done with tick - Case: ${this.getCurrentCase().benchmarkName}, Implementation: ${impl.name}`);
        }

        return false;
    }

    protected runImplementation(): boolean {
        // Handle implementation switching
        if (!this.currentImpl) {
            const nextImpl = this.suite.implementations
                .map(impl => impl.name)
                .find(implName => !this.getCaseResults().has(implName));

            if (!nextImpl) {
                // Move to next case or complete
                if (this.currentCase >= this.suite.cases.length - 1) {
                    this.complete();
                    return true;
                } else {
                    this.currentCase++;
                    this.args = this.getCurrentCase().setup_args();
                    this.referenceResults = null;
                }
            }
            this.betweenImplementations();
            this.currentImpl = nextImpl || '';
            this.iterationsCompleted = 0;
            this.currentResults = [];
            this.totalCpuUsed = 0;
        }

        const impl = this.suite.implementations.find(i => i.name === this.currentImpl);
        if (!impl) return true;
        
        const args = this.args[this.iterationsCompleted];

        // Run single iteration and measure CPU
        const startCpu = Game.cpu.getUsed();
        const result = impl.fn(args);
        this.totalCpuUsed += Game.cpu.getUsed() - startCpu;
        
        this.currentResults.push(result);
        this.iterationsCompleted++;

        // Check if we've completed all iterations for this implementation
        if (this.iterationsCompleted >= this.args.length) {
            // Store all results and average CPU time
            const result: BenchmarkResult<ResultT> = {
                implementationName: impl.name,
                results: this.currentResults,
                avgCpuTime: this.totalCpuUsed / this.args.length,
                totalCpuTime: this.totalCpuUsed,
                valid: true
            };

            // If this is the first implementation, store reference results
            if (!this.referenceResults) {
                this.referenceResults = this.currentResults;
            }

            // Validate results if needed
            if (this.suite.validate && this.referenceResults) {
                result.valid = this.currentResults.every((res, idx) => 
                    this.suite.validate!(res, this.referenceResults![idx], this.args[idx])
                );
            }

            this.getCaseResults().set(this.currentImpl, result);
            this.getCurrentCase().teardown?.();
            this.currentImpl = '';
        }

        return false;
    }

    private getCaseResults(): Map<string, BenchmarkResult<ResultT>> {
        const caseName = this.getCurrentCase().benchmarkName;
        if (!this.results.has(caseName)) {
            this.results.set(caseName, new Map());
        }
        return this.results.get(caseName)!;
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
        
        console.log(`\nBenchmark Results for: ${this.suite.name}`);
        console.log('=====================================');
        for (const [caseName, caseResults] of this.results) {
            console.log(`\nCase: ${caseName}`);
            // Header
            console.log(`${'Implementation'.padEnd(nameWidth)} | ${'Avg CPU'.padStart(numWidth)} | ${'Total CPU'.padStart(numWidth)} | Status`);
            console.log('─'.repeat(nameWidth + numWidth * 2 + 20));
            // Sort implementations by average CPU time
            const sortedResults = Array.from(caseResults.entries())
                .sort((a, b) => a[1].avgCpuTime - b[1].avgCpuTime);
            
            for (const [implName, result] of sortedResults) {
                let raw_results = (result as BenchmarkResult<RoomPosition[]>).results;
                if (raw_results) {
                    for (let res of raw_results) {
                        Game.map.visual.poly(res, {stroke: 'black'});
                    }
                }
                const status = result.valid ? '✓' : '✗';
                const avgCpu = result.avgCpuTime.toFixed(3);
                const totalCpu = result.totalCpuTime.toFixed(3);
                
                console.log(
                    `${result.implementationName.padEnd(nameWidth)} | ` +
                    `${avgCpu.padStart(numWidth)}ms | ` +
                    `${totalCpu.padStart(numWidth)}ms | ` +
                    `${status}`
                );
            }
        }
    }

    protected betweenImplementations() {
        // do nothing
    }
}



