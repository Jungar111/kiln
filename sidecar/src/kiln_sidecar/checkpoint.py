"""Pydantic mirror of the checkpoint schema.

The Rust type in `src-tauri/src/checkpoint.rs` is the other half of this
contract; both deserialize the same sample JSON in their tests. The sidecar uses
this model to validate a proposal before writing its declared decisions as
MLflow tags (Ticket 33).
"""

from __future__ import annotations

from enum import StrEnum
from typing import Final

from pydantic import BaseModel, model_validator


class Severity(StrEnum):
    CRITICAL = "critical"
    NOTABLE = "notable"
    FYI = "fyi"


class Slot(BaseModel):
    in_scope: bool
    severity: Severity
    # No `min_length` here: an out-of-scope slot may legitimately be empty/"N/A".
    # The in-scope non-empty rule (matching Rust `ProposeExperiment::validate`) is
    # enforced below, so Python accepts exactly what the Rust tool boundary does.
    answer: str

    @model_validator(mode="after")
    def _require_answer_when_in_scope(self) -> Slot:
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
    # Rust accepts any title/premise (the gate already rendered), so don't reject
    # here — Python must accept whatever passed the Rust tool boundary.
    title: str
    premise: str
    validation_strategy: Slot
    target_definition: Slot
    feature_provenance: Slot
    preprocessing_fit_scope: Slot
    data_scope_and_exclusions: Slot
    missing_data_handling: Slot
    metric_choice: Slot
    look_here: list[str]
