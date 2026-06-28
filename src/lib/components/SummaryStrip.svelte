<script lang="ts">
  import { fetchSummary, type SummaryRow } from '$lib/arrow-client';
  import { toMessage } from '$lib/errors';

  let { handle }: { handle: string } = $props();

  let rows = $state<SummaryRow[]>([]);
  let error = $state<string | null>(null);

  function fmt(value: number | null): string {
    if (value === null) return '—';
    return Number.isInteger(value) ? String(value) : value.toPrecision(4);
  }

  async function load(h: string): Promise<void> {
    try {
      rows = await fetchSummary(h);
      error = null;
    } catch (err) {
      error = toMessage(err);
    }
  }

  $effect(() => {
    void load(handle);
  });
</script>

{#if error}
  <p class="err">{error}</p>
{:else}
  <div class="strip">
    {#each rows as row (row.column)}
      <div class="col">
        <div class="name">{row.column}</div>
        <div class="dtype">{row.dtype}</div>
        <div class="stat">nulls {row.nulls}</div>
        {#if row.min !== null}
          <div class="stat">min {fmt(row.min)}</div>
          <div class="stat">max {fmt(row.max)}</div>
          <div class="stat">mean {fmt(row.mean)}</div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .strip {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    padding: 10px 16px;
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .col {
    min-width: 120px;
    border: 1px solid var(--bd);
    border-radius: 6px;
    padding: 6px 8px;
    background: var(--bg-panel);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--tx-mut);
  }
  .name {
    color: var(--tx-bright);
    font-weight: 500;
    white-space: nowrap;
    font-family: var(--font-sans);
    font-size: 11px;
  }
  .dtype {
    color: var(--ember-soft);
    margin-bottom: 4px;
  }
  .stat {
    color: var(--tx-dim);
  }
  .err {
    color: var(--bad);
    font-size: 12px;
    margin: 8px 16px;
  }
</style>
