from __future__ import annotations

from typing import TYPE_CHECKING

import mlflow

from kiln_sidecar.checkpoint import ProposeExperiment, Severity, Slot
from kiln_sidecar.mlflow_runs import close_run, start_run_with_decisions

if TYPE_CHECKING:
    from pathlib import Path


def _sample() -> ProposeExperiment:
    slot = Slot(in_scope=True, severity=Severity.CRITICAL, answer="temporal split")
    return ProposeExperiment(
        title="Churn from first 30 days",
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


def test_decisions_land_as_tags(tmp_path: Path) -> None:
    mlflow.set_tracking_uri(f"sqlite:///{tmp_path / 'mlflow.db'}")
    proposal = _sample()
    run_id = start_run_with_decisions(proposal)
    tags = mlflow.get_run(run_id).data.tags
    assert tags["kiln.slot.validation_strategy"] == proposal.validation_strategy.answer
    assert tags["kiln.slot.validation_strategy.severity"] == "critical"
    assert tags["kiln.title"] == proposal.title
    mlflow.end_run()


def test_close_run_writes_outcome_tag(tmp_path: Path) -> None:
    mlflow.set_tracking_uri(f"sqlite:///{tmp_path / 'mlflow.db'}")
    run_id = start_run_with_decisions(_sample())
    mlflow.end_run()  # release the active run before closing it by id
    close_run(run_id, "keep")
    run = mlflow.get_run(run_id)
    assert run.data.tags["kiln.outcome"] == "keep"
    assert run.info.status == "FINISHED"
