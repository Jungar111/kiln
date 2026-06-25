<script lang="ts">
  import { createRunsStore } from '$lib/runs-store.svelte';
  import RunsList from './RunsList.svelte';
  import CompareView from './CompareView.svelte';

  const store = createRunsStore();
  let error = $state<string | null>(null);

  const selectedRuns = $derived(store.runs.filter((run) => store.selected.has(run.run_id)));

  async function load(): Promise<void> {
    try {
      await store.refresh();
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  // Load once when the pane mounts; the sidecar may not be ready yet, hence the
  // error capture above and the manual refresh button.
  void load();
</script>

<div class="results">
  <header>
    <h2>Runs</h2>
    <button
      type="button"
      onclick={() => {
        void load();
      }}>refresh</button
    >
  </header>
  {#if error}<p class="error">{error}</p>{/if}
  <RunsList runs={store.runs} selected={store.selected} />
  {#if selectedRuns.length > 0}
    <CompareView runs={selectedRuns} />
  {/if}
</div>

<style>
  .results {
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100%;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
  }
  button {
    background: #2a2a2a;
    color: #e6e6e6;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 4px 12px;
    cursor: pointer;
  }
  .error {
    color: #e0866f;
    font-size: 12px;
    margin: 0;
  }
</style>
