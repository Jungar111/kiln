# Ticket 32 — Premise gate modal (layered disclosure)

**Phase:** 4
**Depends on:** [31](./31-slot-validation.md)
**Blocks:** [33](./33-mlflow-tag-write.md)

## Goal

When the `sidecar:checkpoint` event arrives, render a modal that shows the proposal using the layered-disclosure pattern from spec §4:

1. **Critical** slots foregrounded.
2. **Notable / fyi** slots collapsed.
3. Each slot has the `look_here` highlight when Claude flagged it.
4. The model's code is **collapsed** under "show experiment code" (linked from Ticket 12's `+page.svelte` content for now; Ticket 40 swaps in a real CodeView).

Two buttons: **Approve** (fires `approve_checkpoint` Tauri command — wired by Ticket 33) and **Decline** (returns a `tool_result` to Claude saying the human declined and why).

## Files

- Create: `src/lib/components/PremiseGate.svelte`.
- Create: `src/lib/components/SlotRow.svelte`.
- Create: `src/lib/checkpoint-store.svelte.ts`.
- Modify: `src/lib/components/AppShell.svelte` — listen for `sidecar:checkpoint`, render the modal on top of the panes.

## Steps

- [ ] **1. Type the payload.**

  ```ts
  // src/lib/checkpoint-types.ts
  export type Severity = 'critical' | 'notable' | 'fyi';
  export type Slot = { readonly in_scope: boolean; readonly severity: Severity; readonly answer: string };
  export type ProposeExperiment = {
    readonly title: string;
    readonly premise: string;
    readonly validation_strategy: Slot;
    readonly target_definition: Slot;
    readonly feature_provenance: Slot;
    readonly preprocessing_fit_scope: Slot;
    readonly data_scope_and_exclusions: Slot;
    readonly missing_data_handling: Slot;
    readonly metric_choice: Slot;
    readonly look_here: readonly string[];
  };
  ```

- [ ] **2. Checkpoint store.**

  ```ts
  // src/lib/checkpoint-store.svelte.ts
  import { listen } from '@tauri-apps/api/event';
  import type { ProposeExperiment } from './checkpoint-types';

  export function createCheckpointStore(): {
    readonly pending: ProposeExperiment | null;
    clear(): void;
  } {
    let pending: ProposeExperiment | null = $state(null);
    void listen<ProposeExperiment>('sidecar:checkpoint', (event) => {
      pending = event.payload;
    });
    return {
      get pending() { return pending; },
      clear() { pending = null; },
    };
  }
  ```

- [ ] **3. SlotRow component.** Renders the slot's name, severity badge, and answer. If `severity === 'critical'`, expanded by default. If `severity === 'fyi'`, collapsed.

- [ ] **4. PremiseGate modal.** Renders the title, premise paragraph, the seven `SlotRow`s grouped by severity, and the `look_here` callouts at the top in a red-bordered box. Approve/Decline buttons.

- [ ] **5. Wire in `AppShell.svelte`.**

  ```svelte
  <script lang="ts">
    import { createCheckpointStore } from '$lib/checkpoint-store.svelte';
    import PremiseGate from './PremiseGate.svelte';
    const ckpt = createCheckpointStore();
  </script>
  ...
  {#if ckpt.pending}
    <PremiseGate proposal={ckpt.pending} onclose={() => ckpt.clear()} />
  {/if}
  ```

- [ ] **6. Smoke test.** Use a temporary "fire fake checkpoint" button in the Chat pane (deleted before the commit) to verify the modal renders correctly.

- [ ] **7. Lint + commit.**

  ```sh
  git commit -m "feat(checkpoint): premise-gate modal with layered disclosure"
  ```

## Acceptance

- Critical slots expanded by default; FYI collapsed; Notable expanded by default but visually quieter.
- `look_here` items appear at the top in a way that grabs attention (red border, bold copy).
- No `any`. No floating promises (listens are `void`ed).

## Out of scope

- The actual "show experiment code" view — fast-follow. Render a placeholder `<button>show code</button>` that does nothing yet.
- Editing slots inline (the human cannot change them, only approve/decline).
- Accessibility audit — fast-follow.
