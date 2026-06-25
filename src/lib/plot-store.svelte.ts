/**
 * Holds the rich MIME displays (matplotlib PNG, plotly HTML, …) for the most
 * recently executed cell, so the Plot tab can render them.
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
  /** Displays from the latest cell that produced any (cleared = empty array). */
  readonly displays: readonly Display[];
  /** Feed displays directly from an ExecuteResponse. Empty arrays are ignored
   *  so a plain `x = 1` cell does not blank out the last plot. */
  setDisplays(displays: readonly Display[]): void;
  /** Drop the current displays (e.g. when a gate is declined / the run resets). */
  clear(): void;
};

/** Type guard for the loosely-typed `plot:displays` event payload. */
function isDisplay(value: unknown): value is Display {
  if (typeof value !== 'object' || value === null) return false;
  const record = value as Record<string, unknown>;
  return typeof record.mime === 'string' && typeof record.payload === 'string';
}

export function createPlotStore(): PlotStore {
  let displays = $state<readonly Display[]>([]);

  function setDisplays(next: readonly Display[]): void {
    // Ignore display-less cells: keep showing the last real plot rather than
    // flashing empty every time a non-plotting cell runs.
    if (next.length > 0) displays = next;
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
    get displays() {
      return displays;
    },
    setDisplays,
    clear() {
      displays = [];
    },
  };
}
