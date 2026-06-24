# Ticket 02 — Spawn and supervise an IPython kernel inside the sidecar

**Phase:** 1 (sidecar bootstrap)
**Depends on:** [01](./01-sidecar-deps.md)
**Blocks:** [03](./03-jsonrpc-control.md), [04](./04-execute-roundtrip.md)

## Goal

Implement a `Kernel` class that starts a `jupyter_client.KernelManager` against an `ipykernel` process, exposes `start()` / `shutdown()` / `is_alive()`, and runs cleanly under `pytest`.

## Spec / context

- One sidecar per project → one kernel per sidecar.
- The MVP cannot afford to retrofit the long-running-execution decision later (spec §10). **Decision:** kernel calls are blocking from a single dedicated `asyncio.to_thread` worker inside the sidecar. The control surface (Ticket 03) stays responsive because it dispatches separately. A background queue is **out of scope**; the chosen path lets the human cancel via `interrupt()` later.

## Files

- Create: `sidecar/src/kiln_sidecar/kernel.py`.
- Create: `sidecar/tests/test_kernel.py`.

## Steps

- [ ] **1. Failing test — start + alive + shutdown.**

  ```python
  # sidecar/tests/test_kernel.py
  from __future__ import annotations

  import pytest

  from kiln_sidecar.kernel import Kernel


  @pytest.fixture
  def kernel() -> Kernel:
      k = Kernel()
      k.start()
      try:
          yield k
      finally:
          k.shutdown()


  def test_kernel_starts_alive_and_stops(kernel: Kernel) -> None:
      assert kernel.is_alive() is True
      kernel.shutdown()
      assert kernel.is_alive() is False
  ```

- [ ] **2. Run — confirm `ModuleNotFoundError: kiln_sidecar.kernel`.**

  ```sh
  cd sidecar && uv run pytest tests/test_kernel.py -v
  ```

- [ ] **3. Minimal implementation.**

  ```python
  # sidecar/src/kiln_sidecar/kernel.py
  """Owns the lifecycle of a single IPython kernel.

  jupyter_client's KernelManager already covers spawning, ZMQ wiring,
  and process supervision. We wrap it so the rest of the codebase only
  ever sees our typed surface — the upstream API is broad and untyped.
  """

  from __future__ import annotations

  from typing import Final

  from jupyter_client.manager import KernelManager


  class Kernel:
      KERNEL_NAME: Final[str] = "python3"

      def __init__(self) -> None:
          self._manager: KernelManager | None = None

      def start(self) -> None:
          if self._manager is not None:
              raise RuntimeError("kernel is already started")
          manager = KernelManager(kernel_name=self.KERNEL_NAME)
          manager.start_kernel()
          self._manager = manager

      def is_alive(self) -> bool:
          return self._manager is not None and self._manager.is_alive()

      def shutdown(self) -> None:
          if self._manager is None:
              return
          self._manager.shutdown_kernel(now=True)
          self._manager = None
  ```

- [ ] **4. Run — verify green.**

  ```sh
  cd sidecar && uv run pytest tests/test_kernel.py -v
  ```

- [ ] **5. Lint.**

  ```sh
  just lint-py
  ```

  Expected: ruff + ty PASS. If `ty` complains about `KernelManager` stubs, add a focused, _typed_ wrapper (no `Any`, no `# type: ignore`). A `typing.cast()` with a one-line justification comment is acceptable; suppressions are not.

- [ ] **6. Commit.**

  ```sh
  git add sidecar/src/kiln_sidecar/kernel.py sidecar/tests/test_kernel.py
  git commit -m "feat(sidecar): wrap jupyter_client KernelManager with typed Kernel"
  ```

## Acceptance

- `pytest tests/test_kernel.py` is green.
- Subprocess is reliably cleaned up after the test (`psutil` would say so — leave manual check; CI catches strays).
- `just lint-py` is green.

## Out of scope

- Code execution / `execute_interactive` — Ticket 04.
- Interrupt / restart — fast-follow after MVP unless Ticket 04 forces it.
