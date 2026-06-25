"""Persist a checkpoint's declared decisions as MLflow run tags.

Storing the seven slots as `kiln.slot.*` tags makes "the thing approved" the
same object as "the thing compared" later (Ticket 60). The caller is expected to
have configured the tracking URI already.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import mlflow

if TYPE_CHECKING:
    from kiln_sidecar.checkpoint import ProposeExperiment, Slot


def _slots(proposal: ProposeExperiment) -> dict[str, Slot]:
    return {
        "validation_strategy": proposal.validation_strategy,
        "target_definition": proposal.target_definition,
        "feature_provenance": proposal.feature_provenance,
        "preprocessing_fit_scope": proposal.preprocessing_fit_scope,
        "data_scope_and_exclusions": proposal.data_scope_and_exclusions,
        "missing_data_handling": proposal.missing_data_handling,
        "metric_choice": proposal.metric_choice,
    }


def start_run_with_decisions(proposal: ProposeExperiment) -> str:
    """Open an MLflow run, tag it with the declared decisions, return its id."""
    run = mlflow.start_run(run_name=proposal.title)
    run_id: str = run.info.run_id
    mlflow.set_tag("kiln.title", proposal.title)
    mlflow.set_tag("kiln.premise", proposal.premise)
    for name, slot in _slots(proposal).items():
        mlflow.set_tag(f"kiln.slot.{name}", slot.answer)
        mlflow.set_tag(f"kiln.slot.{name}.severity", slot.severity.value)
        mlflow.set_tag(f"kiln.slot.{name}.in_scope", str(slot.in_scope))
    mlflow.set_tag("kiln.look_here", "\n".join(proposal.look_here))
    return run_id
