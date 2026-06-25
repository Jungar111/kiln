//! Integration coverage for the drift detector (Ticket 72): a locked in-scope
//! slot value that would change re-fires the premise gate. Exercises the public
//! `kiln_lib::checkpoint` surface the way the chat loop does.

use kiln_lib::checkpoint::{LockedFrame, ProposeExperiment};

const SAMPLE_JSON: &str = r#"{
  "title": "Predict churn from the first 30 days",
  "premise": "Early engagement signals should separate churners from retained users.",
  "validation_strategy": {"in_scope": true, "severity": "critical", "answer": "temporal"},
  "target_definition": {"in_scope": true, "severity": "critical", "answer": "y = no login in days 31-60."},
  "feature_provenance": {"in_scope": true, "severity": "notable", "answer": "Days 0-30 only."},
  "preprocessing_fit_scope": {"in_scope": true, "severity": "notable", "answer": "Fit on train split only."},
  "data_scope_and_exclusions": {"in_scope": true, "severity": "fyi", "answer": "Drop accounts active < 7 days."},
  "missing_data_handling": {"in_scope": true, "severity": "notable", "answer": "Median impute on train."},
  "metric_choice": {"in_scope": true, "severity": "critical", "answer": "PR-AUC."},
  "look_here": []
}"#;

fn sample_proposal() -> ProposeExperiment {
    serde_json::from_str(SAMPLE_JSON).expect("deserialize sample proposal")
}

#[test]
fn changing_validation_strategy_triggers_drift() {
    let mut p1 = sample_proposal();
    p1.validation_strategy.answer = "temporal".into();
    let mut p2 = p1.clone();
    p2.validation_strategy.answer = "random".into();

    let lock = LockedFrame::snapshot(&p1);
    let drift = lock.diff(&p2);
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].slot, "validation_strategy");
    assert_eq!(drift[0].old, "temporal");
    assert_eq!(drift[0].new, "random");
}

#[test]
fn out_of_scope_slot_changes_are_ignored() {
    let mut p1 = sample_proposal();
    p1.metric_choice.in_scope = false;
    let mut p2 = p1.clone();
    p2.metric_choice.answer = "anything else".into();
    assert!(LockedFrame::snapshot(&p1).diff(&p2).is_empty());
}

#[test]
fn identical_reproposal_has_no_drift() {
    let p = sample_proposal();
    assert!(LockedFrame::snapshot(&p).diff(&p).is_empty());
}

#[test]
fn drift_entries_serialize_for_the_event_payload() {
    let mut p1 = sample_proposal();
    p1.validation_strategy.answer = "temporal".into();
    let mut p2 = p1.clone();
    p2.validation_strategy.answer = "random".into();

    let drift = LockedFrame::snapshot(&p1).diff(&p2);
    let json = serde_json::to_value(&drift).expect("serialize drift");
    let first = &json[0];
    assert_eq!(first["slot"], "validation_strategy");
    assert_eq!(first["old"], "temporal");
    assert_eq!(first["new"], "random");
}
