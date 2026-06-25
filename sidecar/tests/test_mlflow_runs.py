from __future__ import annotations

from typing import TYPE_CHECKING, Final

import mlflow
import pytest

from kiln_sidecar.checkpoint import ProposeExperiment, Severity, Slot
from kiln_sidecar.execute import Executor
from kiln_sidecar.kernel import Kernel
from kiln_sidecar.mlflow_runs import close_run, start_run_with_decisions

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path


@pytest.fixture
def executor() -> Iterator[Executor]:
    kernel = Kernel()
    kernel.start()
    ex = Executor(kernel)
    try:
        yield ex
    finally:
        kernel.shutdown()


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


# A self-contained sklearn fit, run *inside the kernel* (where the autolog gate
# lives). It sets the tracking URI in the kernel, opens a run, enables autolog,
# fits, and prints the run_id so the test process can read the metrics back from
# the shared sqlite tracking db. `{db}` and the trailing run_id print are filled
# per-call.
_FIT_CELL: Final[str] = """
import mlflow
import numpy as np
from sklearn.linear_model import LogisticRegression

mlflow.set_tracking_uri("sqlite:///{db}")
mlflow.sklearn.autolog()
with mlflow.start_run() as _run:
    _X, _y = np.random.randn(100, 4), np.random.randint(0, 2, 100)
    LogisticRegression(max_iter=200).fit(_X, _y)
    print("KILN_RUN_ID=" + _run.info.run_id)
"""


def _run_fit_in_kernel(executor: Executor, db: Path, *, ephemeral: bool) -> str:
    result = executor.run(_FIT_CELL.format(db=db), ephemeral=ephemeral)
    assert result.status == "ok", result.traceback
    marker = "KILN_RUN_ID="
    line = next(ln for ln in result.stdout.splitlines() if ln.startswith(marker))
    return line.removeprefix(marker).strip()


def test_ephemeral_fit_not_autologged(tmp_path: Path, executor: Executor) -> None:
    db = tmp_path / "mlflow.db"
    run_id = _run_fit_in_kernel(executor, db, ephemeral=True)

    mlflow.set_tracking_uri(f"sqlite:///{db}")
    metrics = mlflow.get_run(run_id).data.metrics
    assert metrics == {}, f"ephemeral cell must not autolog metrics, got {metrics}"


def test_non_ephemeral_fit_is_autologged(tmp_path: Path, executor: Executor) -> None:
    # Regression guard: the gate must only suppress ephemeral calls.
    db = tmp_path / "mlflow.db"
    run_id = _run_fit_in_kernel(executor, db, ephemeral=False)

    mlflow.set_tracking_uri(f"sqlite:///{db}")
    metrics = mlflow.get_run(run_id).data.metrics
    assert metrics, "non-ephemeral fit must autolog metrics, got none"
