"""Persist a checkpoint's declared decisions as MLflow run tags.

Storing the seven slots as `kiln.slot.*` tags makes "the thing approved" the
same object as "the thing compared" later (Ticket 60). The caller is expected to
have configured the tracking URI already.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Final, Literal

import mlflow

if TYPE_CHECKING:
    from kiln_sidecar.checkpoint import ProposeExperiment, Slot

Verdict = Literal["keep", "kill", "iterate"]
VERDICTS: Final[tuple[Verdict, ...]] = ("keep", "kill", "iterate")


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
    # Install the autolog gate (Ticket 71) so any ephemeral cell run while this
    # run is open is excluded from autolog. Idempotent — safe to call per
    # approval. The kernel installs the same gate in its own process via the
    # execute preamble; this call covers the sidecar process for defense in depth.
    from kiln_sidecar.autolog_gate import install_autolog_gate

    install_autolog_gate()
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


def close_run(run_id: str, verdict: Verdict) -> None:
    """Record the results-gate verdict and finish the run.

    Uses the client API (not the active-run context) so it works regardless of
    which run, if any, is currently active in the sidecar process.
    """
    client = mlflow.MlflowClient()
    client.set_tag(run_id, "kiln.outcome", verdict)
    client.set_terminated(run_id, status="FINISHED")
