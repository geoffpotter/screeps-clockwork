import { initialize } from '../src/index';

import { runTestScenarios } from './basicBot';
import { run } from './tests';
import { runBenchmarks } from './tests/bench/all_benchmarks';
import { visualize } from './visualizations';

export const loop = () => {
  if (Game.cpu.bucket > 500) {
    runTestScenarios();
  }
  initialize(true);
  run();
  visualize();
  runBenchmarks();
};
