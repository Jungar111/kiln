# Ticket 60 — MLflow query layer (`search_runs`)

**Phase:** 7 (run comparison)
**Depends on:** [33](./33-mlflow-tag-write.md), [34](./34-results-gate-ui.md)
**Blocks:** [61](./61-comparison-view.md), [62](./62-decision-diff.md)

## Goal

A typed wrapper around `mlflow.search_runs()` / `MlflowClient.search_runs` exposed via JSON-RPC. The webview gets a flat list of `Run` rows, each carrying params, metrics, and the `kiln.slot.*` tags (Ticket 33's output) — so the comparison view (Ticket 61) can render decisions as first-class diff rows.

> Spec §5.3 — *"Built on MLflow as engine, not surface — query via `mlflow.search_runs()` / `MlflowClient`, render an inline comparison view inside the app."*

## Files

- Create: `sidecar/src/kiln_sidecar/mlflow_query.py`.
- Modify: `sidecar/src/kiln_sidecar/__main__.py` — register `list_runs` method.
- Create: `sidecar/tests/test_mlflow_query.py`.

## Steps

- [ ] **1. Failing test.**

  ```python
  def test_list_runs_returns_kiln_tags(tmp_path: Path) -> None:
      mlflow.set_tracking_uri(f"sqlite:///{tmp_path / 'mlflow.db'}")
      run_id = start_run_with_decisions(_sample_proposal())
      mlflow.log_metric("accuracy", 0.91)
      mlflow.end_run()

      runs = list_runs(experiment_names=None, limit=10)
      assert len(runs) == 1
      assert runs[0].run_id == run_id
      assert runs[0].metrics["accuracy"] == pytest.approx(0.91)
      assert runs[0].decisions["validation_strategy"] == _sample_proposal().validation_strategy.answer
  ```

- [ ] **2. Implement.**

  ```python
  from __future__ import annotations

  from dataclasses import dataclass
  from typing import Final

  import mlflow
  from mlflow.tracking import MlflowClient

  KILN_PREFIX: Final[str] = "kiln.slot."


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
      experiment_ids: list[str]
      if experiment_names:
          experiment_ids = [exp.experiment_id for exp in (client.get_experiment_by_name(n) for n in experiment_names) if exp is not None]
      else:
          experiment_ids = [exp.experiment_id for exp in client.search_experiments()]

      runs = client.search_runs(
          experiment_ids=experiment_ids,
          max_results=limit,
          order_by=["start_time DESC"],
      )

      out: list[Run] = []
      for run in runs:
          tags = run.data.tags
          decisions: dict[str, str] = {
              k.removeprefix(KILN_PREFIX): v
              for k, v in tags.items()
              if k.startswith(KILN_PREFIX) and not k.endswith((".severity", ".in_scope"))
          }
          out.append(
              Run(
                  run_id=run.info.run_id,
                  name=tags.get("kiln.title", run.info.run_name or ""),
                  status=run.info.status,
                  outcome=tags.get("kiln.outcome"),
                  metrics=dict(run.data.metrics),
                  params=dict(run.data.params),
                  decisions=decisions,
                  look_here=[line for line in tags.get("kiln.look_here", "").split("\n") if line],
              )
          )
      return out
  ```

- [ ] **3. RPC.** Register `list_runs` in `__main__.py`:

  ```python
  def list_runs_method(params: dict[str, object]) -> list[dict[str, object]]:
      names_raw = params.get("experiment_names")
      names = names_raw if isinstance(names_raw, list) else None
      limit_raw = params.get("limit", 50)
      limit = limit_raw if isinstance(limit_raw, int) else 50
      runs = list_runs(experiment_names=names, limit=limit)
      return [asdict(r) for r in runs]
  ```

- [ ] **4. Tauri command.** A `list_runs(limit)` command forwards to the RPC.

- [ ] **5. Tests + lint + commit.**

  ```sh
  git commit -m "feat(mlflow): typed list_runs surfacing kiln.slot tags"
  ```

## Acceptance

- The test passes with both metrics and decisions populated.
- Tag-name parsing is correct (`.severity` / `.in_scope` suffixes excluded from `decisions`).
- No `Any`.

## Out of scope

- Server-side pagination of runs (cursors) — fast-follow when run counts get large.
- Time-range filters — fast-follow.
