# Ticket 71 — Enforce experiment-vs-inspection at the MLflow boundary

**Phase:** 8
**Depends on:** [33](./33-mlflow-tag-write.md), [70](./70-repl-panel.md)
**Blocks:** none in MVP

## Goal

Honour the `ephemeral` flag from Ticket 05 at the MLflow autolog boundary: when `ephemeral=True`, the kernel call must **not** be tracked. Autolog hooks see only experiment commands; REPL pokes vanish from the run record.

> Spec §7 — *"Inspection is ephemeral; the experiment is the record. Manual human pokes must not land in the logged MLflow run or the promoted script."*

## Why MLflow's API does not solve this for us

`mlflow.<flavor>.autolog()` patches `.fit` globally. It doesn't know about our distinction. We have to *suppress* autolog around ephemeral calls. The simplest mechanism in MVP is a thread-local context flag that wraps the autolog patches.

## Files

- Modify: `sidecar/src/kiln_sidecar/execute.py` — set `kiln_state.is_ephemeral` for the duration of the call.
- Create: `sidecar/src/kiln_sidecar/autolog_gate.py`.
- Modify: `sidecar/src/kiln_sidecar/mlflow_runs.py` — install the gate during `start_run_with_decisions`.
- Modify: `sidecar/tests/test_mlflow_runs.py` — add a test that asserts an ephemeral `.fit` does not log metrics.

## Steps

- [ ] **1. Failing test.**

  ```python
  def test_ephemeral_fit_not_autologged(tmp_path: Path, executor: Executor) -> None:
      mlflow.set_tracking_uri(f"sqlite:///{tmp_path / 'mlflow.db'}")
      run_id = start_run_with_decisions(_sample_proposal())
      install_autolog_gate()
      mlflow.sklearn.autolog()

      executor.run(
          "from sklearn.linear_model import LogisticRegression\n"
          "import numpy as np\n"
          "X, y = np.random.randn(100, 4), np.random.randint(0, 2, 100)\n"
          "LogisticRegression().fit(X, y)\n",
          ephemeral=True,
      )
      metrics = mlflow.get_run(run_id).data.metrics
      assert metrics == {}  # nothing logged
  ```

- [ ] **2. Implement the gate.**

  ```python
  # sidecar/src/kiln_sidecar/autolog_gate.py
  from __future__ import annotations

  import contextvars
  from collections.abc import Callable
  from functools import wraps
  from typing import Final, TypeVar

  import mlflow

  is_ephemeral: Final[contextvars.ContextVar[bool]] = contextvars.ContextVar(
      "kiln_is_ephemeral", default=False
  )

  T = TypeVar("T")


  def install_autolog_gate() -> None:
      """Wrap MLflow autolog patches so they no-op while is_ephemeral=True."""
      from mlflow.utils.autologging_utils import safe_patch as _orig_safe_patch  # noqa: F401 (private)
      # Replace the function used by autolog to register patches with one that
      # threads a check on `is_ephemeral`. The implementation has to monkey-patch
      # this single hook — autolog calls it for every flavour. Comment in the
      # PR with the upstream code location.
      ...
  ```

  > Implementer's note: the real call site is `mlflow.utils.autologging_utils.safe_patch`. Wrap it once. If a future MLflow refactor moves the seam, fail loudly — do not silently drop the gate.

- [ ] **3. Set the context var in `Executor.run`.**

  ```python
  from kiln_sidecar.autolog_gate import is_ephemeral

  def run(self, code: str, *, ephemeral: bool = False) -> ExecuteResult:
      token = is_ephemeral.set(ephemeral)
      try:
          return self._run_inner(code, ephemeral=ephemeral)
      finally:
          is_ephemeral.reset(token)
  ```

  Note: this must propagate into the kernel process too. **The clean path is to push the flag through `execute_request`'s metadata** so the kernel-side sees it. Implementer must verify with the jupyter_client API surface. Document the chosen path in the file.

- [ ] **4. Run the test + lint + commit.**

  ```sh
  cd sidecar && uv run pytest tests/test_mlflow_runs.py -v
  git commit -m "feat(checkpoint): ephemeral cells bypass MLflow autolog"
  ```

## Acceptance

- The new ephemeral test passes.
- An identical non-ephemeral call **does** log metrics (regression guard).
- The gate is installed exactly once (idempotent).

## Out of scope

- Promotion (session → clean script) — out of MVP entirely. The seam created here is what makes it cheap to add later: the ephemeral cells are already separable.
