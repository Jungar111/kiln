import { listen } from '@tauri-apps/api/event';
import type { CheckpointProposed, DriftEntry, ProposeExperiment } from './checkpoint-types';

export type CheckpointStore = {
  readonly pending: ProposeExperiment | null;
  /** In-scope locked slots this pending proposal would change (Ticket 72). */
  readonly drift: readonly DriftEntry[];
  clear(): void;
};

/**
 * Holds the proposal awaiting a premise-gate decision. Fed by the
 * `checkpoint:proposed` event the Rust `chat` command emits when Claude's reply
 * carries a valid `kiln-experiment` block. The event also carries the drift
 * diff (empty unless a locked slot value would change) so the gate can re-fire
 * with a banner.
 */
export function createCheckpointStore(): CheckpointStore {
  let pending = $state<ProposeExperiment | null>(null);
  let drift = $state<readonly DriftEntry[]>([]);

  void listen<CheckpointProposed>('checkpoint:proposed', (event) => {
    pending = event.payload.proposal;
    drift = event.payload.drift;
  });

  return {
    get pending() {
      return pending;
    },
    get drift() {
      return drift;
    },
    clear() {
      pending = null;
      drift = [];
    },
  };
}
