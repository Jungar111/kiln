# Ticket 72 — Drift detector: re-gate on locked-slot change

**Phase:** 8
**Depends on:** [33](./33-mlflow-tag-write.md), [31](./31-slot-validation.md)
**Blocks:** none in MVP

## Goal

Detect when Claude's subsequent `propose_experiment` calls (within the same investigation) would change a **locked** slot value — and re-fire the premise gate when they would.

> Spec §3.3 — *"Re-gate on premise drift. If Claude's work would change a locked frame decision (target, validation strategy, feature provenance, metric, population), a new premise gate fires."*

Decision: drift is detected by **value comparison** across consecutive proposals, scoped to a single active run (the one MLflow opened in Ticket 33). When the human approves the first proposal, the in-scope slot answers are *snapshotted* as the "locked frame." Later proposals are diffed against the snapshot. Any in-scope slot whose answer differs *meaningfully* (case- and whitespace-insensitive string compare for MVP) re-fires the gate.

## Files

- Modify: `src-tauri/src/checkpoint.rs` — add `locked_frame: Option<LockedFrame>` to the Tauri-managed state.
- Modify: `src-tauri/src/commands.rs` — `approve_checkpoint` writes the lock; new internal helper `check_drift(new_proposal)` returns the list of changed slots.
- Modify: `src-tauri/src/claude.rs` — after validation but before emitting `sidecar:checkpoint`, run drift check. If any in-scope slot changed, route to a `sidecar:drift_detected` event with the diff.
- Modify: `src/lib/components/PremiseGate.svelte` — when payload has `drift: { slot, old, new }[]`, foreground those slots and label "drift" in the header.

## Steps

- [ ] **1. Failing test.**

  ```rust
  // src-tauri/tests/drift.rs
  #[test]
  fn changing_validation_strategy_triggers_drift() {
      let mut p1 = sample_proposal();
      p1.validation_strategy.answer = "temporal".into();
      let mut p2 = p1.clone();
      p2.validation_strategy.answer = "random".into();

      let lock = LockedFrame::from(&p1);
      let drift = lock.diff(&p2);
      assert_eq!(drift.len(), 1);
      assert_eq!(drift[0].slot, "validation_strategy");
  }

  #[test]
  fn out_of_scope_slot_changes_are_ignored() {
      let mut p1 = sample_proposal();
      p1.metric_choice.in_scope = false;
      let mut p2 = p1.clone();
      p2.metric_choice.answer = "anything else".into();
      assert!(LockedFrame::from(&p1).diff(&p2).is_empty());
  }
  ```

- [ ] **2. Implement.**

  ```rust
  #[derive(Debug, Clone)]
  pub struct LockedFrame {
      slots: std::collections::HashMap<&'static str, String>,
  }

  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
  pub struct DriftEntry { pub slot: &'static str, pub old: String, pub new: String }

  impl LockedFrame {
      pub fn from(p: &ProposeExperiment) -> Self {
          let mut slots = std::collections::HashMap::new();
          for (name, slot) in iter_slots(p) {
              if slot.in_scope {
                  slots.insert(name, slot.answer.trim().to_ascii_lowercase());
              }
          }
          Self { slots }
      }

      pub fn diff(&self, p: &ProposeExperiment) -> Vec<DriftEntry> {
          let mut out = Vec::new();
          for (name, slot) in iter_slots(p) {
              if !slot.in_scope { continue; }
              if let Some(old) = self.slots.get(name) {
                  let new = slot.answer.trim().to_ascii_lowercase();
                  if &new != old {
                      out.push(DriftEntry { slot: name, old: old.clone(), new: slot.answer.clone() });
                  }
              }
          }
          out
      }
  }

  fn iter_slots(p: &ProposeExperiment) -> [(&'static str, &Slot); 7] {
      [
          ("validation_strategy", &p.validation_strategy),
          ("target_definition", &p.target_definition),
          ("feature_provenance", &p.feature_provenance),
          ("preprocessing_fit_scope", &p.preprocessing_fit_scope),
          ("data_scope_and_exclusions", &p.data_scope_and_exclusions),
          ("missing_data_handling", &p.missing_data_handling),
          ("metric_choice", &p.metric_choice),
      ]
  }
  ```

- [ ] **3. Wire into the chat loop.** After validation but before emitting `sidecar:checkpoint`:

  ```rust
  if let Some(lock) = state.locked_frame.lock().unwrap().as_ref() {
      let drift = lock.diff(&proposal);
      if !drift.is_empty() {
          app.emit("sidecar:drift_detected", &drift).ok();
      }
  }
  ```

- [ ] **4. UI.** `PremiseGate.svelte` receives an optional `drift?: readonly DriftEntry[]` prop. When present, render at the top a banner "Drift detected" listing changed slots and their old → new values. The matching `SlotRow`s in the body show a red "DRIFT" badge.

- [ ] **5. Smoke test.**

  - Approve a proposal with `validation_strategy = "temporal"`.
  - Ask Claude to switch to `validation_strategy = "random"`.
  - Confirm the new gate fires with the drift banner.

- [ ] **6. Lint + commit.**

  ```sh
  git commit -m "feat(drift): re-gate when a locked in-scope slot value changes"
  ```

## Acceptance

- Unit tests pass.
- The first proposal does not show a drift banner (no lock yet).
- The second-and-later proposal shows the drift banner if and only if at least one in-scope slot changed (case/whitespace-insensitive).
- Releasing the lock (results gate fires) clears the snapshot — see Ticket 34's outcome handlers.

## Out of scope

- Heuristics beyond exact-value compare (e.g. "temporal-CV" vs "TimeSeriesSplit" should be the same) — fast-follow; spec §11 open question.
- Surfacing drift in the chat thread itself — fast-follow.
- Diffing the schema versions of `propose_experiment` itself — out of MVP.
