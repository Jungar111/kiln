//! The "checkpoint object" — the structured `propose_experiment` payload Claude
//! fills in before the premise gate (spec §4). This Rust type and the Python
//! Pydantic mirror in `sidecar/src/kiln_sidecar/checkpoint.py` are the schema's
//! source of truth; the cross-language contract is exercised by deserializing
//! the same sample JSON in both languages' tests.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Notable,
    Fyi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub in_scope: bool,
    pub severity: Severity,
    /// Free-text answer or "N/A". Must be non-empty when `in_scope` is true —
    /// enforced by [`ProposeExperiment::validate`] (Ticket 31), not serde.
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeExperiment {
    pub title: String,
    pub premise: String,
    pub validation_strategy: Slot,
    pub target_definition: Slot,
    pub feature_provenance: Slot,
    pub preprocessing_fit_scope: Slot,
    pub data_scope_and_exclusions: Slot,
    pub missing_data_handling: Slot,
    pub metric_choice: Slot,
    /// Where Claude is least sure / most wants the human's eye.
    pub look_here: Vec<String>,
}

/// One failed required-slot check.
#[derive(Debug, PartialEq, Eq)]
pub struct SlotError {
    pub slot: &'static str,
    pub reason: &'static str,
}

/// The seven locked decision slots, paired with their canonical names, in the
/// order the gate renders them. Single source of truth for validation and drift.
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

impl ProposeExperiment {
    /// Reject empty consequential slots at the tool boundary (spec §9): a
    /// required slot that is in scope must carry a non-empty answer.
    pub fn validate(&self) -> Vec<SlotError> {
        iter_slots(self)
            .into_iter()
            .filter(|(_, slot)| slot.in_scope && slot.answer.trim().is_empty())
            .map(|(slot, _)| SlotError {
                slot,
                reason: "empty in-scope answer",
            })
            .collect()
    }
}

/// Normalise a slot answer for value comparison: trim and lowercase so trivial
/// whitespace/case edits don't read as drift (spec §3.3, MVP exact-value compare).
fn normalise(answer: &str) -> String {
    answer.trim().to_lowercase()
}

/// The locked frame: a snapshot of the in-scope slot answers taken when the
/// human approves the first proposal of an investigation. Later proposals are
/// diffed against it to detect premise drift (spec §3.3).
#[derive(Debug, Clone)]
pub struct LockedFrame {
    slots: std::collections::HashMap<&'static str, String>,
}

/// One slot whose locked value would change. `old`/`new` carry the original
/// (un-normalised) answers so the UI can show a human-readable before → after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriftEntry {
    pub slot: &'static str,
    pub old: String,
    pub new: String,
}

impl LockedFrame {
    /// Snapshot the in-scope slots of an approved proposal. Out-of-scope slots
    /// are intentionally omitted so they can't trigger drift later.
    pub fn snapshot(p: &ProposeExperiment) -> Self {
        let slots = iter_slots(p)
            .into_iter()
            .filter(|(_, slot)| slot.in_scope)
            .map(|(name, slot)| (name, slot.answer.clone()))
            .collect();
        Self { slots }
    }

    /// Return the in-scope slots whose locked value would change under `p`.
    /// Comparison is case- and whitespace-insensitive (MVP). A slot the human
    /// never locked (out of scope at approval) never reports drift, and a slot
    /// that is out of scope in `p` is skipped — we only re-gate when a still
    /// active locked decision actually flips.
    pub fn diff(&self, p: &ProposeExperiment) -> Vec<DriftEntry> {
        iter_slots(p)
            .into_iter()
            .filter(|(_, slot)| slot.in_scope)
            .filter_map(|(name, slot)| {
                let old = self.slots.get(name)?;
                (normalise(old) != normalise(&slot.answer)).then(|| DriftEntry {
                    slot: name,
                    old: old.clone(),
                    new: slot.answer.clone(),
                })
            })
            .collect()
    }
}

/// Tauri-managed state holding the locked frame for the active investigation.
/// Written when a premise gate is approved (Ticket 33's run opens), read by the
/// chat loop to diff later proposals, and cleared when the results gate closes
/// the run (Ticket 34). `None` means no run is open, so no proposal can drift.
#[derive(Debug, Default)]
pub struct DriftState {
    locked_frame: std::sync::Mutex<Option<LockedFrame>>,
}

impl DriftState {
    /// Snapshot an approved proposal as the new locked frame.
    pub fn lock(&self, p: &ProposeExperiment) {
        *self.guard() = Some(LockedFrame::snapshot(p));
    }

    /// Release the lock (the run closed). Idempotent.
    pub fn release(&self) {
        *self.guard() = None;
    }

    /// Diff `p` against the current lock, if any. Empty when nothing is locked.
    pub fn check_drift(&self, p: &ProposeExperiment) -> Vec<DriftEntry> {
        match self.guard().as_ref() {
            Some(frame) => frame.diff(p),
            None => Vec::new(),
        }
    }

