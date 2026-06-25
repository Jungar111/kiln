<script lang="ts">
  import { fetchPage, type DfPage } from '$lib/arrow-client';
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
      error = err instanceof Error ? err.message : String(err);
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
    gap: 8px;
    height: 100%;
    min-height: 0;
  }
  .scroll {
    flex: 1;
    overflow: auto;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
  }
  table {
    border-collapse: collapse;
    font-size: 12px;
    width: 100%;
  }
  th {
    position: sticky;
    top: 0;
    background: #1f1f1f;
    color: #e6e6e6;
    text-align: left;
    padding: 4px 8px;
    cursor: pointer;
    white-space: nowrap;
    border-bottom: 1px solid #333;
  }
  td {
    padding: 3px 8px;
    color: #ddd;
    white-space: nowrap;
    border-bottom: 1px solid #1f1f1f;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;
    color: #aaa;
  }
  button {
    background: #2a2a2a;
    color: #e6e6e6;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 3px 12px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .err {
    color: #e0866f;
    font-size: 12px;
    margin: 0;
  }
</style>
