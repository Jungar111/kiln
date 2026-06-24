# Ticket 01 — Add the sidecar's runtime dependencies

**Phase:** 1 (sidecar bootstrap)
**Depends on:** none (Phase 0 is complete)
**Blocks:** [02](./02-kernel-lifecycle.md)

## Goal

Pin the Python packages the sidecar needs and prove they import inside the `kiln-sidecar` venv. **Pinning + import check only — no kernel code yet.**

## Why this is its own ticket

Three large native dependencies land here (`pyzmq`, `pyarrow`, `mlflow`). Each can fail to wheel on macOS / Windows. Isolating the install + import smoke test from the kernel code makes the regression bisectable.

## Files

- Modify: `sidecar/pyproject.toml` — add to `[project] dependencies`.
- Create: `sidecar/tests/test_deps_present.py`.

## Steps

- [ ] **1. Write the failing test.**

  Create `sidecar/tests/test_deps_present.py`:

  ```python
  """The sidecar runtime imports must succeed.

  These are the headline native packages. If any of these fails to import on
  a clean install we want to know *before* anything else stops working.
  """

  from __future__ import annotations

  import importlib


  def test_runtime_imports() -> None:
      for name in (
          "ipykernel",
          "jupyter_client",
          "zmq",
          "mlflow",
          "pyarrow",
          "pyarrow.flight",
      ):
          importlib.import_module(name)
  ```

- [ ] **2. Run it to confirm failure.**

  ```sh
  cd sidecar && uv run pytest tests/test_deps_present.py -v
  ```

  Expected: collection error or per-import `ModuleNotFoundError`.

- [ ] **3. Add the runtime deps to `sidecar/pyproject.toml`.**

  Replace `dependencies = []` with:

  ```toml
  dependencies = [
      "ipykernel>=6.30",
      "jupyter_client>=8.6",
      "pyzmq>=26",
      "mlflow>=3.0,<4",
      "pyarrow>=18,<22",
      "pydantic>=2.9",
  ]
  ```

  (`pydantic` lands here because every later ticket models data with it.)

- [ ] **4. Sync and re-run the test.**

  ```sh
  cd sidecar && uv sync --group dev
  uv run pytest tests/test_deps_present.py -v
  ```

  Expected: PASS.

- [ ] **5. Re-run the type checker and ruff** — the new deps must not introduce stub-resolution errors:

  ```sh
  just lint-py
  ```

  Expected: PASS.

- [ ] **6. Commit.**

  ```sh
  git add sidecar/pyproject.toml sidecar/uv.lock sidecar/tests/test_deps_present.py
  git commit -m "feat(sidecar): pin ipykernel/mlflow/pyarrow/pyzmq/pydantic"
  ```

## Acceptance

- `just test-py` is green.
- `just lint-py` is green.
- `uv.lock` is committed.
- No new entries in `# noqa`/`# type: ignore` (CI hook will fail anyway).

## Out of scope

- Actually starting a kernel — Ticket 02.
- Arrow server — Ticket 40.
- MLflow autolog or tag writes — Tickets 33 / 60.