    /// Recover from a poisoned mutex rather than propagating a panic across the
    /// IPC boundary: the lock guards only a snapshot, so the worst a poisoned
    /// lock costs is a stale frame, which the next approval overwrites.
    fn guard(&self) -> std::sync::MutexGuard<'_, Option<LockedFrame>> {
        self.locked_frame
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
pub(crate) const SAMPLE_JSON: &str = r#"{
  "title": "Predict churn from the first 30 days",
  "premise": "Early engagement signals should separate churners from retained users.",
  "validation_strategy": {"in_scope": true, "severity": "critical", "answer": "Temporal split: train on cohorts before 2025-01, test after."},
  "target_definition": {"in_scope": true, "severity": "critical", "answer": "y = no login in days 31-60. No lookahead."},
  "feature_provenance": {"in_scope": true, "severity": "notable", "answer": "All features from days 0-30 only."},
  "preprocessing_fit_scope": {"in_scope": true, "severity": "notable", "answer": "Scaler fit on train split only."},
  "data_scope_and_exclusions": {"in_scope": true, "severity": "fyi", "answer": "Drop accounts active < 7 days."},
  "missing_data_handling": {"in_scope": true, "severity": "notable", "answer": "Median impute, fit on train."},
  "metric_choice": {"in_scope": true, "severity": "critical", "answer": "PR-AUC; classes are imbalanced."},
  "look_here": ["Is the 31-60 day churn window the right horizon?"]
}"#;

#[cfg(test)]
mod tests {
    use super::{DriftState, LockedFrame, ProposeExperiment, Severity, SAMPLE_JSON};

    fn sample() -> ProposeExperiment {
        serde_json::from_str(SAMPLE_JSON).expect("deserialize sample")
    }

    #[test]
    fn drift_state_starts_unlocked() {
        let state = DriftState::default();
        let mut p = sample();
        p.validation_strategy.answer = "totally different".into();
        // No lock yet — the first proposal never drifts.
        assert!(state.check_drift(&p).is_empty());
    }

    #[test]
    fn drift_state_locks_then_detects_then_releases() {
        let state = DriftState::default();
        let p1 = sample();
        state.lock(&p1);

        let mut p2 = p1.clone();
        p2.validation_strategy.answer = "random".into();
        assert_eq!(state.check_drift(&p2).len(), 1);

        // Closing the run clears the snapshot; later proposals can't drift.
        state.release();
        assert!(state.check_drift(&p2).is_empty());
    }

    #[test]
    fn no_change_means_no_drift() {
        let p = sample();
        assert!(LockedFrame::snapshot(&p).diff(&p).is_empty());
    }

    #[test]
    fn changing_an_in_scope_slot_is_drift() {
        let p1 = sample();
        let mut p2 = p1.clone();
        p2.validation_strategy.answer = "random k-fold".into();
        let drift = LockedFrame::snapshot(&p1).diff(&p2);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].slot, "validation_strategy");
        assert_eq!(drift[0].old, p1.validation_strategy.answer);
        assert_eq!(drift[0].new, "random k-fold");
    }

    #[test]
    fn whitespace_and_case_only_changes_are_not_drift() {
        let p1 = sample();
        let mut p2 = p1.clone();
        p2.metric_choice.answer = format!("  {}  ", p1.metric_choice.answer.to_uppercase());
        assert!(LockedFrame::snapshot(&p1).diff(&p2).is_empty());
    }

    #[test]
    fn out_of_scope_slot_changes_are_ignored() {
        let mut p1 = sample();
        p1.metric_choice.in_scope = false;
        let mut p2 = p1.clone();
        p2.metric_choice.answer = "anything else".into();
        assert!(LockedFrame::snapshot(&p1).diff(&p2).is_empty());
    }

    #[test]
    fn slot_unlocked_at_approval_never_drifts() {
        // Out of scope when locked, then back in scope with a different value:
        // there is no locked value to compare against, so no drift.
        let mut p1 = sample();
        p1.feature_provenance.in_scope = false;
        let lock = LockedFrame::snapshot(&p1);
        let mut p2 = p1.clone();
        p2.feature_provenance.in_scope = true;
        p2.feature_provenance.answer = "leaked features from the future".into();
        assert!(lock.diff(&p2).is_empty());
    }

    #[test]
    fn multiple_slot_changes_all_reported() {
        let p1 = sample();
        let mut p2 = p1.clone();
        p2.validation_strategy.answer = "random".into();
        p2.target_definition.answer = "different label".into();
        let mut drift = LockedFrame::snapshot(&p1).diff(&p2);
        drift.sort_by_key(|d| d.slot);
        let slots: Vec<&str> = drift.iter().map(|d| d.slot).collect();
        assert_eq!(slots, ["target_definition", "validation_strategy"]);
    }

    #[test]
    fn deserializes_sample_proposal() {
        let p: ProposeExperiment = serde_json::from_str(SAMPLE_JSON).expect("deserialize sample");
        assert_eq!(p.title, "Predict churn from the first 30 days");
        assert_eq!(p.metric_choice.severity, Severity::Critical);
        assert!(p.validation_strategy.in_scope);
        assert_eq!(p.look_here.len(), 1);
    }

    #[test]
    fn valid_proposal_has_no_errors() {
        let p: ProposeExperiment = serde_json::from_str(SAMPLE_JSON).unwrap();
        assert!(p.validate().is_empty());
    }

    #[test]
    fn empty_in_scope_answer_is_rejected() {
        let mut p: ProposeExperiment = serde_json::from_str(SAMPLE_JSON).unwrap();
        p.validation_strategy.answer = String::new();
        let errors = p.validate();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].slot, "validation_strategy");
    }

    #[test]
    fn out_of_scope_empty_answer_is_allowed() {
        let mut p: ProposeExperiment = serde_json::from_str(SAMPLE_JSON).unwrap();
        p.feature_provenance.in_scope = false;
        p.feature_provenance.answer = String::new();
        assert!(p.validate().is_empty());
    }
}
