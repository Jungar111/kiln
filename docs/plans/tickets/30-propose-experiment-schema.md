# Ticket 30 — `propose_experiment` tool schema

**Phase:** 4 (checkpoint object + premise gate)
**Depends on:** [22](./22-claude-client-stub.md)
**Blocks:** [31](./31-slot-validation.md), [32](./32-premise-gate-ui.md)

> **⚠️ Adapted for the Claude Code CLI.** Since Kiln drives Claude through the
> CLI (not the API), there is no API request to embed a tool schema into. The
> checkpoint is delivered like this instead: Kiln appends a system-prompt
> instruction telling Claude to emit the proposal as a single fenced
> ```` ```kiln-experiment ```` JSON block; `src-tauri/src/claude.rs` extracts and
> deserializes that block into the `ProposeExperiment` Rust type, validates it
> (Ticket 31), and fires a `checkpoint:proposed` Tauri event. The Rust type and
> the Python Pydantic mirror remain the schema's source of truth (cross-language
> contract test instead of a schemars golden snapshot). **`schemars` / the API
> tool-array embedding (Step 3) is dropped.** A real MCP `propose_experiment`
> tool server is the future upgrade that restores a hard tool boundary + a
> re-prompt loop; the prompt-parse path is the lazy MVP with a known ceiling.

## Goal

Define the **structured** input schema for the `propose_experiment` tool the model uses to propose an investigation. The schema is the spec's "checkpoint object" — the slots that turn "trust Claude to mention the train/test split" into "the split decision is a structural field that cannot be empty" (spec §4).

## What the schema must encode

From spec §4 the required slots (when in scope) are:

| Key                         | Description                                                              |
| --------------------------- | ------------------------------------------------------------------------ |
| `validation_strategy`       | Split / CV scheme, temporal vs random.                                   |
| `target_definition`         | What `y` is, how constructed, any lookahead in the label.                |
| `feature_provenance`        | Are any features computed from information unavailable at prediction time. |
| `preprocessing_fit_scope`   | Scalers / imputers / encoders fit before or after the split.             |
| `data_scope_and_exclusions` | Rows dropped, what population this is actually about.                    |
| `missing_data_handling`     | Drop vs impute, and whether imputation leaks.                            |
| `metric_choice`             | And whether it fits the problem.                                         |

Each slot may legitimately be `"N/A"`, but must be present. Each carries a **severity** (`critical | notable | fyi`) and the proposal carries a **`look_here`** array (spec §4 "Scannability devices").

## Files

- Create: `src-tauri/src/checkpoint.rs` — Rust types (the source of truth for the schema).
- Create: `sidecar/src/kiln_sidecar/checkpoint.py` — Python mirror (Pydantic) for any sidecar-side use.
- Modify: `src-tauri/src/claude.rs` — embed the JSON-Schema for the tool.
- Create: `src-tauri/tests/checkpoint_schema.rs` — golden-test the schema.

## Steps

- [ ] **1. Define the Rust types.**

  ```rust
  // src-tauri/src/checkpoint.rs
  use serde::{Deserialize, Serialize};

  #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
  #[serde(rename_all = "lowercase")]
  pub enum Severity { Critical, Notable, Fyi }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct Slot {
      pub in_scope: bool,
      pub severity: Severity,
      /// Free-text answer or "N/A". MUST be non-empty when `in_scope` is true.
      pub answer: String,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct ProposeExperiment {
      pub title: String,
      pub premise: String,
      pub validation_strategy: Slot,
      pub target_definition: Slot,
      pub feature_provenance: Slot,
      pub preprocessing_fit_scope: Slot,
      pub data_scope_and_exclusions: Slot,
      pub missing_data_handling: Slot,
      pub metric_choice: Slot,
      /// Where Claude is least sure / most wants the human's eye.
      pub look_here: Vec<String>,
  }
  ```

- [ ] **2. Mirror in Python (Pydantic).**

  ```python
  # sidecar/src/kiln_sidecar/checkpoint.py
  from __future__ import annotations

  from enum import StrEnum
  from typing import Final

  from pydantic import BaseModel, Field, model_validator


  class Severity(StrEnum):
      CRITICAL = "critical"
      NOTABLE = "notable"
      FYI = "fyi"


  class Slot(BaseModel):
      in_scope: bool
      severity: Severity
      answer: str = Field(min_length=1)

      @model_validator(mode="after")
      def _require_answer_when_in_scope(self) -> "Slot":
          if self.in_scope and not self.answer.strip():
              raise ValueError("answer must be non-empty when in_scope=True")
          return self


  REQUIRED_SLOTS: Final[tuple[str, ...]] = (
      "validation_strategy",
      "target_definition",
      "feature_provenance",
      "preprocessing_fit_scope",
      "data_scope_and_exclusions",
      "missing_data_handling",
      "metric_choice",
  )


  class ProposeExperiment(BaseModel):
      title: str = Field(min_length=1)
      premise: str = Field(min_length=1)
      validation_strategy: Slot
      target_definition: Slot
      feature_provenance: Slot
      preprocessing_fit_scope: Slot
      data_scope_and_exclusions: Slot
      missing_data_handling: Slot
      metric_choice: Slot
      look_here: list[str]
  ```

- [ ] **3. Embed the schema in the Claude tool definition.** Replace the stub `Tool` in `claude.rs` with `serde_json::to_value::<schemars::schema::RootSchema>(&schemars::schema_for!(ProposeExperiment))?`. Add `schemars = "0.8"` to `Cargo.toml` and `#[derive(schemars::JsonSchema)]` on the Rust types.

- [ ] **4. Golden test.** Snapshot the generated JSON-Schema under `src-tauri/tests/fixtures/propose_experiment.schema.json`. Test asserts current output matches the snapshot; deliberate schema changes update the snapshot in the same PR.

- [ ] **5. Lint + test + commit.**

  ```sh
  just lint && just test
  git commit -m "feat(checkpoint): propose_experiment tool schema (rust + py)"
  ```

## Acceptance

- The golden test passes.
- The same field set appears in both languages, in the same order.
- `model_validator` rejects an empty in-scope `answer`.
- No `Any` / no `# type: ignore`.

## Out of scope

- Enforcing the schema at the Claude side beyond what tool-use already gives — Ticket 31 deals with rejecting empties.
- UI for the slots — Ticket 32.
- MLflow tag writing — Ticket 33.
