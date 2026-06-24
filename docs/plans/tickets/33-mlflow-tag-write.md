# Ticket 33 — Persist declared decisions as MLflow run tags

**Phase:** 4
**Depends on:** [32](./32-premise-gate-ui.md)
**Blocks:** [34](./34-results-gate-ui.md), [60](./60-mlflow-query.md)

## Goal

When the human clicks **Approve**, the harness opens an MLflow run, writes the seven slots as run **tags** (one tag per slot, prefixed `kiln.slot.`), turns on `mlflow.<flavor>.autolog()`, and announces the run id back to the chat. *Approval binds to declared decisions, not lines of code* (spec §4). Storing them as tags makes "the thing approved" identical to "the thing compared" later (Ticket 60).

## Files

- Modify: `sidecar/src/kiln_sidecar/__main__.py` — register a new RPC method `approve_checkpoint`.
- Create: `sidecar/src/kiln_sidecar/mlflow_runs.py`.
- Modify: `src-tauri/src/commands.rs` — add `approve_checkpoint` Tauri command, forwards to RPC.
- Modify: `src/lib/components/PremiseGate.svelte` — Approve now `invoke('approve_checkpoint', { proposal })`.

## Steps

- [ ] **1. Failing test.**

  ```python
  # sidecar/tests/test_mlflow_runs.py
  from __future__ import annotations

  from pathlib import Path

  import mlflow

  from kiln_sidecar.checkpoint import ProposeExperiment, Slot, Severity
  from kiln_sidecar.mlflow_runs import start_run_with_decisions


  def test_decisions_land_as_tags(tmp_path: Path) -> None:
      mlflow.set_tracking_uri(f"sqlite:///{tmp_path / 'mlflow.db'}")
      proposal = _sample_proposal()
      run_id = start_run_with_decisions(proposal)
      tags = mlflow.get_run(run_id).data.tags
      assert tags["kiln.slot.validation_strategy"] == proposal.validation_strategy.answer
      assert tags["kiln.title"] == proposal.title
      mlflow.end_run()
  ```

- [ ] **2. Implement `start_run_with_decisions`.**

  ```python
  from __future__ import annotations

  import mlflow

  from kiln_sidecar.checkpoint import REQUIRED_SLOTS, ProposeExperiment


  def start_run_with_decisions(proposal: ProposeExperiment) -> str:
      run = mlflow.start_run(run_name=proposal.title)
      run_id: str = run.info.run_id
      mlflow.set_tag("kiln.title", proposal.title)
      mlflow.set_tag("kiln.premise", proposal.premise)
      for slot_name in REQUIRED_SLOTS:
          slot = getattr(proposal, slot_name)
          mlflow.set_tag(f"kiln.slot.{slot_name}", slot.answer)
          mlflow.set_tag(f"kiln.slot.{slot_name}.severity", slot.severity.value)
          mlflow.set_tag(f"kiln.slot.{slot_name}.in_scope", str(slot.in_scope))
      mlflow.set_tag("kiln.look_here", "\n".join(proposal.look_here))
      return run_id
  ```

- [ ] **3. RPC method.** In `__main__.py`:

  ```python
  def approve_checkpoint(params: dict[str, object]) -> dict[str, object]:
      proposal_raw = params.get("proposal")
      if not isinstance(proposal_raw, dict):
          raise ValueError("proposal must be an object")
      proposal = ProposeExperiment.model_validate(proposal_raw)
      run_id = start_run_with_decisions(proposal)
      mlflow.sklearn.autolog()  # opportunistically; safe to call multiple times
      return {"run_id": run_id}
  ```

- [ ] **4. Tauri command + frontend wiring.** Approve button:

  ```ts
  await invoke<{ run_id: string }>('approve_checkpoint', { proposal });
  ckpt.clear();
  // append assistant message: "Run started: <run_id>"
  ```

- [ ] **5. Tracking URI bootstrap.** On sidecar start, point MLflow at `mlruns.db` under the repo root:

  ```python
  mlflow.set_tracking_uri(f"sqlite:///{Path.cwd() / 'mlruns.db'}")
  ```

- [ ] **6. Lint + tests + commit.**

  ```sh
  git commit -m "feat(checkpoint): approve writes declared decisions as MLflow tags"
  ```

## Acceptance

- The test passes — tags land in MLflow's sqlite backend.
- Approving twice for the same proposal opens **two** runs (we do not dedup; that's a UX call for the human).
- The `mlruns.db` file is gitignored already (verify).

## Out of scope

- Closing the run automatically — fast-follow. For now, the run stays "RUNNING" until the next results-gate.
- Other autolog flavours (lightgbm, pytorch) — call `mlflow.autolog()` once we know which we need.
