<script lang="ts">
  import type { SvelteSet } from 'svelte/reactivity';
  import type { Run } from '$lib/runs-store.svelte';

  let { runs, selected }: { runs: readonly Run[]; selected: SvelteSet<string> } = $props();

  function toggle(runId: string): void {
    if (selected.has(runId)) selected.delete(runId);
    else selected.add(runId);
  }
</script>

<ul class="runs">
  {#each runs as run (run.run_id)}
    <li>
      <label>
        <input
          type="checkbox"
          checked={selected.has(run.run_id)}
          onchange={() => {
            toggle(run.run_id);
          }}
        />
        <span class="name">{run.name || run.run_id.slice(0, 8)}</span>
        <span class="status">{run.status}</span>
        {#if run.outcome}<span class="outcome o-{run.outcome}">{run.outcome}</span>{/if}
      </label>
    </li>
  {:else}
    <li class="empty">No runs yet — approve a checkpoint to start one.</li>
  {/each}
</ul>

<style>
  .runs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  label {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-radius: 4px;
    cursor: pointer;
  }
  label:hover {
    background: #242424;
  }
  .name {
    flex: 1;
  }
  .status {
    font-size: 11px;
    color: #888;
  }
  .outcome {
    font-size: 10px;
    text-transform: uppercase;
    padding: 1px 6px;
    border-radius: 999px;
  }
  .o-keep {
    background: #1f5a3d;
    color: #d9ffe9;
  }
  .o-kill {
    background: #5a1f1f;
    color: #ffd9d9;
  }
  .o-iterate {
    background: #5a4a1f;
    color: #fff0cf;
  }
  .empty {
    color: #777;
    padding: 6px;
  }
</style>
