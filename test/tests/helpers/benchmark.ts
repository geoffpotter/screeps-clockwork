import { cpuTime } from '../../utils/cpuTime';

// Core types for the benchmarking system
export type MetricMode = 'minimize' | 'maximize' | 'match';
export type ValidationResult = true | string;
export type GradingFunction<TResult, TInput> = (result: TResult, input: TInput, reference: TResult | undefined) => number;
export type ValidationFunction<TResult, TInput> = (result: TResult, input: TInput, reference: TResult | undefined) => ValidationResult;

// Statistical analysis functions
interface Statistics {
  mean: number;
  stdDev: number;
  p50: number;
  p95: number;
  p99: number;
  confidenceInterval95: [number, number];
}

function calculateStatistics(values: number[]): Statistics {
  if (values.length === 0) {
    const empty = { mean: 0, stdDev: 0, p50: 0, p95: 0, p99: 0, confidenceInterval95: [0, 0] as [number, number] };
    return empty;
  }

  const sorted = [...values].sort((a, b) => a - b);
  const mean = values.reduce((sum, v) => sum + v, 0) / values.length;
  
  // Standard deviation
  const variance = values.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / values.length;
  const stdDev = Math.sqrt(variance);
  
  // Percentiles
  const getPercentile = (p: number) => {
    const index = Math.ceil(sorted.length * p / 100) - 1;
    return sorted[Math.max(0, Math.min(index, sorted.length - 1))];
  };
  
  const p50 = getPercentile(50);
  const p95 = getPercentile(95);
  const p99 = getPercentile(99);
  
  // 95% confidence interval (assuming normal distribution)
  const standardError = stdDev / Math.sqrt(values.length);
  const marginOfError = 1.96 * standardError; // 1.96 for 95% CI
  const confidenceInterval95: [number, number] = [mean - marginOfError, mean + marginOfError];
  
  return { mean, stdDev, p50, p95, p99, confidenceInterval95 };
}

interface MetricDefinition<TResult, TInput> {
  name: string;
  grade: GradingFunction<TResult, TInput>;
  mode: MetricMode;
}

interface TestCase<TInput> {
  name: string;
  generate: () => TInput[];
}

interface Implementation<TResult, TInput> {
  name: string;
  fn: (input: TInput) => TResult;
}

interface BenchmarkResults<TResult> {
  implementationName: string;
  avgCpuTime: number;
  totalCpuTime: number;
  cpuTimes: number[]; // Individual CPU measurements for statistical analysis
  successCount: number;
  totalAttempts: number;
  failures: string[];
  metrics: Map<string, {
    values: number[];
    avgValue: number;
    winsVsReference: number;
    winsVsOthers: number;
    timesUnbeaten: number; // For reference: times no implementation beat/matched it
  }>;
}

interface CaseResults<TResult> {
  caseName: string;
  testCount: number;
  implementations: Map<string, BenchmarkResults<TResult>>;
  reference: BenchmarkResults<TResult>;
}

/**
 * Main Benchmark class with fluent API for easier setup
 */
export class Benchmark<TResult, TInput = any> {
  private name: string;
  private referenceFn?: (input: TInput) => TResult;
  private referenceName?: string;
  private implementations: Implementation<TResult, TInput>[] = [];
  private testCases: TestCase<TInput>[] = [];
  private metrics: MetricDefinition<TResult, TInput>[] = [];
  private validationFn?: ValidationFunction<TResult, TInput>;
  private iterationsCount = 1;
  private repeatsCount = 1;
  private warmupRounds = 0;
  private beforeEachFn?: (input: TInput) => void;
  private afterEachFn?: (input: TInput) => void;
  private tickCpuLimit = 0.8;
  private runner?: BenchmarkRunner<TResult, TInput>;

  constructor(name: string) {
    this.name = name;
  }

  /**
   * Set the reference implementation that other implementations will be compared against
   */
  reference(name: string, fn: (input: TInput) => TResult): this {
    this.referenceName = name;
    this.referenceFn = fn;
    return this;
  }

  /**
   * Add an implementation to benchmark
   */
  implement(name: string, fn: (input: TInput) => TResult): this {
    this.implementations.push({ name, fn });
    return this;
  }

  /**
   * Add a test case that generates input data
   */
  testCase(name: string, generate: () => TInput[]): this {
    this.testCases.push({ name, generate });
    return this;
  }

  /**
   * Set validation function to check if results are correct
   */
  validate(fn: ValidationFunction<TResult, TInput>): this {
    this.validationFn = fn;
    return this;
  }

  /**
   * Add a metric to measure (minimize mode - lower is better)
   */
  minimize(name: string, grade: GradingFunction<TResult, TInput>): this {
    this.metrics.push({ name, grade, mode: 'minimize' });
    return this;
  }

  /**
   * Add a metric to measure (maximize mode - higher is better)
   */
  maximize(name: string, grade: GradingFunction<TResult, TInput>): this {
    this.metrics.push({ name, grade, mode: 'maximize' });
    return this;
  }

  /**
   * Add a metric to measure (match mode - closer to reference is better)
   */
  match(name: string, grade: GradingFunction<TResult, TInput>): this {
    this.metrics.push({ name, grade, mode: 'match' });
    return this;
  }

  /**
   * Set number of iterations to run each test (for averaging)
   */
  iterations(count: number): this {
    this.iterationsCount = count;
    return this;
  }

  /**
   * Set function to run before each test
   */
  beforeEach(fn: (input: TInput) => void): this {
    this.beforeEachFn = fn;
    return this;
  }

  /**
   * Set function to run after each test
   */
  afterEach(fn: (input: TInput) => void): this {
    this.afterEachFn = fn;
    return this;
  }

