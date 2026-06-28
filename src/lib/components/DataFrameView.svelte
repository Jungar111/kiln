<script lang="ts">
  import { fetchPage, type DfPage } from '$lib/arrow-client';
  import { toMessage } from '$lib/errors';
  import SummaryStrip from './SummaryStrip.svelte';

  let { handle, rows: totalRows }: { handle: string; rows: number } = $props();

  const PAGE = 1000;

  let page = $state<DfPage>({ columns: [], rows: [] });
  let offset = $state(0);
  let sortBy = $state<string | null>(null);
  let sortDir = $state<'asc' | 'desc'>('asc');
  let error = $state<string | null>(null);
  let loadedHandle = '';

  function display(value: unknown): string {
    if (value === null || value === undefined) return '';
    if (typeof value === 'bigint') return value.toString();
    if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'string') {
      return String(value);
    }
    return JSON.stringify(value);
  }

  async function load(): Promise<void> {
    try {
      const opts =
        sortBy === null ? { offset, limit: PAGE } : { offset, limit: PAGE, sortBy, sortDir };
      page = await fetchPage(handle, opts);
      error = null;
    } catch (err) {
      error = toMessage(err);
    }
  }

  function sortColumn(column: string): void {
    if (sortBy === column) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = column;
      sortDir = 'asc';
    }
    offset = 0;
    void load();
  }

  function nextPage(): void {
    if (offset + PAGE < totalRows) {
      offset += PAGE;
      void load();
    }
  }

  function prevPage(): void {
    if (offset > 0) {
      offset = Math.max(0, offset - PAGE);
      void load();
    }
  }

  // Reload from scratch whenever a different DataFrame is shown.
  $effect(() => {
    if (handle !== loadedHandle) {
      loadedHandle = handle;
      offset = 0;
      sortBy = null;
      sortDir = 'asc';
      void load();
    }
  });
</script>

<div class="df">
  <div class="bar">
    <span class="df-name">df</span>
    <span class="df-shape">{totalRows.toLocaleString()} × {page.columns.length}</span>
    <span class="sep">·</span>
    <span class="ok">zero-copy</span>
    <span class="sep">·</span>
    <span class="muted">server-side sort / filter</span>
  </div>

  <SummaryStrip {handle} />

  {#if error}
    <p class="err">{error}</p>
  {/if}

  <div class="scroll">
    <table>
      <thead>
        <tr>
          {#each page.columns as column (column)}
            <th
              class:sorted={sortBy === column}
              onclick={() => {
                sortColumn(column);
              }}
            >
              {column}{sortBy === column ? (sortDir === 'asc' ? ' ▲' : ' ▼') : ''}
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each page.rows as row, i (i)}
          <tr>
            {#each row as value, c (c)}<td>{display(value)}</td>{/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <footer>
    <button type="button" onclick={prevPage} disabled={offset === 0}>prev</button>
    <span>
      rows {totalRows === 0 ? 0 : offset + 1}–{Math.min(offset + PAGE, totalRows)} of {totalRows}
    </span>
    <button type="button" onclick={nextPage} disabled={offset + PAGE >= totalRows}>next</button>
  </footer>
</div>

<style>
  .df {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .bar {
    height: 42px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 16px;
    border-bottom: 1px solid var(--bd);
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .df-name {
    color: var(--ember-soft);
  }
  .df-shape {
    color: var(--tx-dim2);
  }
  .sep {
    color: var(--tx-mut3);
  }
  .ok {
    color: var(--good);
  }
  .muted {
    color: var(--tx-dim2);
  }
  .scroll {
    flex: 1;
    overflow: auto;
  }
  table {
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 11.5px;
    width: 100%;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--bg-panel-2);
    color: var(--tx-2);
    text-align: left;
    padding: 7px 10px;
    cursor: pointer;
    white-space: nowrap;
    border-bottom: 1px solid var(--bd-2);
    border-right: 1px solid var(--bd-row);
    font-weight: 400;
  }
  th.sorted {
    color: var(--ember-soft);
  }
  td {
    padding: 6px 10px;
    color: var(--tx);
    white-space: nowrap;
    border-bottom: 1px solid var(--bd-row);
  }
  tbody tr:nth-child(even) {
    background: var(--bg-row-alt);
  }
  footer {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    border-top: 1px solid var(--bd);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--tx-dim);
  }
  button {
    background: var(--bg-panel-2);
    color: var(--tx-2);
    border: 1px solid var(--bd-2);
    border-radius: 6px;
    padding: 3px 12px;
    cursor: pointer;
    font: inherit;
    font-size: 11px;
  }
  button:hover:not(:disabled) {
    border-color: var(--bd-ember);
  }
  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .err {
    color: var(--bad);
    font-size: 12px;
    margin: 8px 16px;
  }
</style>
