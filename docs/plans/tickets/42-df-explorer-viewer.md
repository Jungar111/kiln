# Ticket 42 — DataFrame explorer viewer (paginated, sortable)

**Phase:** 5
**Depends on:** [40](./40-arrow-server.md), [41](./41-df-handle-hook.md), [20](./20-app-shell.md)
**Blocks:** [43](./43-summary-stats.md)

## Goal

Render a DataFrame in the Results pane: a virtualised table that fetches pages of 1_000 rows directly from the sidecar's Arrow Flight server, with column sort and a column-name filter. Bytes do not pass through Rust.

## Why a virtualised table

- Million-row frames are normal in data science. `<table>` with all rows in the DOM is dead on arrival.
- The viewer requests at most 1_000 rows around the visible window; sort/filter are server-side via Flight `do_action`.

## Files

- Add deps: `pnpm add apache-arrow @tanstack/svelte-virtual`.
- Create: `src/lib/arrow-client.ts`.
- Create: `src/lib/components/DataFrameView.svelte`.
- Modify: `src/lib/components/ResultsPane.svelte`.

## Steps

- [ ] **1. Failing UI smoke.** Open the app, run a cell returning a DataFrame, see only the existing JSON pretty-print of the handle (from Ticket 12's textarea).

- [ ] **2. Arrow client.**

  ```ts
  // src/lib/arrow-client.ts
  import { invoke } from '@tauri-apps/api/core';
  import { tableFromIPC } from 'apache-arrow';

  export type DfPage = { readonly columns: readonly string[]; readonly rows: readonly Record<string, unknown>[] };

  let cachedPort: number | null = null;

  async function arrowPort(): Promise<number> {
    if (cachedPort !== null) return cachedPort;
    const { port } = await invoke<{ port: number }>('rpc', { method: 'arrow_port' });
    cachedPort = port;
    return port;
  }

  export async function fetchPage(
    handle: string,
    opts: { readonly offset: number; readonly limit: number; readonly sortBy?: string; readonly sortDir?: 'asc' | 'desc' },
  ): Promise<DfPage> {
    const port = await arrowPort();
    // Use a Flight `do_action` named "page" with JSON args; the server slices the
    // registered table accordingly and returns a record-batch stream.
    const url = `http://127.0.0.1:${port}/page`;
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ handle, ...opts }),
    });
    const buf = new Uint8Array(await response.arrayBuffer());
    const table = tableFromIPC(buf);
    return { columns: table.schema.fields.map((f) => f.name), rows: [...table] as Record<string, unknown>[] };
  }
  ```

  (The sidecar exposes the `/page` HTTP path in this ticket; pure-Flight would require a JS Flight client which is heavier. The localhost HTTP path serves Arrow IPC bytes.)

- [ ] **3. Sidecar HTTP path.** Add a tiny `aiohttp` (or stdlib) HTTP server alongside Flight that handles `/page` and writes Arrow IPC. Page slicing uses pyarrow's `compute` for sort + filter.

- [ ] **4. Viewer component.** A virtualised list rendering 1_000 rows at a time, with a column header row that triggers re-fetch on click (toggles sort direction).

- [ ] **5. Wire into `ResultsPane.svelte`.** When `ExecuteResult.df != null`, mount `<DataFrameView handle={df.handle} columns={df.schema} />`.

- [ ] **6. Smoke test.**

  ```sh
  just dev
  # in the app: run `import pandas as pd; pd.DataFrame({'x': range(1_000_000)})`
  ```

  Verify the viewer paints, scrolls smoothly, and sorts by clicking the column.

- [ ] **7. Lint + commit.**

  ```sh
  git commit -m "feat(df): paginated, sortable DataFrame view over Arrow IPC"
  ```

## Acceptance

- One-million-row pandas frame opens without hanging.
- Memory usage stays bounded (verify with Activity Monitor — page caching ≤ 50 MB).
- No DataFrame bytes pass through a Tauri command (search `src-tauri/src/` for `pandas` / `arrow` and confirm absent).

## Out of scope

- Editing cells — out of MVP.
- Full-text search — out of MVP.
- Group-by / pivot — out of MVP.