  /**
   * Set the number of repeats per CPU measurement (default: 1)
   * Higher repeats help measure very fast functions more accurately
   * CPU is measured across all repeats, then divided by iterations
   */
  repeats(count: number): this {
    this.repeatsCount = count;
    return this;
  }

  /**
   * Set the number of warmup rounds to run before actual measurements (default: 0)
   * Warmup rounds help stabilize performance by allowing JIT compilation and cache warming
   * Warmup runs are not counted toward final results
   */
  warmup(rounds: number): this {
    this.warmupRounds = rounds;
    return this;
  }

  /**
   * Set CPU limit for tick-based execution (as fraction of Game.cpu.limit)
   * Benchmarks will only run when total tick CPU usage is below this threshold
   */
  cpuLimit(fraction: number): this {
    this.tickCpuLimit = fraction;
    return this;
  }

  /**
   * Run the benchmark (non-blocking, spreads across multiple ticks)
   * Returns true when complete, false when still running
   */
  run(): boolean {
    if (!this.referenceFn) {
      throw new Error('Reference function is required');
    }
    if (this.implementations.length === 0) {
      throw new Error('At least one implementation is required');
    }
    if (this.testCases.length === 0) {
      throw new Error('At least one test case is required');
    }

    if (!this.runner) {
      this.runner = new BenchmarkRunner({
        name: this.name,
        referenceFn: this.referenceFn,
        referenceName: this.referenceName || 'reference',
        implementations: this.implementations,
        testCases: this.testCases,
        metrics: this.metrics,
        validationFn: this.validationFn,
        iterations: this.iterationsCount,
        repeats: this.repeatsCount,
        warmupRounds: this.warmupRounds,
        beforeEachFn: this.beforeEachFn,
        afterEachFn: this.afterEachFn,
        tickCpuLimit: this.tickCpuLimit
      });
    }

    return this.runner.run();
  }

  /**
   * Get results (only valid after run() returns true)
   */
  getResults(): CaseResults<TResult>[] {
    if (!this.runner) {
      throw new Error('Benchmark has not been run yet');
    }
    return this.runner.getResults();
  }

  /**
   * Display results in console
   */
  displayResults(): void {
    if (!this.runner) {
      throw new Error('Benchmark has not been run yet');
    }
    this.runner.displayResults();
  }
}

/**
 * Internal runner class that handles the actual benchmark execution
 */
class BenchmarkRunner<TResult, TInput> {
  private config: {
    name: string;
    referenceFn: (input: TInput) => TResult;
    referenceName: string;
    implementations: Implementation<TResult, TInput>[];
    testCases: TestCase<TInput>[];
    metrics: MetricDefinition<TResult, TInput>[];
    validationFn?: ValidationFunction<TResult, TInput>;
    iterations: number;
    repeats: number;
    warmupRounds: number;
    beforeEachFn?: (input: TInput) => void;
    afterEachFn?: (input: TInput) => void;
    tickCpuLimit: number;
  };
  
  private state: {
    currentCaseIndex: number;
    currentInputIndex: number;
    currentInputs: TInput[];
    results: CaseResults<TResult>[];
    complete: boolean;
  };

  private getCpuUsed: () => number;
  private logger: (...args: any[]) => void;

  constructor(config: BenchmarkRunner<TResult, TInput>['config'], options?: {
    getCpuUsed?: () => number;
    logger?: (...args: any[]) => void;
  }) {
    this.config = config;
    this.getCpuUsed = options?.getCpuUsed || (() => Game?.cpu?.getUsed() || 0);
    this.logger = options?.logger || console.log;
    
    this.state = {
      currentCaseIndex: 0,
      currentInputIndex: 0,
      currentInputs: [],
      results: [],
      complete: false
    };

    this.initializeNextCase();
  }

  private initializeNextCase(): void {
    if (this.state.currentCaseIndex >= this.config.testCases.length) {
      this.state.complete = true;
      return;
    }

    const testCase = this.config.testCases[this.state.currentCaseIndex];
    this.state.currentInputs = testCase.generate();
    this.state.currentInputIndex = 0;

    // Initialize results for this case
    const caseResult: CaseResults<TResult> = {
      caseName: testCase.name,
      testCount: this.state.currentInputs.length,
      implementations: new Map(),
      reference: this.createEmptyResults(this.config.referenceName)
    };

    // Initialize implementation results
    for (const impl of this.config.implementations) {
      caseResult.implementations.set(impl.name, this.createEmptyResults(impl.name));
    }

    this.state.results.push(caseResult);
  }

  private createEmptyResults(name: string): BenchmarkResults<TResult> {
    const metrics = new Map<string, { values: number[]; avgValue: number; winsVsReference: number; winsVsOthers: number; timesUnbeaten: number; }>();
    
    // Add CPU time metric
    metrics.set('CPU Time', { values: [], avgValue: 0, winsVsReference: 0, winsVsOthers: 0, timesUnbeaten: 0 });
    
    // Add custom metrics
    for (const metric of this.config.metrics) {
      metrics.set(metric.name, { values: [], avgValue: 0, winsVsReference: 0, winsVsOthers: 0, timesUnbeaten: 0 });
    }

    return {
      implementationName: name,
      avgCpuTime: 0,
      totalCpuTime: 0,
      cpuTimes: [],
      successCount: 0,
      totalAttempts: 0,
      failures: [],
      metrics
    };
  }

