# Ticket 61 — Inline run-comparison view

**Phase:** 7
**Depends on:** [60](./60-mlflow-query.md), [20](./20-app-shell.md)
**Blocks:** [62](./62-decision-diff.md)

## Goal

A view that lists recent runs, lets the human pick 2-N to compare, and renders side-by-side: name + outcome, params, metrics. Decision-diff rendering arrives in Ticket 62; this ticket lays the surface.

> Spec §5.3 — *"Run comparison (the keep/kill decision surface). Built on MLflow as engine, not surface — render an inline comparison view inside the app."*

## Files

- Create: `src/lib/runs-store.svelte.ts`.
- Create: `src/lib/components/RunsList.svelte`.
- Create: `src/lib/components/CompareView.svelte`.
- Modify: `src/lib/components/AppShell.svelte` — add a tab in the Results pane (DataFrame / Plot / Runs).

## Steps

- [ ] **1. Store.**

  ```ts
  // src/lib/runs-store.svelte.ts
  import { invoke } from '@tauri-apps/api/core';

  export type Run = {
    readonly run_id: string;
    readonly name: string;
    readonly status: string;
    readonly outcome: 'keep' | 'kill' | 'iterate' | null;
    readonly metrics: Readonly<Record<string, number>>;
    readonly params: Readonly<Record<string, string>>;
    readonly decisions: Readonly<Record<string, string>>;
    readonly look_here: readonly string[];
  };

  export function createRunsStore(): {
    readonly runs: readonly Run[];
    refresh(): Promise<void>;
    selected: Set<string>;
  } {
    let runs = $state<Run[]>([]);
    const selected = $state(new Set<string>());

    async function refresh(): Promise<void> {
      const result = await invoke<readonly Run[]>('list_runs', { limit: 200 });
      runs = [...result];
    }

    return { get runs() { return runs; }, refresh, selected };
  }
  ```

- [ ] **2. RunsList.** A scrollable list with a checkbox per row, name, status, outcome badge.

- [ ] **3. CompareView.** Given the selected runs, render a table with one column per run and rows:

  - Name, status, outcome (top section)
  - **Params** (middle section, alphabetical)
  - **Metrics** (numeric, dense; format with 4 sig figs)
  - **Decisions** (decisions render is a placeholder in this ticket — Ticket 62 adds the diff)

  Highlight cells that differ across columns (light yellow background).

- [ ] **4. Tab in Results pane.** Three tabs: `DataFrame`, `Plot`, `Runs`. Persist active tab in a session-only store.

- [ ] **5. Smoke test.**

  - Approve a checkpoint → start a run → keep it.
  - Approve another with a different metric → keep.
  - Open Runs tab → tick both → compare view shows them.

- [ ] **6. Lint + commit.**

  ```sh
  git commit -m "feat(runs): inline run-comparison view (params + metrics)"
  ```

## Acceptance

- Selecting two runs renders both columns within 100ms.
- Outcome badge colours: green = keep, red = kill, amber = iterate.
- No `any`.

## Out of scope

- Decision-as-diff rendering — Ticket 62.
- Run search / filter — fast-follow.
- Charting metrics across runs — fast-follow.
