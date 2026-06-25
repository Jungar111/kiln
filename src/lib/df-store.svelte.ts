// Mirrors `DfHandle` in `src-tauri/src/sidecar_client.rs`. Only the handle
// crosses the control plane; the bytes are paged directly over Arrow IPC.
export type DfHandle = {
  readonly handle: string;
  readonly rows: number;
  readonly cols: number;
  readonly schema: readonly string[];
};

export type DfStore = {
  readonly current: DfHandle | null;
  set(df: DfHandle): void;
  clear(): void;
};

/**
 * The most recently displayed DataFrame, shared app-wide so any execute (e.g. an
 * inspection REPL poke) can surface its frame in the Results pane's DataFrame tab.
 */
function createDfStore(): DfStore {
  let current = $state<DfHandle | null>(null);
  return {
    get current() {
      return current;
    },
    set(df: DfHandle) {
      current = df;
    },
    clear() {
      current = null;
    },
  };
}

export const dfStore: DfStore = createDfStore();