  run(): boolean {
    if (this.state.complete) return true;

    const cpuLimit = Game?.cpu?.limit || 100;
    const maxAllowedCpuUsage = cpuLimit * this.config.tickCpuLimit;

    // Check if we should even start running benchmarks this tick
    if (this.getCpuUsed() >= maxAllowedCpuUsage) {
      return false; // Skip this tick, too much CPU already used
    }

    while (this.getCpuUsed() < maxAllowedCpuUsage) {
      // Check global CPU usage before each test to avoid timeout
      if (this.getCpuUsed() > maxAllowedCpuUsage * 0.8) { // Leave buffer for cleanup
        break;
      }

      if (this.runNextTest()) {
        return true; // Benchmark complete
      }

      // Safety check - if we've exceeded our CPU budget, break
      if (this.getCpuUsed() >= maxAllowedCpuUsage) {
        break;
      }
    }

    // Log progress
    const currentCase = this.config.testCases[this.state.currentCaseIndex];
    const cpuUsed = this.getCpuUsed().toFixed(3);
    this.logger(`[Benchmark Progress] '${this.config.name}' - Case '${currentCase.name}' (${this.state.currentCaseIndex + 1}/${this.config.testCases.length}) - Test ${this.state.currentInputIndex + 1}/${this.state.currentInputs.length} - Total CPU: ${cpuUsed}/${cpuLimit.toFixed(0)}`);
    
    return false;
  }

  private runNextTest(): boolean {
    if (this.state.currentInputIndex >= this.state.currentInputs.length) {
      // Move to next case
      this.state.currentCaseIndex++;
      this.initializeNextCase();
      
      if (this.state.complete) {
        this.finalizeResults();
        return true;
      }
      return false;
    }

    const input = this.state.currentInputs[this.state.currentInputIndex];
    const caseResult = this.state.results[this.state.currentCaseIndex];
    
    // Create list of all implementations (including reference) and shuffle for this test value
    const allImplementations = [
      { name: this.config.referenceName, fn: this.config.referenceFn, isReference: true },
      ...this.config.implementations.map(impl => ({ name: impl.name, fn: impl.fn, isReference: false }))
    ];
    
    // Shuffle implementations for each test value to avoid order bias
    const shuffledImplementations = [...allImplementations].sort(() => Math.random() - 0.5);
    
    const allResults: Array<{ name: string; result: TResult | undefined; cpuTime: number; valid: boolean; error?: string; isReference: boolean }> = [];
    let refResult: TResult | undefined;
    
    // Run all implementations in randomized order
    for (const impl of shuffledImplementations) {
      const [implResult, implCpuTime, implValid, implError] = this.runSingleImplementation(
        impl.fn, input, impl.name
      );
      
      allResults.push({ 
        name: impl.name, 
        result: implResult, 
        cpuTime: implCpuTime, 
        valid: implValid, 
        error: implError,
        isReference: impl.isReference
      });
      
      // Capture reference result for validation purposes
      if (impl.isReference) {
        refResult = implResult;
      }
    }
    
    // Find reference result details for metrics calculation
    const referenceResultData = allResults.find(r => r.isReference)!;
    let refValid = referenceResultData.valid;
    let refCpuTime = referenceResultData.cpuTime;
    let refError = referenceResultData.error;
    
    // Re-validate all implementations now that we have the reference result
    for (const resultData of allResults) {
      if (resultData.result !== undefined && this.config.validationFn) {
        const refForValidation = resultData.isReference ? undefined : refResult;
        const validation = this.config.validationFn(resultData.result, input, refForValidation);
        if (validation !== true) {
          resultData.valid = false;
          resultData.error = resultData.error ? `${resultData.error}; Validation: ${validation}` : `Validation failed: ${validation}`;
          
          // Update reference data if it's the reference implementation
          if (resultData.isReference) {
            refValid = false;
            refError = resultData.error;
          }
        }
      }
    }
    
    // Record results for each implementation individually (success or failure)
    this.recordResult(caseResult.reference, refResult, refCpuTime, refValid, refError, input, refResult);
    
    const validImplementationResults: Array<{ name: string; result: TResult; cpuTime: number; valid: boolean }> = [];
    
    // Record results for reference if it succeeded
    if (refValid && refResult !== undefined) {
      validImplementationResults.push({ name: this.config.referenceName, result: refResult, cpuTime: refCpuTime, valid: true });
    }
    
    // Record results for all other implementations
    for (const resultData of allResults) {
      if (!resultData.isReference) {
        const benchResult = caseResult.implementations.get(resultData.name)!;
        this.recordResult(benchResult, resultData.result, resultData.cpuTime, resultData.valid, resultData.error, input, refResult);
        
        if (resultData.valid && resultData.result !== undefined) {
          validImplementationResults.push({ name: resultData.name, result: resultData.result, cpuTime: resultData.cpuTime, valid: true });
        }
      }
    }
    
    // Calculate wins only among implementations that succeeded in this test
    if (validImplementationResults.length > 1) {
      this.calculateWins(caseResult, refResult, refCpuTime, refValid, validImplementationResults, input);
    }
    
    this.state.currentInputIndex++;
    return false;
  }

  private runSingleImplementation(
    fn: (input: TInput) => TResult,
    input: TInput,
    name: string
  ): [TResult | undefined, number, boolean, string | undefined] {
    let result: TResult | undefined;
    let cpuTimeValue = 0;
    let error: string | undefined;

    try {
      this.config.beforeEachFn?.(input);
      
      // Run warmup rounds (not measured)
      if (this.config.warmupRounds > 0) {
        for (let warmup = 0; warmup < this.config.warmupRounds; warmup++) {
          for (let i = 0; i < this.config.repeats; i++) {
            fn(input); // Warmup run - result ignored
          }
        }
      }
      
      // Run actual measured rounds
      cpuTimeValue = cpuTime(() => {
        for (let i = 0; i < this.config.repeats; i++) {
          result = fn(input);
        }
      });
      
      this.config.afterEachFn?.(input);
    } catch (e) {
      error = `Runtime error: ${e instanceof Error ? e.message : String(e)}`;
    }

    let valid = true;
    if (result !== undefined && this.config.validationFn) {
      // Note: Validation will be done after all implementations run with proper reference result
      // For now, just do basic validation without reference comparison
      const validation = this.config.validationFn(result, input, undefined);
      if (validation !== true) {
        valid = false;
        error = error ? `${error}; Validation: ${validation}` : `Validation failed: ${validation}`;
      }
    }

    return [result, cpuTimeValue, valid, error];
  }

