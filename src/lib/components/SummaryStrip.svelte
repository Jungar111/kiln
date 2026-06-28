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
    padding-bottom: 6px;
  }
  .col {
    min-width: 120px;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    padding: 6px 8px;
    background: #161616;
    font-size: 11px;
    color: #aaa;
  }
  .name {
    color: #e6e6e6;
    font-weight: 600;
    white-space: nowrap;
  }
  .dtype {
    color: #9ad;
    margin-bottom: 4px;
  }
  .err {
    color: #e0866f;
    font-size: 12px;
  }
</style>
