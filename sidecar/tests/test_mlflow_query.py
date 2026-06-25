from __future__ import annotations

from typing import TYPE_CHECKING

import mlflow
import pytest

from kiln_sidecar.checkpoint import ProposeExperiment, Severity, Slot
from kiln_sidecar.mlflow_query import list_runs
from kiln_sidecar.mlflow_runs import start_run_with_decisions

if TYPE_CHECKING:
    from pathlib import Path


def _sample() -> ProposeExperiment:
    slot = Slot(in_scope=True, severity=Severity.CRITICAL, answer="temporal split")
    return ProposeExperiment(
        title="Churn",
        premise="Early engagement separates churners.",
        validation_strategy=slot,
        target_definition=slot,
        feature_provenance=slot,
        preprocessing_fit_scope=slot,
        data_scope_and_exclusions=slot,
        missing_data_handling=slot,
        metric_choice=slot,
        look_here=["Is the horizon right?"],
    )


def test_list_runs_returns_kiln_tags(tmp_path: Path) -> None:
    mlflow.set_tracking_uri(f"sqlite:///{tmp_path / 'mlflow.db'}")
    proposal = _sample()
    run_id = start_run_with_decisions(proposal)
    mlflow.log_metric("accuracy", 0.91)
    mlflow.end_run()

    runs = list_runs(experiment_names=None, limit=10)
    assert len(runs) == 1
    assert runs[0].run_id == run_id
    assert runs[0].metrics["accuracy"] == pytest.approx(0.91)
    assert runs[0].decisions["validation_strategy"] == proposal.validation_strategy.answer
    # severity/in_scope suffixes are not decisions
    assert "validation_strategy.severity" not in runs[0].decisions
    assert runs[0].look_here == ["Is the horizon right?"]
