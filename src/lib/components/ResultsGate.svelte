<script lang="ts">
  import type { Verdict } from '$lib/checkpoint-types';
  import InspectionRepl from './InspectionRepl.svelte';

  let { runId, onpick }: { runId: string; onpick: (verdict: Verdict) => void } = $props();
</script>

<div class="results-gate" role="group" aria-label="Results gate">
  <span class="label">Run <code>{runId.slice(0, 8)}</code> — keep, kill, or iterate?</span>
  <!-- Same inspection surface as the premise gate: poke the kernel before the
       keep/kill/iterate call. Ephemeral — nothing lands in the run record. -->
  <InspectionRepl />
  <div class="actions">
    <button
      type="button"
      class="keep"
      onclick={() => {
        onpick('keep');
      }}>keep</button
    >
    <button
      type="button"
      class="kill"
      onclick={() => {
        onpick('kill');
      }}>kill</button
    >
    <button
      type="button"
      class="iterate"
      onclick={() => {
        onpick('iterate');
      }}>iterate</button
    >
  </div>
</div>

<style>
  .results-gate {
    position: fixed;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    width: min(560px, 92vw);
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: #161616;
    border: 1px solid #333;
    border-radius: 10px;
    padding: 12px 16px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.4);
    z-index: 9;
    color: #e6e6e6;
  }
  code {
    color: #9ad;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  button {
    border: none;
    border-radius: 6px;
    padding: 6px 14px;
    cursor: pointer;
    font: inherit;
  }
  .keep {
    background: #2e6f4e;
    color: #eafff2;
  }
  .kill {
    background: #6f2e2e;
    color: #ffeaea;
  }
  .iterate {
    background: #5a4a2e;
    color: #fff3e0;
  }
</style>
