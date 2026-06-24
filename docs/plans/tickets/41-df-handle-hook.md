# Ticket 41 — Custom kernel display hook: DataFrames become handles

**Phase:** 5
**Depends on:** [40](./40-arrow-server.md)
**Blocks:** [42](./42-df-explorer-viewer.md)

## Goal

Install an IPython display formatter so that when a cell evaluates to a `pandas.DataFrame` (or `polars.DataFrame`), the kernel emits a small JSON handle (`{"kiln/handle":"<id>", "rows": N, "cols": M, "schema": [...]}`) instead of the giant default HTML repr. The `Executor`'s iopub hook recognises the handle and surfaces it as a top-level `df` field on `ExecuteResult`.

> Spec §6 — *"DataFrames are never rendered via the kernel's default HTML repr — a custom display hook registers the frame in a side store and emits a lightweight handle; the viewer pages it over the local Arrow socket."*

## Files

- Modify: `sidecar/src/kiln_sidecar/__main__.py` — install the display hook into the kernel.
- Create: `sidecar/src/kiln_sidecar/df_display.py`.
- Modify: `sidecar/src/kiln_sidecar/execute.py` — recognise the handle MIME, expose `df` field.
- Modify: `sidecar/tests/test_execute.py` — add a DataFrame round-trip test (uses pandas).

## Steps

- [ ] **1. Failing test.**

  ```python
  def test_dataframe_becomes_a_handle(executor: Executor) -> None:
      executor.run("import pandas as pd; df = pd.DataFrame({'a':[1,2,3]}); df")
      result = executor.last_result
      assert result.df is not None
      assert result.df.rows == 3
      assert result.df.handle  # opaque id
  ```

- [ ] **2. Implement `df_display.py`.**

  ```python
  # sidecar/src/kiln_sidecar/df_display.py
  """Registers a DataFrame -> Arrow handle formatter in the running kernel."""

  from __future__ import annotations

  from typing import TYPE_CHECKING, Final

  import pyarrow as pa

  if TYPE_CHECKING:
      from kiln_sidecar.arrow_server import FrameRegistry

  MIME: Final[str] = "application/vnd.kiln.df+json"


  def install(registry: "FrameRegistry") -> None:
      from IPython import get_ipython  # imported lazily — only valid inside the kernel

      ip = get_ipython()
      if ip is None:
          raise RuntimeError("display hook must be installed inside a running kernel")
      formatter = ip.display_formatter.formatters.setdefault(MIME, _new_formatter())

      def format_pandas(obj: object) -> str:
          import pandas as pd

          if not isinstance(obj, pd.DataFrame):
              return ""
          handle = registry.register(pa.Table.from_pandas(obj))
          return _payload(handle.id, obj.shape[0], obj.shape[1], list(obj.columns))

      formatter.for_type("pandas.core.frame.DataFrame", format_pandas)

      try:
          import polars as pl

          def format_polars(obj: object) -> str:
              if not isinstance(obj, pl.DataFrame):
                  return ""
              handle = registry.register(obj.to_arrow())
              return _payload(handle.id, obj.height, obj.width, obj.columns)

          formatter.for_type("polars.dataframe.frame.DataFrame", format_polars)
      except ImportError:
          pass


  def _payload(handle: str, rows: int, cols: int, columns: list[str]) -> str:
      import json
      return json.dumps({"kiln/handle": handle, "rows": rows, "cols": cols, "schema": columns})


  def _new_formatter() -> object:
      from IPython.core.formatters import BaseFormatter
      return BaseFormatter()
  ```

- [ ] **3. Install the hook at sidecar boot.** After `Kernel.start()`:

  ```python
  executor.run(
      "import kiln_sidecar.df_display as _dfd\n"
      "_dfd.install(__kiln_registry__)",  # __kiln_registry__ pushed via execute below
      ephemeral=True,
  )
  ```

  Pushing `__kiln_registry__`: use `manager.client().push({...})` — or simpler, register a single Tcp socket and have the formatter look it up by env-var. Keep the choice in this ticket; document it in the file.

- [ ] **4. Surface `df` on `ExecuteResult`.** Extend `ExecuteResult` with `df: DfHandle | None`. Parse the new MIME bundle in `Executor.run`.

- [ ] **5. Run pytest + lint + commit.**

  ```sh
  git commit -m "feat(df): kernel display hook emits Arrow handles instead of HTML"
  ```

## Acceptance

- A pandas DataFrame round-trips into a non-empty handle.
- The default HTML repr is **not** in the iopub messages (assert it's absent in the test).
- `polars.DataFrame` is also handled when polars is installed (skip the polars case if not).

## Out of scope

- The viewer — Ticket 42.
- Numpy arrays / scipy sparse — out of MVP.
- Memory budget on the registry — out of MVP; a stale-frame eviction can land later.
