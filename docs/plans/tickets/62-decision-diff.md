# Ticket 62 — Decision diff: declared decisions as first-class diff rows

**Phase:** 7
**Depends on:** [61](./61-comparison-view.md)
**Blocks:** none in MVP

## Goal

Show the seven slot answers (per-run) as a top section in the compare view, *visually elevated above params and metrics*, with diffs highlighted. This is the differentiator from a vanilla MLflow UI: the declared frame decisions are the lens through which everything else is read.

> Spec §5.3 — *"Shows params + metrics **and** declared premise decisions as first-class diffs ('A: temporal split / B: random split')."*

## Files

- Modify: `src/lib/components/CompareView.svelte` — add `<DecisionDiff />` block at the top.
- Create: `src/lib/components/DecisionDiff.svelte`.
- Create: `src/lib/heuristics.ts` — the cheap deterministic flags from spec §10.

## Steps

- [ ] **1. Decision diff component.**

  - Group rows by slot name in spec §4's order (`validation_strategy` first — it's the headline leakage call).
  - Per row, render one cell per selected run with the slot's answer.
  - Cells that disagree get a thick left border in a contrasting colour.
  - Above the table, render any `look_here` items aggregated across selected runs as red callouts.

- [ ] **2. Cheap deterministic auto-flags.**

  ```ts
  // src/lib/heuristics.ts
  import type { Run } from './runs-store.svelte';

  export type Flag = { readonly run: string; readonly slot: string; readonly note: string };

  export function flagSuspiciousRuns(runs: readonly Run[]): readonly Flag[] {
    const flags: Flag[] = [];
    for (const run of runs) {
      // Spec §10 — "AUC above threshold → check leakage".
      const auc = run.metrics['auc'] ?? run.metrics['roc_auc'];
      if (typeof auc === 'number' && auc > 0.99) {
        flags.push({ run: run.run_id, slot: 'auc>0.99', note: 'Suspiciously high AUC — check for leakage.' });
      }
      // Spec §10 — "large train/test metric gap → check overfit".
      const trainAcc = run.metrics['train_accuracy'];
      const testAcc = run.metrics['accuracy'] ?? run.metrics['test_accuracy'];
      if (typeof trainAcc === 'number' && typeof testAcc === 'number' && trainAcc - testAcc > 0.1) {
        flags.push({ run: run.run_id, slot: 'train-vs-test', note: `Train/test gap ${(trainAcc - testAcc).toFixed(2)} — check overfit.` });
      }
    }
    return flags;
  }
  ```

  Render the flags in the compare-view header as red banners. Per spec §10 *"Never let Claude be the only thing checking Claude"* — these flags are computed in the harness, independent of Claude's commentary.

- [ ] **3. Smoke test.**

  - Take two runs with the same proposal but different `validation_strategy` answers (temporal vs random).
  - Confirm the slot row appears at the top, with the differing answers highlighted.
  - Force a metric `auc=0.999` and confirm the leakage banner fires.

- [ ] **4. Lint + commit.**

  ```sh
  git commit -m "feat(runs): decision diff + cheap deterministic auto-flags"
  ```

## Acceptance

- Decision rows render above metric rows.
- At least the two heuristics above fire on contrived data.
- No `any`. No floating promises.

## Out of scope

- Configurable flag thresholds in UI — fast-follow.
- Per-feature drift charts — out of MVP.
- ML-based anomaly detection on the runs themselves — out of MVP.
