/**
 * Holds the rich MIME displays (matplotlib PNG, plotly HTML, …) for every
 * executed cell that produced any, so the Plot tab can render the full history.
 *
 * Two feed paths, both supported so the integrator can wire whichever fits:
 *
 *  1. `setDisplays(...)` — call it with the `displays` array off any
 *     `ExecuteResponse` (the `execute` Tauri command returns it). This is the
 *     direct path for a frontend-driven execute loop (e.g. the experiment runner
 *     or the inspection REPL).
 *  2. The `plot:displays` Tauri event — a future Rust-side experiment-execute
 *     path can push displays without a webview round-trip. The store subscribes
 *     on creation; no teardown is needed for the app lifetime.
 *
 * The `Display` shape mirrors `Display` in `src-tauri/src/sidecar_client.rs`
 * and `kiln_sidecar.execute.Display`.
 */

import { listen } from '@tauri-apps/api/event';

export type Display = {
  readonly mime: string;
  readonly payload: string;
  readonly metadata: Readonly<Record<string, unknown>>;
};

export type PlotStore = {
  /** One entry per executed cell that produced displays, oldest → newest. */
  readonly history: readonly (readonly Display[])[];
  /** Append the displays from an ExecuteResponse. Empty arrays are ignored so a
   *  plain `x = 1` cell does not add a blank history entry. */
  setDisplays(displays: readonly Display[]): void;
  /** Drop the whole history (e.g. when a gate is declined / the run resets). */
  clear(): void;
};

/** Type guard for the loosely-typed `plot:displays` event payload. */
function isDisplay(value: unknown): value is Display {
  if (typeof value !== 'object' || value === null) return false;
  const record = value as Record<string, unknown>;
  return typeof record.mime === 'string' && typeof record.payload === 'string';
}

export function createPlotStore(): PlotStore {
  let history = $state<readonly (readonly Display[])[]>([]);

  function setDisplays(next: readonly Display[]): void {
    // Append, don't replace: every plotting cell keeps its own entry so the
    // Plot tab is a history. Display-less cells are ignored (no blank entry).
    if (next.length > 0) history = [...history, next];
  }

  // listen() returns a Promise<UnlistenFn>; we deliberately don't await it in
  // this sync factory. `void` satisfies no-floating-promises.
  void listen<readonly unknown[]>('plot:displays', (event) => {
    const payload = event.payload;
    if (Array.isArray(payload)) {
      setDisplays(payload.filter(isDisplay));
    }
  });

  return {
    get history() {
      return history;
    },
    setDisplays,
    clear() {
      history = [];
    },
  };
}

/** Shared instance so any execute (e.g. the inspection REPL) feeds the Plot tab. */
export const plotStore: PlotStore = createPlotStore();
