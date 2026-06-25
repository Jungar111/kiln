// Mirror of `src-tauri/src/checkpoint.rs` (serde serialises snake_case fields and
// lowercase severities). Arrives over the `checkpoint:proposed` Tauri event.

export type Severity = 'critical' | 'notable' | 'fyi';

export type Slot = {
  readonly in_scope: boolean;
  readonly severity: Severity;
  readonly answer: string;
};

export type ProposeExperiment = {
  readonly title: string;
  readonly premise: string;
  readonly validation_strategy: Slot;
  readonly target_definition: Slot;
  readonly feature_provenance: Slot;
  readonly preprocessing_fit_scope: Slot;
  readonly data_scope_and_exclusions: Slot;
  readonly missing_data_handling: Slot;
  readonly metric_choice: Slot;
  readonly look_here: readonly string[];
};

/** The seven required slots, in render order, with human labels. */
export const SLOT_FIELDS = [
  ['validation_strategy', 'Validation strategy'],
  ['target_definition', 'Target definition'],
  ['feature_provenance', 'Feature provenance'],
  ['preprocessing_fit_scope', 'Preprocessing fit scope'],
  ['data_scope_and_exclusions', 'Data scope & exclusions'],
  ['missing_data_handling', 'Missing-data handling'],
  ['metric_choice', 'Metric choice'],
] as const;

export type SlotKey = (typeof SLOT_FIELDS)[number][0];

/** Human label for a slot key, for drift banners and badges. */
export function slotLabel(key: string): string {
  return SLOT_FIELDS.find(([k]) => k === key)?.[1] ?? key;
}

/**
 * One locked in-scope slot whose value would change — mirror of Rust
 * `DriftEntry` (Ticket 72). Rides along on the `checkpoint:proposed` event.
 */
export type DriftEntry = {
  readonly slot: SlotKey;
  readonly old: string;
  readonly new: string;
};

/** Payload of the `checkpoint:proposed` event — mirror of Rust `CheckpointProposed`. */
export type CheckpointProposed = {
  readonly proposal: ProposeExperiment;
  readonly drift: readonly DriftEntry[];
};

/** Results-gate outcomes (spec §3.4). */
export type Verdict = 'keep' | 'kill' | 'iterate';
