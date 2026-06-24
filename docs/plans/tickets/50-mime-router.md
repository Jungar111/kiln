# Ticket 50 — Route kernel `display_data` MIME bundles

**Phase:** 6 (plot viewer)
**Depends on:** [04](./04-execute-roundtrip.md), [41](./41-df-handle-hook.md)
**Blocks:** [51](./51-plot-panel.md)

## Goal

Extend the `Executor` to surface every `display_data` / `execute_result` MIME bundle the kernel emits, not just the values it already captures. The new field on `ExecuteResult` is `displays: list[Display]`, where each `Display` carries one of `{image/png base64, text/html, application/vnd.kiln.df+json}`. The viewer (Ticket 51) picks the highest-fidelity rendering per display.

> Spec §5.2 — *"intercepts kernel `display_data` / `execute_result` MIME bundles (`image/png`, `text/html`) and routes to a plot panel."*

## Why a fan-out instead of one field per type

Matplotlib emits PNG **and** SVG **and** text. Plotly emits HTML **and** PNG. Picking once on the sidecar side commits the viewer; deferring picks until the frontend keeps the surface flexible.

## Files

- Modify: `sidecar/src/kiln_sidecar/execute.py` — collect displays.
- Modify: `sidecar/tests/test_execute.py` — add matplotlib + plotly tests.
- Modify: `sidecar/pyproject.toml` `[dependency-groups] dev` — add `matplotlib`, `plotly` (test-only).

## Steps

- [ ] **1. Failing test.**

  ```python
  def test_matplotlib_emits_png_display(executor: Executor) -> None:
      executor.run("import matplotlib; matplotlib.use('Agg')", ephemeral=True)
      result = executor.run("import matplotlib.pyplot as plt; plt.plot([1,2,3]); plt.gcf()")
      assert any(d.mime == "image/png" for d in result.displays)


  def test_plotly_emits_html_display(executor: Executor) -> None:
      result = executor.run(
          "import plotly.express as px\n"
          "fig = px.scatter(x=[1,2,3], y=[3,2,1])\n"
          "fig"
      )
      assert any(d.mime == "text/html" for d in result.displays)
  ```

- [ ] **2. Implement.**

  ```python
  @dataclass(frozen=True, slots=True)
  class Display:
      mime: str
      payload: str  # base64 for binary; plain text for html/json
      metadata: dict[str, object]

  # in Executor.run, extend the iopub hook:
  elif msg_type in ("display_data", "execute_result"):
      data = content.get("data", {})
      if isinstance(data, dict):
          for mime, payload in data.items():
              displays.append(Display(
                  mime=mime,
                  payload=payload if isinstance(payload, str) else "",
                  metadata=content.get("metadata", {}) if isinstance(content.get("metadata", {}), dict) else {},
              ))
  ```

  Add `displays: list[Display] = []` to the closure and pass it into `ExecuteResult`. Strip the DataFrame handle MIME from `displays` (it is already surfaced as `df`).

- [ ] **3. Tests + lint + commit.**

  ```sh
  cd sidecar && uv run pytest -v tests/test_execute.py
  just lint-py
  git commit -m "feat(executor): surface every display_data MIME bundle"
  ```

## Acceptance

- Both matplotlib and plotly tests pass.
- The DataFrame handle MIME is NOT in `displays` (it's already surfaced as `df`).
- The original `value` / `stdout` / `traceback` semantics are unchanged.

## Out of scope

- Rendering — Ticket 51.
- ipywidgets / Jupyter Widget Comm protocol — out of MVP.
- LaTeX (`text/latex`) — included by default (it'll appear in `displays`), but no special handling.