  private recordResult(
    benchResult: BenchmarkResults<TResult>,
    result: TResult | undefined,
    cpuTime: number,
    valid: boolean,
    error: string | undefined,
    input: TInput,
    referenceResult: TResult | undefined
  ): void {
    benchResult.totalAttempts++;
    
    if (error) {
      benchResult.failures.push(`Input ${this.state.currentInputIndex}: ${error}`);
    }
    
    // Since we now skip failed test cases entirely, we only record successful results
    if (valid && result !== undefined) {
      benchResult.successCount++;
      benchResult.totalCpuTime += cpuTime;
      benchResult.avgCpuTime = benchResult.totalCpuTime / benchResult.successCount;
      benchResult.cpuTimes.push(cpuTime);
      
      // Record CPU time
      benchResult.metrics.get('CPU Time')!.values.push(cpuTime);
      
      // Record custom metrics
      for (const metric of this.config.metrics) {
        try {
          const value = metric.grade(result, input, referenceResult!);
          if (isFinite(value)) {
            benchResult.metrics.get(metric.name)!.values.push(value);
          }
        } catch (e) {
          // Ignore metric calculation errors
        }
      }
    }
    // Note: Failed implementations now record failures but continue participating in benchmarks
  }

  private calculateWins(
    caseResult: CaseResults<TResult>,
    referenceResult: TResult | undefined,
    referenceCpuTime: number,
    referenceValid: boolean,
    implementationResults: Array<{ name: string; result: TResult; cpuTime: number; valid: boolean }>,
    input: TInput
  ): void {
    if (!referenceValid) return;
    
    // Calculate wins for each metric
    const allMetrics = ['CPU Time', ...this.config.metrics.map(m => m.name)];
    
    for (const metricName of allMetrics) {
      const isCustomMetric = metricName !== 'CPU Time';
      const metricDef = this.config.metrics.find(m => m.name === metricName);
      const mode = isCustomMetric ? metricDef!.mode : 'minimize';
      
      // Get values for this metric - include reference in competition
      const referenceValue = isCustomMetric 
        ? (referenceResult ? metricDef!.grade(referenceResult, input, referenceResult) : 0)
        : referenceCpuTime;
      
      const allValues: Array<{ name: string; value: number; isReference: boolean }> = [];
      
      // Add reference value to competition
      if (isFinite(referenceValue)) {
        allValues.push({ name: this.config.referenceName, value: referenceValue, isReference: true });
      }
      
      // Add implementation values
      for (const implResult of implementationResults) {
        if (!implResult.result) continue; // Skip undefined results
        
        // Skip if this is the reference implementation (it's already added above)
        if (implResult.name === this.config.referenceName) continue;
        
        const value = isCustomMetric
          ? metricDef!.grade(implResult.result, input, referenceResult)
          : implResult.cpuTime;
        
        if (isFinite(value)) {
          allValues.push({ name: implResult.name, value, isReference: false });
        }
      }
      let isReferenceWinner = false;
      // Find winners among ALL participants (including reference)
      if (allValues.length > 1) {
        const winners = this.findWinners(allValues.map(v => ({ name: v.name, value: v.value })), mode, referenceValue);
        // Award wins to ALL winners (including reference if it wins)
        for (const winnerName of winners) {
          isReferenceWinner = winnerName === this.config.referenceName;
          
          if (isReferenceWinner) {
            // Reference won - increment its wins vs others
            caseResult.reference.metrics.get(metricName)!.winsVsOthers++;
          } else {
            // Implementation won
            const benchResult = caseResult.implementations.get(winnerName)!;
            benchResult.metrics.get(metricName)!.winsVsOthers++;
          }
        }
        
        // Check ALL implementations against reference for "Beat Ref %" (separate from winning)
        let anyBeatReference = false;
        for (const { name, value } of allValues) {
          if (name !== this.config.referenceName) {
            if (this.beatsReference(value, referenceValue, mode)) {
              const benchResult = caseResult.implementations.get(name)!;
              benchResult.metrics.get(metricName)!.winsVsReference++;
              anyBeatReference = true;
            }
          }
        }
        
        // If no implementation beat the reference, increment reference's timesUnbeaten
        if (!anyBeatReference && isReferenceWinner) {
          caseResult.reference.metrics.get(metricName)!.timesUnbeaten++;
        }
      }
    }
  }

  private findWinners(values: Array<{ name: string; value: number }>, mode: MetricMode, referenceValue?: number): string[] {
    if (values.length === 0) return [];
    
    if (mode === 'minimize') {
      const minValue = Math.min(...values.map(v => v.value));
      const winners = values.filter(v => v.value === minValue).map(v => v.name)
      return winners;
    } else if (mode === 'maximize') {
      const maxValue = Math.max(...values.map(v => v.value));
      return values.filter(v => v.value === maxValue).map(v => v.name);
    } else { // match
      if (referenceValue === undefined) throw new Error('Reference value required for match mode');
      const winners = values.filter(v => v.value === referenceValue).map(v => v.name)
      return winners;
    }
  }

