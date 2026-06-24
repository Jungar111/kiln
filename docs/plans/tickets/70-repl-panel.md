# Ticket 70 — REPL panel for manual inspection at checkpoints

**Phase:** 8 (REPL + drift)
**Depends on:** [05](./05-experiment-vs-inspection.md), [32](./32-premise-gate-ui.md)
**Blocks:** [71](./71-experiment-vs-inspection-enforcement.md)

## Goal

A small REPL surface that appears next to the premise gate and the results gate. Every line the human types is sent to `execute(code, ephemeral: true)`. The output is shown inline; nothing is logged. This is *"the instrument of the review, the antidote to rubber-stamping"* (spec §5 secondary surfaces).

## Why it lives next to the gates, not as a top-level pane

Per spec §5: *"REPL access for manual inspection. Not the main flow. Lives in the idle-kernel moment at a checkpoint."* If the REPL were a peer of the chat surface, it would invite the human to drop into implementation — exactly the failure mode the product is designed to prevent.

## Files

- Create: `src/lib/components/InspectionRepl.svelte`.
- Modify: `src/lib/components/PremiseGate.svelte` — embed the REPL collapsed under "inspect kernel state".
- Modify: `src/lib/components/ResultsGate.svelte` — same.

## Steps

- [ ] **1. Component.**

  ```svelte
  <script lang="ts">
    import { invoke } from '@tauri-apps/api/core';

    type ExecuteResponse = {
      readonly status: 'ok' | 'error';
      readonly stdout: string;
      readonly value: string | null;
      readonly traceback: string | null;
      readonly ephemeral: boolean;
    };

    type Entry = { readonly code: string; readonly response: ExecuteResponse };
    let history = $state<Entry[]>([]);
    let draft = $state('');

    async function submit(event: SubmitEvent): Promise<void> {
      event.preventDefault();
      const code = draft;
      draft = '';
      const response = await invoke<ExecuteResponse>('execute', { code, ephemeral: true });
      history = [...history, { code, response }];
    }
  </script>

  <details class="inspect">
    <summary>inspect kernel state</summary>
    <ol>
      {#each history as entry}
        <li>
          <pre class="in">› {entry.code}</pre>
          {#if entry.response.stdout}<pre>{entry.response.stdout}</pre>{/if}
          {#if entry.response.value !== null}<pre class="value">{entry.response.value}</pre>{/if}
          {#if entry.response.traceback}<pre class="error">{entry.response.traceback}</pre>{/if}
        </li>
      {/each}
    </ol>
    <form onsubmit={submit}>
      <input bind:value={draft} placeholder="ephemeral cell" />
      <button type="submit">run</button>
    </form>
    <p class="hint">Calls here run with <code>ephemeral=true</code> — they will not appear in the logged run.</p>
  </details>
  ```

- [ ] **2. Embed in both gates.** Both `PremiseGate.svelte` and `ResultsGate.svelte` include `<InspectionRepl />`.

- [ ] **3. Smoke test.** Approve a fake checkpoint, type `df.shape`, see the value. Decline / kill / restart — confirm the REPL session is reset (state lives on the gate, not globally).

- [ ] **4. Lint + commit.**

  ```sh
  git commit -m "feat(repl): inspection REPL embedded in premise/results gates"
  ```

## Acceptance

- Every command goes through `execute` with `ephemeral=true` (verify via the sidecar log).
- The REPL is not addressable outside the gates — there is no global `<InspectionRepl>` mount.
- No `any`.

## Out of scope

- Multiline editing — fast-follow.
- Tab-completion — out of MVP.
- History persistence — out of MVP.
