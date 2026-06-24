# Ticket 43 — Summary stats / distributions / null patterns

**Phase:** 5
**Depends on:** [42](./42-df-explorer-viewer.md)
**Blocks:** none in the MVP

## Goal

A header strip above the table showing per-column: dtype, null count, min / max / mean for numeric, top-K cardinality for categorical, distribution sparkline. This is the bar the spec sets: *"lets the human smell when a result is wrong (leakage, degenerate distribution, suspicious AUC)"* (§5.2).

## Files

- Modify: `sidecar` — add a Flight action `summary` returning per-column stats as Arrow.
- Modify: `src/lib/components/DataFrameView.svelte` — render a `<SummaryStrip />` row.
- Create: `src/lib/components/SummaryStrip.svelte`.

## Steps

- [ ] **1. Sidecar `summary(handle)` action.**

  ```python
  # sidecar/src/kiln_sidecar/df_summary.py
  from __future__ import annotations
  import pyarrow as pa
  import pyarrow.compute as pc

  def summarise(table: pa.Table) -> pa.Table:
      rows: list[dict[str, object]] = []
      for name in table.column_names:
          col = table.column(name)
          null_count = col.null_count
          row: dict[str, object] = {"column": name, "dtype": str(col.type), "nulls": null_count}
          if pa.types.is_integer(col.type) or pa.types.is_floating(col.type):
              row["min"] = pc.min(col).as_py()
              row["max"] = pc.max(col).as_py()
              row["mean"] = pc.mean(col).as_py()
          else:
              row["top"] = pc.mode(col).as_py()
          rows.append(row)
      return pa.Table.from_pylist(rows)
  ```

- [ ] **2. Viewer renders the strip.** Each column shows compact stats and a 32-bin sparkline (computed server-side as percentile counts).

- [ ] **3. Smoke test.**

  - Run `import seaborn as sns; df = sns.load_dataset('penguins'); df`. Confirm correct dtypes, sensible nulls.
  - Force leakage: `df['cheat'] = df['body_mass_g']`. The viewer's null/distribution callout shows perfect correlation when a future column-pair correlation feature lands (placeholder for now).

- [ ] **4. Lint + commit.**

  ```sh
  git commit -m "feat(df): summary strip — dtype, nulls, distribution sparkline"
  ```

## Acceptance

- Strip renders within 200 ms for a 1M-row × 20-col frame (the bar from §5.2).
- Sparklines do not flicker on scroll (they are derived from the full frame, not the visible page).
- No `any`. No floating promises.

## Out of scope

- Column-pair correlations — fast-follow.
- Auto-flagging suspicious distributions (e.g. perfect-correlation = leakage) — Ticket 62 places the hook.
- Configurable bin counts in the UI — out of MVP.
