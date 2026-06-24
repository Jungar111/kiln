# Ticket 05 — Experiment vs inspection: ephemeral execute flag

**Phase:** 1 (sidecar bootstrap)
**Depends on:** [04](./04-execute-roundtrip.md)
**Blocks:** [71](./71-experiment-vs-inspection-enforcement.md)

## Goal

Lay the contract that distinguishes Claude's logged kernel actions from the human's REPL pokes. Add an `ephemeral: bool` parameter to `execute` and **tag the kernel command itself**, so later tickets can decide whether to suppress MLflow autolog and avoid recording the cell in any promotion path.

> Spec §7 — *"Inspection is ephemeral; the experiment is the record. Draw this from day one — cheap now, miserable to retrofit."*

## Why now, not later

The implementation is two lines in this ticket. Retrofitting after MLflow autolog is wired (Ticket 33) means either rewriting MLflow run boundaries or accepting human pokes in the logged record. Land the seam now.

## Files

- Modify: `sidecar/src/kiln_sidecar/execute.py`.
- Modify: `sidecar/src/kiln_sidecar/__main__.py`.
- Modify: `sidecar/tests/test_execute.py`.

## Steps

- [ ] **1. Failing test.**

  Append to `test_execute.py`:

  ```python
  def test_ephemeral_flag_threads_through(executor: Executor) -> None:
      result = executor.run("x = 1", ephemeral=True)
      assert result.ephemeral is True

      result = executor.run("y = 2")
      assert result.ephemeral is False
  ```

- [ ] **2. Run — fails (`ephemeral` argument unknown).**

- [ ] **3. Thread the flag.**

  In `execute.py`:

  ```python
  @dataclass(frozen=True, slots=True)
  class ExecuteResult:
      status: Status
      stdout: str
      value: str | None
      traceback: str | None
      ephemeral: bool

  class Executor:
      def run(self, code: str, *, ephemeral: bool = False) -> ExecuteResult:
          # ...existing body...
          return ExecuteResult(
              status="ok" if status == "ok" and traceback is None else "error",
              stdout=stdout_buf.getvalue(),
              value=value,
              traceback=traceback,
              ephemeral=ephemeral,
          )
  ```

- [ ] **4. Expose it on the JSON-RPC method.**

  In `__main__.py`, update the inline `execute` callback:

  ```python
  def execute(params: dict[str, object]) -> dict[str, object]:
      code = params.get("code")
      if not isinstance(code, str):
          raise ValueError("`code` must be a string")
      ephemeral_raw = params.get("ephemeral", False)
      ephemeral = bool(ephemeral_raw) if isinstance(ephemeral_raw, bool) else False
      result = executor.run(code, ephemeral=ephemeral)
      return {
          "status": result.status,
          "stdout": result.stdout,
          "value": result.value,
          "traceback": result.traceback,
          "ephemeral": result.ephemeral,
      }
  ```

- [ ] **5. Re-run pytest + lint.**

- [ ] **6. Commit.**

  ```sh
  git commit -m "feat(sidecar): mark execute calls ephemeral=true for human REPL pokes"
  ```

## Acceptance

- `test_ephemeral_flag_threads_through` green.
- All earlier tests still green.
- The flag is purely informational at this point (no behaviour change in MLflow yet). Ticket 71 turns it into enforcement.

## Out of scope

- Actually suppressing MLflow autolog for ephemeral cells — Ticket 71. Wire the *seam* here; enforce it there.
- Promotion / session-to-script export — out of MVP entirely.