  private beatsReference(value: number, referenceValue: number, mode: MetricMode): boolean {
    if (mode === 'minimize') {
      return value < referenceValue; // Strict inequality - only count actual improvements
    } else if (mode === 'maximize') {
      return value > referenceValue; // Strict inequality - only count actual improvements  
    } else { // match
      return value === referenceValue; // Exact match counts as "beating" for match mode
    }
  }

  private finalizeResults(): void {
    // Calculate average values for all metrics
    for (const caseResult of this.state.results) {
      this.finalizeResultsForCase(caseResult.reference);
      for (const [, implResult] of caseResult.implementations) {
        this.finalizeResultsForCase(implResult);
      }
    }
  }

  private finalizeResultsForCase(result: BenchmarkResults<TResult>): void {
    for (const [, metric] of result.metrics) {
      if (metric.values.length > 0) {
        metric.avgValue = metric.values.reduce((a, b) => a + b, 0) / metric.values.length;
      }
    }
  }

  getResults(): CaseResults<TResult>[] {
    return this.state.results;
  }

  displayResults(): void {
    let html = `<style>
      table { border-collapse: collapse; font-family: monospace; font-size: 12px; width: 100%; }
      table th, table td { border: 1px solid #666; }
      table td { padding: 5px 5px; text-align: right;}
      table th { padding: 5px 5px; background-color: #333; color: #fff; font-weight: bold; text-align: center;}
      table td:first-child { text-align: left; }
      .benchmark-title { color: #00ff00; font-weight: bold; font-size: 13px; margin-bottom: 10px; }
      .case-title { color: #ffff00; font-weight: bold; font-size: 12px; margin: 15px 0 5px 0; }
      .case-info { color: #888; font-size: 11px; margin-bottom: 10px; }
      .failures { color: #ff6666; font-weight: bold; margin-top: 12px; }
      .failure-item { color: #ffaa00; margin-left: 10px; }
      .failure-detail { color: #ff6666; margin-left: 20px; font-size: 11px; }
    </style>`;
    html = html.split('\n').join('');
    console.log(html + "<table><tr><th>Implementation</th><th>Avg CPU</th><th>CPU P50</th><th>CPU P95</th><th>StdDev</th><th>Wins %</th><th>Beat %</th></tr><tr><td>test</td><td>1</td><td>2</td><td>3</td><td>4</td><td>5</td><td>6</td></tr></table>");
    console.log(html.replace("<", "{"));
    html += `<div class="benchmark-title">📊 Benchmark Results: ${this.config.name}</div>`;
    
    for (const caseResult of this.state.results) {
      const repeatsText = this.config.repeats > 1 ? `, ${this.config.repeats} repeats` : '';
      const warmupText = this.config.warmupRounds > 0 ? `, ${this.config.warmupRounds} warmup` : '';
      
      html += `<div class="case-title">📋 Case: ${caseResult.caseName}</div>`;
      html += `<div class="case-info">(${caseResult.testCount} tests, ${this.config.iterations} iterations${repeatsText}${warmupText})</div>`;
      
      // Start table
      html += '<table>';
      
      // Build header
      html += '<tr><th>Implementation</th><th>CPU Avg/P50/P95/StdDev</th><th>CPU vs Ref</th><th>CPU Win%(Beat%)</th>';
      for (const metric of this.config.metrics) {
        html += `<th>${metric.name} Win%(Beat%)</th>`;
      }
      html += '<th>Status</th></tr>';
      
      // Sort results by CPU win percentage (descending), then by average CPU time
      const allResults = [caseResult.reference, ...Array.from(caseResult.implementations.values())];
      allResults.sort((a, b) => {
        if (a.failures.length !== b.failures.length) {
          return a.failures.length - b.failures.length;
        }
        
        // Calculate CPU win rates for sorting
        const aSuccessRate = a.totalAttempts > 0 ? (a.successCount / a.totalAttempts) : 0;
        const bSuccessRate = b.totalAttempts > 0 ? (b.successCount / b.totalAttempts) : 0;
        
        const aCpuMetric = a.metrics.get('CPU Time')!;
        const bCpuMetric = b.metrics.get('CPU Time')!;
        
        const aCpuWinRate = aSuccessRate > 0 ? (aCpuMetric.winsVsOthers / a.successCount) : 0;
        const bCpuWinRate = bSuccessRate > 0 ? (bCpuMetric.winsVsOthers / b.successCount) : 0;
        
        // Sort by CPU win rate (descending - higher win rate first)
        if (bCpuWinRate !== aCpuWinRate) {
          return bCpuWinRate - aCpuWinRate;
        }
        
        // If win rates are equal, sort by average CPU time (ascending - faster first)
        return a.avgCpuTime - b.avgCpuTime;
      });
      
      // Calculate performance ranges for smart coloring
      const performanceRanges = this.calculatePerformanceRanges(allResults, caseResult);
      
      // Add data rows
      for (const result of allResults) {
        html += this.buildTableRow(result, caseResult, performanceRanges);
      }
      
      html += '</table>';
      
      // Show failures if any
      const failedResults = allResults.filter(r => r.failures.length > 0);
      if (failedResults.length > 0) {
        html += '<div class="failures">⚠️  Failures:</div>';
        for (const result of failedResults) {
          html += `<div class="failure-item">${result.implementationName}:</div>`;
          result.failures.slice(0, 3).forEach(failure => {
            html += `<div class="failure-detail">${failure}</div>`;
          });
          if (result.failures.length > 3) {
            html += `<div class="failure-detail">... and ${result.failures.length - 3} more</div>`;
          }
        }
      }
    }
    
    this.logger(html);
  }

