import type { Run } from './runs-store.svelte';

export type Flag = { readonly run: string; readonly slot: string; readonly note: string };

/**
 * Cheap deterministic flags computed in the harness, independent of Claude's
 * commentary (spec §10 — "never let Claude be the only thing checking Claude").
 */
export function flagSuspiciousRuns(runs: readonly Run[]): readonly Flag[] {
  const flags: Flag[] = [];
  for (const run of runs) {
    const auc = run.metrics.auc ?? run.metrics.roc_auc;
    if (typeof auc === 'number' && auc > 0.99) {
      flags.push({
        run: run.run_id,
        slot: 'auc>0.99',
        note: 'Suspiciously high AUC — check for leakage.',
      });
    }
    const trainAcc = run.metrics.train_accuracy;
    const testAcc = run.metrics.accuracy ?? run.metrics.test_accuracy;
    if (typeof trainAcc === 'number' && typeof testAcc === 'number' && trainAcc - testAcc > 0.1) {
      flags.push({
        run: run.run_id,
        slot: 'train-vs-test',
        note: `Train/test gap ${(trainAcc - testAcc).toFixed(2)} — check overfit.`,
      });
    }
  }
  return flags;
}
