# Ticket 04 — `execute` method: code in, value out

**Phase:** 1 (sidecar bootstrap)
**Depends on:** [02](./02-kernel-lifecycle.md), [03](./03-jsonrpc-control.md)
**Blocks:** [05](./05-experiment-vs-inspection.md), [12](./12-tauri-execute-command.md)

## Goal

Wire the JSON-RPC dispatcher to the IPython kernel. A request `{"method":"execute","params":{"code":"1+1"}}` returns `{"status":"ok","stdout":"","value":"2"}`. Errors come back structured.

## Files

- Modify: `sidecar/src/kiln_sidecar/__main__.py` (register `execute`).
- Create: `sidecar/src/kiln_sidecar/execute.py`.
- Create: `sidecar/tests/test_execute.py`.

## Steps

- [ ] **1. Failing test.**

  ```python
  # sidecar/tests/test_execute.py
  from __future__ import annotations

  import pytest

  from kiln_sidecar.execute import ExecuteResult, Executor
  from kiln_sidecar.kernel import Kernel


  @pytest.fixture
  def executor() -> Executor:
      kernel = Kernel()
      kernel.start()
      ex = Executor(kernel)
      try:
          yield ex
      finally:
          kernel.shutdown()


  def test_expression_returns_value(executor: Executor) -> None:
      result = executor.run("1 + 1")
      assert result == ExecuteResult(status="ok", stdout="", value="2", traceback=None)


  def test_print_captures_stdout(executor: Executor) -> None:
      result = executor.run("print('hi')")
      assert result.stdout.strip() == "hi"
      assert result.value is None


  def test_exception_returns_error(executor: Executor) -> None:
      result = executor.run("raise ValueError('boom')")
      assert result.status == "error"
      assert "ValueError" in (result.traceback or "")
  ```

- [ ] **2. Run — failure.**

- [ ] **3. Implement `Executor` against `KernelClient.execute_interactive`.**

  ```python
  # sidecar/src/kiln_sidecar/execute.py
  """Sync wrapper around the IPython kernel's execute reply.

  We use execute_interactive because it folds the iopub stream messages
  (stdout/display) and the shell reply into a single callback, which fits
  the request/response shape of the JSON-RPC surface.
  """

  from __future__ import annotations

  import io
  from dataclasses import dataclass
  from typing import Final, Literal

  from kiln_sidecar.kernel import Kernel

  Status = Literal["ok", "error"]


  @dataclass(frozen=True, slots=True)
  class ExecuteResult:
      status: Status
      stdout: str
      value: str | None
      traceback: str | None


  class Executor:
      _TIMEOUT_SECONDS: Final[float] = 600.0

      def __init__(self, kernel: Kernel) -> None:
          self._kernel = kernel

      def run(self, code: str) -> ExecuteResult:
          manager = self._kernel.require_manager()
          client = manager.client()
          client.start_channels()
          try:
              stdout_buf = io.StringIO()
              value: str | None = None
              traceback: str | None = None

              def on_iopub(msg: dict[str, object]) -> None:
                  nonlocal value, traceback
                  header = msg.get("header", {})
                  if not isinstance(header, dict):
                      return
                  msg_type = header.get("msg_type")
                  content = msg.get("content", {})
                  if not isinstance(content, dict):
                      return
                  if msg_type == "stream":
                      text = content.get("text", "")
                      if isinstance(text, str):
                          stdout_buf.write(text)
                  elif msg_type == "execute_result":
                      data = content.get("data", {})
                      if isinstance(data, dict):
                          plain = data.get("text/plain")
                          if isinstance(plain, str):
                              value = plain
                  elif msg_type == "error":
                      tb = content.get("traceback", [])
                      if isinstance(tb, list):
                          traceback = "\n".join(str(line) for line in tb)

              reply = client.execute_interactive(
                  code,
                  output_hook=on_iopub,
                  timeout=self._TIMEOUT_SECONDS,
              )
              status = reply["content"].get("status", "error")
              return ExecuteResult(
                  status="ok" if status == "ok" and traceback is None else "error",
                  stdout=stdout_buf.getvalue(),
                  value=value,
                  traceback=traceback,
              )
          finally:
              client.stop_channels()
  ```

  Add the helper to `Kernel`:

  ```python
  # kernel.py — append to Kernel
  def require_manager(self) -> KernelManager:
      if self._manager is None:
          raise RuntimeError("kernel is not started")
      return self._manager
  ```

- [ ] **4. Register the dispatcher method.**

  ```python
  # __main__.py — replace the body of main()
  from kiln_sidecar.execute import Executor
  from kiln_sidecar.kernel import Kernel
  from kiln_sidecar.rpc import Dispatcher

  def main() -> int:
      kernel = Kernel()
      kernel.start()
      executor = Executor(kernel)
      dispatcher = Dispatcher()

      def execute(params: dict[str, object]) -> dict[str, object]:
          code = params.get("code")
          if not isinstance(code, str):
              raise ValueError("`code` must be a string")
          result = executor.run(code)
          return {
              "status": result.status,
              "stdout": result.stdout,
              "value": result.value,
              "traceback": result.traceback,
          }

      dispatcher.register("execute", execute)
      try:
          for line in sys.stdin:
              stripped = line.strip()
              if not stripped:
                  continue
              sys.stdout.write(dispatcher.handle(stripped))
              sys.stdout.write("\n")
              sys.stdout.flush()
      finally:
          kernel.shutdown()
      return 0
  ```

- [ ] **5. Run tests.**

  ```sh
  cd sidecar && uv run pytest -v
  ```

- [ ] **6. End-to-end smoke check.**

  ```sh
  ( echo '{"jsonrpc":"2.0","id":1,"method":"execute","params":{"code":"2+2"}}'; sleep 1 ) \
    | uv run --directory sidecar kiln-sidecar
  ```

  Expected output line: `{"jsonrpc":"2.0","id":1,"result":{"status":"ok","stdout":"","value":"4","traceback":null}}`.

- [ ] **7. Commit.**

  ```sh
  git commit -m "feat(sidecar): JSON-RPC execute method backed by IPython kernel"
  ```

## Acceptance

- All three `test_execute` cases green.
- Smoke check returns the expected line.
- No type ignores in any of the new code.

## Out of scope

- Async / cancel — fast-follow if Phase 8 requires it.
- DataFrame interception — Ticket 41.
- Plot interception — Ticket 50.