  public calculatePerformanceRanges(allResults: BenchmarkResults<TResult>[], caseResult: CaseResults<TResult>) {
    // Get valid results (no failures) for baseline calculations
    const validResults = allResults.filter(r => r.failures.length === 0);
    if (validResults.length === 0) return null;

    // Calculate ranges for CPU time
    const cpuTimes = validResults.map(r => calculateStatistics(r.cpuTimes).p50);
    const cpuMin = Math.min(...cpuTimes);
    const cpuMax = Math.max(...cpuTimes);

    // Calculate ranges for custom metrics
    const metricRanges = new Map<string, { min: number; max: number; mode: MetricMode }>();
    
    for (const metric of this.config.metrics) {
      const values = validResults.map(r => {
        const stats = calculateStatistics(r.metrics.get(metric.name)!.values);
        return stats.p50;
      }).filter(v => isFinite(v));
      
      if (values.length > 0) {
        metricRanges.set(metric.name, {
          min: Math.min(...values),
          max: Math.max(...values),
          mode: metric.mode
        });
      }
    }

    return {
      cpu: { min: cpuMin, max: cpuMax },
      metrics: metricRanges
    };
  }

  private getPerformanceColor(value: number, min: number, max: number, mode: MetricMode = 'minimize'): string {
    if (!isFinite(value) || !isFinite(min) || !isFinite(max) || min === max) {
      return '#888888'; // Gray for invalid/indeterminate values
    }

    // Normalize value to 0-1 range
    const normalized = (value - min) / (max - min);
    
    // For minimize mode (like CPU time, path cost), lower is better
    // For maximize mode (like accuracy), higher is better  
    // For match mode, closer to reference (min) is better
    const performance = mode === 'maximize' ? normalized : (1 - normalized);
    
    // Smart color thresholds
    if (performance >= 0.95) return '#00ff00';      // Bright green: best or very close to best
    if (performance >= 0.85) return '#88ff00';      // Green-yellow: very good
    if (performance >= 0.70) return '#ffff00';      // Yellow: good
    if (performance >= 0.50) return '#ffaa00';      // Orange: mediocre
    if (performance >= 0.30) return '#ff6600';      // Red-orange: poor
    return '#ff0000';                               // Red: very poor
  }

