<script lang="ts">
  import { createRunsStore } from '$lib/runs-store.svelte';
  import { dfStore } from '$lib/df-store.svelte';
  import { plotStore } from '$lib/plot-store.svelte';
  import RunsList from './RunsList.svelte';
  import CompareView from './CompareView.svelte';
  import DataFrameView from './DataFrameView.svelte';
  import PlotPanel from './PlotPanel.svelte';

  type Tab = 'dataframe' | 'plot' | 'runs';
  let tab = $state<Tab>('runs');

  const store = createRunsStore();
  let error = $state<string | null>(null);
  const selectedRuns = $derived(store.runs.filter((run) => store.selected.has(run.run_id)));

  async function loadRuns(): Promise<void> {
    try {
      await store.refresh();
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  void loadRuns();
</script>

<div class="results">
  <nav class="tabs">
    <button class:active={tab === 'dataframe'} type="button" onclick={() => (tab = 'dataframe')}
      >DataFrame</button
    >
    <button class:active={tab === 'plot'} type="button" onclick={() => (tab = 'plot')}>Plot</button>
    <button class:active={tab === 'runs'} type="button" onclick={() => (tab = 'runs')}>Runs</button>
  </nav>

  {#if tab === 'dataframe'}
    {#if dfStore.current}
      <DataFrameView handle={dfStore.current.handle} rows={dfStore.current.rows} />
    {:else}
      <p class="empty">Run a cell that returns a DataFrame to explore it here.</p>
    {/if}
  {:else if tab === 'plot'}
    {#if plotStore.displays.length > 0}
      <PlotPanel displays={plotStore.displays} />
    {:else}
      <p class="empty">Run a cell that draws a plot to see it here.</p>
    {/if}
  {:else}
    <div class="runs-tab">
      <header>
        <h2>Runs</h2>
        <button
          type="button"
          onclick={() => {
            void loadRuns();
          }}>refresh</button
        >
      </header>
      {#if error}<p class="empty">{error}</p>{/if}
      <RunsList runs={store.runs} selected={store.selected} />
      {#if selectedRuns.length > 0}
        <CompareView runs={selectedRuns} />
      {/if}
    </div>
  {/if}
</div>

<style>
  .results {
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100%;
    min-height: 0;
  }
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid #2a2a2a;
  }
  .tabs button {
    background: transparent;
    color: #999;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 4px 10px;
    cursor: pointer;
    font: inherit;
  }
  .tabs button.active {
    color: #e6e6e6;
    border-bottom-color: #9ad;
  }
  .runs-tab {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 16px;
  }
  header button {
    background: #2a2a2a;
    color: #e6e6e6;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 4px 12px;
    cursor: pointer;
  }
  .empty {
    color: #777;
    font-size: 13px;
  }
</style>
