"""Typed wrapper over MLflow run search.

Surfaces the `kiln.slot.*` decision tags (Ticket 33) alongside params and metrics
so the comparison view can render declared decisions as first-class diff rows.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final

from mlflow.tracking import MlflowClient

KILN_PREFIX: Final[str] = "kiln.slot."
_DECISION_SUFFIXES: Final[tuple[str, ...]] = (".severity", ".in_scope")


@dataclass(frozen=True, slots=True)
class Run:
    run_id: str
    name: str
    status: str
    outcome: str | None
    metrics: dict[str, float]
    params: dict[str, str]
    decisions: dict[str, str]
    look_here: list[str]


def list_runs(experiment_names: list[str] | None, limit: int) -> list[Run]:
    client = MlflowClient()
    if experiment_names:
        experiment_ids = [
            exp.experiment_id
            for exp in (client.get_experiment_by_name(n) for n in experiment_names)
            if exp is not None
        ]
    else:
        experiment_ids = [exp.experiment_id for exp in client.search_experiments()]

    runs = client.search_runs(
        experiment_ids=experiment_ids,
        max_results=limit,
        order_by=["start_time DESC"],
    )

    out: list[Run] = []
    for run in runs:
        tags: dict[str, str] = dict(run.data.tags)
        decisions = {
            key.removeprefix(KILN_PREFIX): value
            for key, value in tags.items()
            if key.startswith(KILN_PREFIX) and not key.endswith(_DECISION_SUFFIXES)
        }
        look_here = [line for line in tags.get("kiln.look_here", "").split("\n") if line]
        out.append(
            Run(
                run_id=run.info.run_id,
                name=tags.get("kiln.title", run.info.run_name or ""),
                status=run.info.status,
                outcome=tags.get("kiln.outcome"),
                metrics=dict(run.data.metrics),
                params=dict(run.data.params),
                decisions=decisions,
                look_here=look_here,
            )
        )
    return out