  public buildTableRow(
    result: BenchmarkResults<TResult>,
    caseResult: CaseResults<TResult>,
    performanceRanges: any
  ): string {
    const isReference = result === caseResult.reference;
    const cpuStats = calculateStatistics(result.cpuTimes);
    // Removed successRate variable as it's no longer used after win rate calculation fix
    
    // Calculate win percentages - use total test count for normalization
    const cpuMetric = result.metrics.get('CPU Time')!;
    const cpuWinRate = caseResult.testCount > 0 ? Math.min(100, ((cpuMetric.winsVsOthers / caseResult.testCount) * 100)).toFixed(0) : '0';
    const cpuBeatRefRate = isReference ? 
      (caseResult.testCount > 0 ? Math.min(100, ((cpuMetric.timesUnbeaten / caseResult.testCount) * 100)).toFixed(0) + '%' : '0%') :
      (caseResult.testCount > 0 ? Math.min(100, ((cpuMetric.winsVsReference / caseResult.testCount) * 100)).toFixed(0) + '%' : '0%');
    
    // Status and colors
    const hasFailures = result.failures.length > 0;
    
    // Calculate percentage difference vs reference
    const refCpuAvg = caseResult.reference.avgCpuTime;
    let cpuVsRef = 'ref';
    let cpuVsRefColor = '#00aaff';
    if (!isReference && refCpuAvg > 0) {
      // Don't show percentage for failed implementations
      if (hasFailures && result.successCount === 0) {
        cpuVsRef = 'N/A';
        cpuVsRefColor = '#666666'; // Gray for N/A
      } else {
        const percentDiff = ((result.avgCpuTime - refCpuAvg) / refCpuAvg) * 100;
        if (percentDiff < 0) {
          cpuVsRef = `${percentDiff.toFixed(1)}%`;
          cpuVsRefColor = '#00ff00'; // Green for better (faster)
        } else if (percentDiff > 0) {
          cpuVsRef = `+${percentDiff.toFixed(1)}%`;
          cpuVsRefColor = percentDiff < 50 ? '#ffff00' : '#ff6600'; // Yellow/orange for slower
        } else {
          cpuVsRef = '0.0%';
          cpuVsRefColor = '#88ff88'; // Light green for equal
        }
      }
    }
    const status = hasFailures ? `❌ (${result.failures.length})` : '✅';
    const nameColor = isReference ? '#00aaff' : (hasFailures ? '#ff6666' : '#ffffff');
    
    // Calculate separate color ranges for different CPU metrics
    let avgCpuColor = '#888888';
    let p50CpuColor = '#888888';
    let p95CpuColor = '#888888';
    
    if (performanceRanges?.cpu && !hasFailures) {
      // For Average CPU column - use avgCpuTime values for comparison
      const allAvgCpus = [caseResult.reference, ...Array.from(caseResult.implementations.values())]
        .filter(r => r.failures.length === 0)
        .map(r => r.avgCpuTime)
        .filter(v => isFinite(v) && v > 0);
      
      if (allAvgCpus.length > 1) {
        const minAvg = Math.min(...allAvgCpus);
        const maxAvg = Math.max(...allAvgCpus);
        avgCpuColor = this.getPerformanceColor(result.avgCpuTime, minAvg, maxAvg, 'minimize');
      }
      
      // For P50 CPU column - use P50 values
      p50CpuColor = this.getPerformanceColor(cpuStats.p50, performanceRanges.cpu.min, performanceRanges.cpu.max, 'minimize');
      
      // For P95 CPU column - use P95 values for comparison
      const allP95Cpus = [caseResult.reference, ...Array.from(caseResult.implementations.values())]
        .filter(r => r.failures.length === 0)
        .map(r => calculateStatistics(r.cpuTimes).p95)
        .filter(v => isFinite(v) && v > 0);
      
      if (allP95Cpus.length > 1) {
        const minP95 = Math.min(...allP95Cpus);
        const maxP95 = Math.max(...allP95Cpus);
        p95CpuColor = this.getPerformanceColor(cpuStats.p95, minP95, maxP95, 'minimize');
      }
    } else if (hasFailures) {
      avgCpuColor = p50CpuColor = p95CpuColor = '#ff6666';
    }
    
    // Color StdDev based on CPU performance ranges (lower stddev is better)
    let stdDevColor = '#888888'; // Default fallback
    if (performanceRanges?.cpu && !hasFailures) {
      const allStdDevs = [caseResult.reference, ...Array.from(caseResult.implementations.values())]
        .filter(r => r.failures.length === 0)
        .map(r => calculateStatistics(r.cpuTimes).stdDev)
        .filter(v => isFinite(v) && v > 0);
      
      if (allStdDevs.length > 1) {
        const minStdDev = Math.min(...allStdDevs);
        const maxStdDev = Math.max(...allStdDevs);
        stdDevColor = this.getPerformanceColor(cpuStats.stdDev, minStdDev, maxStdDev, 'minimize');
      }
    } else if (hasFailures) {
      stdDevColor = '#ff6666';
    }
    
    let row = `<tr>`;
    row += `<td style="color: ${nameColor};">${result.implementationName}</td>`;
    
    // Combined CPU column with individual coloring
    row += `<td><span style="color: ${avgCpuColor};">${result.avgCpuTime.toFixed(3)}</span>/<span style="color: ${p50CpuColor};">${cpuStats.p50.toFixed(3)}</span>/<span style="color: ${p95CpuColor};">${cpuStats.p95.toFixed(3)}</span>/<span style="color: ${stdDevColor};">${cpuStats.stdDev.toFixed(3)}</span></td>`;
    
    // CPU vs Reference column
    row += `<td><span style="color: ${cpuVsRefColor};">${cpuVsRef}</span></td>`;
    
    // Combined CPU Win/Beat column
    row += `<td><span style="color: #ffff00;">${cpuWinRate}%</span>(<span style="color: #888;">${cpuBeatRefRate}</span>)</td>`;
    
    // Custom metrics
    for (const metric of this.config.metrics) {
      const metricResult = result.metrics.get(metric.name)!;
      const metricStats = calculateStatistics(metricResult.values);
      const winRate = caseResult.testCount > 0 ? Math.min(100, ((metricResult.winsVsOthers / caseResult.testCount) * 100)).toFixed(0) : '0';
      const beatRate = isReference ? 
        (caseResult.testCount > 0 ? Math.min(100, ((metricResult.timesUnbeaten / caseResult.testCount) * 100)).toFixed(0) + '%' : '0%') :
        (caseResult.testCount > 0 ? Math.min(100, ((metricResult.winsVsReference / caseResult.testCount) * 100)).toFixed(0) + '%' : '0%');
      
      // Add indicator if this implementation has failures (using reference values in aggregates)
      const metricDisplay = hasFailures ? `${metricStats.p50.toFixed(3)}*` : metricStats.p50.toFixed(3);
      
      // Smart metric performance coloring for all implementations
      let metricColor = '#888888'; // Default fallback
      if (hasFailures) {
        metricColor = '#ffaa00'; // Orange for failures
      } else if (performanceRanges?.metrics.has(metric.name)) {
        const range = performanceRanges.metrics.get(metric.name)!;
        metricColor = this.getPerformanceColor(metricStats.p50, range.min, range.max, range.mode);
      }
      
      // Combined metric column with value and win/beat rates
      row += `<td><span style="color: ${metricColor};">${metricDisplay}</span> <span style="color: #ffff00;">${winRate}%</span>(<span style="color: #888;">${beatRate}</span>)</td>`;
    }
    
    const statusColor = hasFailures ? '#ff0000' : '#00ff00';
    row += `<td style="color: ${statusColor};">${status}</td>`;
    row += `</tr>`;
    
    return row;
  }
}

/**
 * Convenience function to create a new benchmark
 */
export function benchmark<TResult, TInput = any>(name: string): Benchmark<TResult, TInput> {
  return new Benchmark<TResult, TInput>(name);
}

// Global benchmark management
const activeBenchmarks = new Map<string, Benchmark<any, any>>();
const completedBenchmarks = new Set<string>();

/**
 * Register a benchmark to be run as part of the global benchmark suite
 */
export function registerBenchmark<TResult, TInput>(name: string, setupFn: (b: Benchmark<TResult, TInput>) => void): void {
  const bench = benchmark<TResult, TInput>(name);
  setupFn(bench);
  activeBenchmarks.set(name, bench);
  completedBenchmarks.delete(name); // Reset completion status
  resultsDisplayed = false; // Reset display flag when new benchmarks are registered
}

/**
 * Run all registered benchmarks (tick-based execution)
 * Returns true when all benchmarks are complete
 */
export function runAllBenchmarks(): boolean {
  let allComplete = true;
  
  // First pass: Run all benchmarks to completion without displaying results
  for (const [name, bench] of activeBenchmarks) {
    if (!completedBenchmarks.has(name)) {
      if (bench.run()) {
        completedBenchmarks.add(name);
      } else {
        allComplete = false;
      }
    }
  }
  
  // Second pass: Display all results together when everything is complete
  if (allComplete) {
    displayAllBenchmarkResults();
  }
  
  return allComplete;
}

