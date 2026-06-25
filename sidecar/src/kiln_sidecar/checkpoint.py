"""Pydantic mirror of the checkpoint schema.

The Rust type in `src-tauri/src/checkpoint.rs` is the other half of this
contract; both deserialize the same sample JSON in their tests. The sidecar uses
this model to validate a proposal before writing its declared decisions as
MLflow tags (Ticket 33).
"""

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
