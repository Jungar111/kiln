import { invoke } from '@tauri-apps/api/core';
import { SvelteSet } from 'svelte/reactivity';
import type { Verdict } from './checkpoint-types';

export type Run = {
  readonly run_id: string;
  readonly name: string;
  readonly status: string;
  readonly outcome: Verdict | null;
  readonly metrics: Readonly<Record<string, number>>;
  readonly params: Readonly<Record<string, string>>;
  readonly decisions: Readonly<Record<string, string>>;
  readonly look_here: readonly string[];
};

export type RunsStore = {
  readonly runs: readonly Run[];
  refresh(): Promise<void>;
  /** Reactive set of selected run ids (drives the compare view). */
  readonly selected: SvelteSet<string>;
};

export function createRunsStore(): RunsStore {
  let runs = $state<Run[]>([]);
  const selected = new SvelteSet<string>();

  async function refresh(): Promise<void> {
    const result = await invoke<readonly Run[]>('list_runs', { limit: 200 });
    runs = [...result];
  }

  return {
    get runs() {
      return runs;
    },
    refresh,
    selected,
  };
}