// Track if results have been displayed to avoid overloading client
let resultsDisplayed = false;

/**
 * Display results for all completed benchmarks in a single log message
 */
function displayAllBenchmarkResults(): void {
  if (activeBenchmarks.size === 0 || resultsDisplayed) return;
  
  let html = `<style>
    table { border-collapse: collapse; font-family: monospace; font-size: 12px; margin-bottom: 20px; width: 100%; }
    table th, table td { border: 1px solid #666; }
    table td { padding: 5px 5px; text-align: right;}
    table th { padding: 5px 5px; background-color: #333; color: #fff; font-weight: bold; text-align: center;}
    td:first-child { text-align: left; }
    .benchmark-title { color: #00ff00; font-weight: bold; font-size: 16px; margin-bottom: 15px; }
    .case-title { color: #ffff00; font-weight: bold; font-size: 13px; margin: 15px 0 5px 0; }
    .case-info { color: #888; font-size: 11px; margin-bottom: 10px; }
    .failures { color: #ff6666; font-weight: bold; margin-top: 15px; }
    .failure-item { color: #ffaa00; margin-left: 10px; }
    .failure-detail { color: #ff6666; margin-left: 20px; font-size: 11px; }
    .benchmark-separator { color: #666; font-size: 14px; margin: 30px 0 20px 0; border-top: 2px solid #444; padding-top: 15px; }
  </style>`;
  html = html.split('\n').join('');
  html += `<div class="benchmark-title">📊 All Benchmark Results</div>`;
  
  let isFirstBenchmark = true;
  for (const [name, bench] of activeBenchmarks) {
    if (completedBenchmarks.has(name)) {
      if (!isFirstBenchmark) {
        html += `<div class="benchmark-separator">═══════════════════════════════════════════════════════════</div>`;
      }
      isFirstBenchmark = false;
      
      html += generateBenchmarkHTML(name, bench);
    }
  }
  
  console.log(html);
  resultsDisplayed = true; // Mark results as displayed to prevent re-output
}

/**
 * Generate HTML for a single benchmark's results
 */
function generateBenchmarkHTML(benchmarkName: string, bench: Benchmark<any, any>): string {
  const results = bench.getResults();
  let html = `<div class="benchmark-title">${benchmarkName}</div>`;
  
  for (const caseResult of results) {
    // Get the runner to access config
    const runner = (bench as any).runner;
    if (!runner) continue;
    
    const config = runner.config;
    const repeatsText = config.repeats > 1 ? `, ${config.repeats} repeats` : '';
    const warmupText = config.warmupRounds > 0 ? `, ${config.warmupRounds} warmup` : '';
    
    html += `<div class="case-title">📋 Case: ${caseResult.caseName}</div>`;
    html += `<div class="case-info">(${caseResult.testCount} tests, ${config.iterations} iterations${repeatsText}${warmupText})</div>`;
    
    // Start table
    html += '<table>';
    
    // Build header
    html += '<tr><th>Implementation</th><th>CPU Avg/P50/P95/StdDev</th><th>CPU vs Ref</th><th>CPU Win%(Beat%)</th>';
    for (const metric of config.metrics) {
      html += `<th>${metric.name} Win%(Beat%)</th>`;
    }
    html += '<th>Status</th></tr>';
    
    // Sort results by CPU win percentage (descending), then by average CPU time
    const allResults = [caseResult.reference, ...Array.from(caseResult.implementations.values())];
    allResults.sort((a, b) => {
      if (a.failures.length !== b.failures.length) {
        return a.failures.length - b.failures.length;
      }
      
      // Calculate CPU win rates for sorting
      const aSuccessRate = a.totalAttempts > 0 ? (a.successCount / a.totalAttempts) : 0;
      const bSuccessRate = b.totalAttempts > 0 ? (b.successCount / b.totalAttempts) : 0;
      
      const aCpuMetric = a.metrics.get('CPU Time')!;
      const bCpuMetric = b.metrics.get('CPU Time')!;
      
      const aCpuWinRate = aSuccessRate > 0 ? (aCpuMetric.winsVsOthers / a.successCount) : 0;
      const bCpuWinRate = bSuccessRate > 0 ? (bCpuMetric.winsVsOthers / b.successCount) : 0;
      
      // Sort by CPU win rate (descending - higher win rate first)
      if (bCpuWinRate !== aCpuWinRate) {
        return bCpuWinRate - aCpuWinRate;
      }
      
      // If win rates are equal, sort by average CPU time (ascending - faster first)
      return a.avgCpuTime - b.avgCpuTime;
    });
    
    // Calculate performance ranges for smart coloring
    const performanceRanges = runner.calculatePerformanceRanges(allResults, caseResult);
    
    // Add data rows
    for (const result of allResults) {
      html += runner.buildTableRow(result, caseResult, performanceRanges);
    }
    
    html += '</table>';
    
    // Show failures if any
    const failedResults = allResults.filter(r => r.failures.length > 0);
    if (failedResults.length > 0) {
      html += '<div class="failures">⚠️  Failures:</div>';
      for (const result of failedResults) {
        html += `<div class="failure-item">${result.implementationName}:</div>`;
        result.failures.slice(0, 3).forEach(failure => {
          html += `<div class="failure-detail">${failure}</div>`;
        });
        if (result.failures.length > 3) {
          html += `<div class="failure-detail">... and ${result.failures.length - 3} more</div>`;
        }
      }
    }
  }
  
  return html;
}

/**
 * Clear all benchmark results and reset state
 */
export function clearBenchmarks(): void {
  activeBenchmarks.clear();
  completedBenchmarks.clear();
  resultsDisplayed = false; // Reset display flag
}
