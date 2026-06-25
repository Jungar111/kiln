import { invoke } from '@tauri-apps/api/core';
import type { Table } from 'apache-arrow';
import { tableFromIPC } from 'apache-arrow';

export type DfPage = {
  readonly columns: readonly string[];
  readonly rows: readonly (readonly unknown[])[];
};

export type SummaryRow = {
  readonly column: string;
  readonly dtype: string;
  readonly nulls: number;
  readonly min: number | null;
  readonly max: number | null;
  readonly mean: number | null;
};

export type PageOpts = {
  readonly offset: number;
  readonly limit: number;
  readonly sortBy?: string;
  readonly sortDir?: 'asc' | 'desc';
};

let cachedPort: number | null = null;

async function arrowPort(): Promise<number> {
  if (cachedPort !== null) return cachedPort;
  const port = await invoke<number>('arrow_port');
  cachedPort = port;
  return port;
}

async function postArrow(path: string, body: unknown): Promise<Table> {
  const port = await arrowPort();
  const response = await fetch(`http://127.0.0.1:${String(port)}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    throw new Error(`arrow ${path} failed: ${String(response.status)} ${await response.text()}`);
  }
  return tableFromIPC(new Uint8Array(await response.arrayBuffer()));
}

/** Read one column's value at row `i` as an opaque cell (any → unknown sink). */
function cell(table: Table, column: string, i: number): unknown {
  const vector = table.getChild(column);
  return vector === null ? null : vector.get(i);
}

function numOrNull(value: unknown): number | null {
  return typeof value === 'number' ? value : value === null ? null : Number(value);
}

export async function fetchPage(handle: string, opts: PageOpts): Promise<DfPage> {
  const table = await postArrow('/page', { handle, ...opts });
  const columns = table.schema.fields.map((field) => field.name);
  const rows: unknown[][] = [];
  for (let i = 0; i < table.numRows; i++) {
    rows.push(columns.map((column) => cell(table, column, i)));
  }
  return { columns, rows };
}

export async function fetchSummary(handle: string): Promise<SummaryRow[]> {
  const table = await postArrow('/summary', { handle });
  const out: SummaryRow[] = [];
  for (let i = 0; i < table.numRows; i++) {
    out.push({
      column: String(cell(table, 'column', i)),
      dtype: String(cell(table, 'dtype', i)),
      nulls: numOrNull(cell(table, 'nulls', i)) ?? 0,
      min: numOrNull(cell(table, 'min', i)),
      max: numOrNull(cell(table, 'max', i)),
      mean: numOrNull(cell(table, 'mean', i)),
    });
  }
  return out;
}
