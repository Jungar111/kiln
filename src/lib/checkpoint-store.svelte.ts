import { listen } from '@tauri-apps/api/event';
import type { ProposeExperiment } from './checkpoint-types';

export type CheckpointStore = {
  readonly pending: ProposeExperiment | null;
  clear(): void;
};

/**
 * Holds the proposal awaiting a premise-gate decision. Fed by the
 * `checkpoint:proposed` event the Rust `chat` command emits when Claude's reply
 * carries a valid `kiln-experiment` block.
 */
export function createCheckpointStore(): CheckpointStore {
  let pending = $state<ProposeExperiment | null>(null);

  void listen<ProposeExperiment>('checkpoint:proposed', (event) => {
    pending = event.payload;
  });

  return {
    get pending() {
      return pending;
    },
    clear() {
      pending = null;
    },
  };
}
