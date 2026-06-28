<script lang="ts">
  import type { Verdict } from '$lib/checkpoint-types';
  import InspectionRepl from './InspectionRepl.svelte';

  let { runId, onpick }: { runId: string; onpick: (verdict: Verdict) => void } = $props();
</script>

<div class="gate" role="group" aria-label="Results gate">
  <!-- Same inspection surface as the premise gate: poke the kernel before the
       keep/kill/iterate call. Ephemeral — nothing lands in the run record. -->
  <InspectionRepl />
  <div class="bar">
    <div class="note">
      Run <code>{runId.slice(0, 8)}</code> — your call binds the
      <span class="tag">kept</span> tag.
    </div>
    <div class="actions">
      <button
        type="button"
        class="keep"
        onclick={() => {
          onpick('keep');
        }}>Keep</button
      >
      <button
        type="button"
        class="kill"
        onclick={() => {
          onpick('kill');
        }}>Kill</button
      >
      <button
        type="button"
        class="iterate"
        onclick={() => {
          onpick('iterate');
        }}>Iterate</button
      >
    </div>
  </div>
</div>

<style>
  .gate {
    flex-shrink: 0;
    border-top: 1px solid var(--bd);
    background: var(--bg-panel);
    display: flex;
    flex-direction: column;
  }
  .gate :global(.inspect) {
    margin: 10px 22px 0;
  }
  .bar {
    padding: 12px 22px;
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .note {
    font-size: 11.5px;
    color: var(--tx-dim);
  }
  code {
    font-family: var(--font-mono);
    color: var(--tx-2);
  }
  .tag {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--good);
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: 9px;
  }
  button {
    border: none;
    border-radius: 6px;
    padding: 8px 16px;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
  }
  .keep {
    background: var(--good);
    color: #0f1a10;
  }
  .kill {
    background: var(--bad);
    color: #1a0f0c;
  }
  .iterate {
    background: transparent;
    color: var(--tx-2);
    border: 1px solid #3a3630;
    font-weight: 400;
  }
  button:hover {
    filter: brightness(1.07);
  }
</style>
