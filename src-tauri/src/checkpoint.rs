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

impl ProposeExperiment {
    /// Reject empty consequential slots at the tool boundary (spec §9): a
    /// required slot that is in scope must carry a non-empty answer.
    pub fn validate(&self) -> Vec<SlotError> {
        let pairs: [(&'static str, &Slot); 7] = [
            ("validation_strategy", &self.validation_strategy),
            ("target_definition", &self.target_definition),
            ("feature_provenance", &self.feature_provenance),
            ("preprocessing_fit_scope", &self.preprocessing_fit_scope),
            ("data_scope_and_exclusions", &self.data_scope_and_exclusions),
            ("missing_data_handling", &self.missing_data_handling),
            ("metric_choice", &self.metric_choice),
        ];
        pairs
            .into_iter()
            .filter(|(_, slot)| slot.in_scope && slot.answer.trim().is_empty())
            .map(|(slot, _)| SlotError {
                slot,
                reason: "empty in-scope answer",
            })
            .collect()
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
    use super::{ProposeExperiment, Severity, SAMPLE_JSON};

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
