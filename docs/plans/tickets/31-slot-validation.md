# Ticket 31 — Tool-boundary slot validation

**Phase:** 4
**Depends on:** [30](./30-propose-experiment-schema.md)
**Blocks:** [32](./32-premise-gate-ui.md), [33](./33-mlflow-tag-write.md)

## Goal

When Claude's `propose_experiment` tool call is **received**, validate it and reject with a structured error if a required in-scope slot is empty or missing. The rejection comes back as a new `tool_use_result` with `is_error: true` so Claude self-corrects rather than the human being asked to fix a broken proposal.

> Spec §9 — *"tool-boundary enforcement — reject/re-prompt on empty consequential slots"*

## Files

- Modify: `src-tauri/src/claude.rs` — handle `tool_use` blocks, validate, re-prompt loop.
- Modify: `src-tauri/src/checkpoint.rs` — add a `validate()` method that returns a list of `SlotError`s.
- Create: `src-tauri/tests/slot_validation.rs`.

## Steps

- [ ] **1. Failing test.**

  ```rust
  // src-tauri/tests/slot_validation.rs
  use kiln_lib::checkpoint::{ProposeExperiment, Severity, Slot};

  #[test]
  fn empty_in_scope_answer_is_rejected() {
      let mut p = sample_proposal();
      p.validation_strategy.answer = String::new();
      let errors = p.validate();
      assert_eq!(errors.len(), 1);
      assert_eq!(errors[0].slot, "validation_strategy");
  }

  #[test]
  fn out_of_scope_with_empty_answer_is_allowed() {
      let mut p = sample_proposal();
      p.validation_strategy.in_scope = false;
      p.validation_strategy.answer = "N/A".into();
      assert!(p.validate().is_empty());
  }

  fn sample_proposal() -> ProposeExperiment { /* … minimal valid proposal … */ }
  ```

- [ ] **2. Implement `validate`.**

  ```rust
  #[derive(Debug, PartialEq, Eq)]
  pub struct SlotError { pub slot: &'static str, pub reason: &'static str }

  impl ProposeExperiment {
      pub fn validate(&self) -> Vec<SlotError> {
          let mut errors = Vec::new();
          let pairs: [(&'static str, &Slot); 7] = [
              ("validation_strategy", &self.validation_strategy),
              ("target_definition", &self.target_definition),
              ("feature_provenance", &self.feature_provenance),
              ("preprocessing_fit_scope", &self.preprocessing_fit_scope),
              ("data_scope_and_exclusions", &self.data_scope_and_exclusions),
              ("missing_data_handling", &self.missing_data_handling),
              ("metric_choice", &self.metric_choice),
          ];
          for (name, slot) in pairs {
              if slot.in_scope && slot.answer.trim().is_empty() {
                  errors.push(SlotError { slot: name, reason: "empty in-scope answer" });
              }
          }
          errors
      }
  }
  ```

- [ ] **3. Wire the re-prompt loop in `claude::ClaudeClient::send`.** When a `ToolUse` block named `propose_experiment` lands:

  1. Parse `input` into `ProposeExperiment`.
  2. Call `validate()`.
  3. If empty → emit a `sidecar:checkpoint` Tauri event with the proposal and return the assistant message verbatim.
  4. If non-empty → send a follow-up `messages` request with a `tool_result` block of `{ "is_error": true, "content": "<errors as text>" }` and loop (cap at 3 attempts).

  Tests for the loop go in `src-tauri/tests/claude_loop.rs` — use a `wiremock`-style stub so no live API calls are needed in CI.

- [ ] **4. Lint + test + commit.**

  ```sh
  git commit -m "feat(checkpoint): reject empty in-scope slots at the tool boundary"
  ```

## Acceptance

- Unit tests pass.
- Claude-loop integration test passes against the mock server.
- Cap of 3 retries is enforced; on the fourth attempt the assistant message returns as-is with a `look_here` entry mentioning the failed slots.

## Out of scope

- UI rendering — Ticket 32.
- Persisting the proposal — Ticket 33.
- Auto-derive in-scope from static code analysis — out of MVP (spec §9 fast-follow).
