"""Contract test for the checkpoint schema.

The SAMPLE_JSON below is kept byte-identical to `SAMPLE_JSON` in
`src-tauri/src/checkpoint.rs` — if both languages parse it, the cross-language
contract holds.
"""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from kiln_sidecar.checkpoint import ProposeExperiment, Severity

SAMPLE_JSON: str = """{
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
}"""


def test_deserializes_sample_proposal() -> None:
    proposal = ProposeExperiment.model_validate_json(SAMPLE_JSON)
    assert proposal.title == "Predict churn from the first 30 days"
    assert proposal.metric_choice.severity is Severity.CRITICAL
    assert proposal.validation_strategy.in_scope is True
    assert len(proposal.look_here) == 1


def test_empty_answer_is_rejected_by_pydantic() -> None:
    with pytest.raises(ValidationError):
        ProposeExperiment.model_validate_json(
            SAMPLE_JSON.replace("PR-AUC; classes are imbalanced.", "")
        )
