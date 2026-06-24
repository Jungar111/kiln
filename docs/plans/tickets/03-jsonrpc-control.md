# Ticket 03 — Stdio JSON-RPC control surface

**Phase:** 1 (sidecar bootstrap)
**Depends on:** [02](./02-kernel-lifecycle.md)
**Blocks:** [04](./04-execute-roundtrip.md), [11](./11-control-client.md)

## Goal

Replace `__main__.py`'s stub with a stdio JSON-RPC v2 loop. Two methods are exposed in this ticket: `ping → "pong"` and `version → {kernel, sidecar}`. The transport is line-delimited JSON over stdin/stdout so the Rust side can drive it without any extra dependencies.

## Why JSON-RPC, why stdio

- **JSON-RPC**: a single tiny spec; no framework. Each request has `id`/`method`/`params`; each response has `id`/`result` or `id`/`error`. Bidirectional later if needed.
- **Stdio**: trivially supervised by Tokio in Rust; survives crashes cleanly; no port collisions with the Arrow server.

## Files

- Modify: `sidecar/src/kiln_sidecar/__main__.py`.
- Create: `sidecar/src/kiln_sidecar/rpc.py`.
- Create: `sidecar/tests/test_rpc.py`.

## Steps

- [ ] **1. Failing test.**

  ```python
  # sidecar/tests/test_rpc.py
  from __future__ import annotations

  import json

  from kiln_sidecar.rpc import Dispatcher


  def test_ping_returns_pong() -> None:
      dispatcher = Dispatcher()
      raw = dispatcher.handle(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "ping"}))
      assert json.loads(raw) == {"jsonrpc": "2.0", "id": 1, "result": "pong"}


  def test_unknown_method_returns_error() -> None:
      dispatcher = Dispatcher()
      raw = dispatcher.handle(
          json.dumps({"jsonrpc": "2.0", "id": 7, "method": "no.such"})
      )
      payload = json.loads(raw)
      assert payload["error"]["code"] == -32601  # method not found
      assert payload["id"] == 7
  ```

- [ ] **2. Run — failure.**

- [ ] **3. Implement `Dispatcher`.**

  ```python
  # sidecar/src/kiln_sidecar/rpc.py
  """Tiny JSON-RPC 2.0 dispatcher.

  We deliberately do not pull in a JSON-RPC library — the surface is small
  enough that we'd be paying dependency cost for almost no code. Conforms to
  https://www.jsonrpc.org/specification.
  """

  from __future__ import annotations

  import json
  from collections.abc import Callable
  from typing import Final, Literal

  JsonScalar = str | int | float | bool | None
  JsonValue = JsonScalar | list["JsonValue"] | dict[str, "JsonValue"]
  Method = Callable[[dict[str, JsonValue]], JsonValue]

  PARSE_ERROR: Final[int] = -32700
  INVALID_REQUEST: Final[int] = -32600
  METHOD_NOT_FOUND: Final[int] = -32601
  INTERNAL_ERROR: Final[int] = -32603


  class Dispatcher:
      def __init__(self) -> None:
          self._methods: dict[str, Method] = {
              "ping": _ping,
              "version": _version,
          }

      def register(self, name: str, fn: Method) -> None:
          self._methods[name] = fn

      def handle(self, raw: str) -> str:
          try:
              request: dict[str, JsonValue] = json.loads(raw)
          except json.JSONDecodeError:
              return _error(None, PARSE_ERROR, "parse error")

          request_id = request.get("id")
          method_name = request.get("method")
          if not isinstance(method_name, str):
              return _error(request_id, INVALID_REQUEST, "method must be a string")

          method = self._methods.get(method_name)
          if method is None:
              return _error(request_id, METHOD_NOT_FOUND, f"unknown method {method_name!r}")

          params_raw = request.get("params", {})
          params: dict[str, JsonValue] = params_raw if isinstance(params_raw, dict) else {}

          try:
              result = method(params)
          except Exception as exc:
              return _error(request_id, INTERNAL_ERROR, str(exc))
          return json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result})


  def _ping(_: dict[str, JsonValue]) -> Literal["pong"]:
      return "pong"


  def _version(_: dict[str, JsonValue]) -> dict[str, str]:
      from kiln_sidecar import __version__

      return {"sidecar": __version__, "kernel": "ipython"}


  def _error(request_id: JsonValue, code: int, message: str) -> str:
      return json.dumps(
          {"jsonrpc": "2.0", "id": request_id, "error": {"code": code, "message": message}}
      )
  ```

- [ ] **4. Rewrite `__main__.py` to drive the dispatcher off stdio.**

  ```python
  # sidecar/src/kiln_sidecar/__main__.py
  from __future__ import annotations

  import sys

  from kiln_sidecar.rpc import Dispatcher


  def main() -> int:
      dispatcher = Dispatcher()
      for line in sys.stdin:
          stripped = line.strip()
          if not stripped:
              continue
          sys.stdout.write(dispatcher.handle(stripped))
          sys.stdout.write("\n")
          sys.stdout.flush()
      return 0


  if __name__ == "__main__":
      raise SystemExit(main())
  ```

- [ ] **5. Smoke test the CLI.**

  ```sh
  echo '{"jsonrpc":"2.0","id":1,"method":"ping"}' | uv run --directory sidecar kiln-sidecar
  ```

  Expected: `{"jsonrpc":"2.0","id":1,"result":"pong"}`.

- [ ] **6. Lint + test + commit.**

  ```sh
  just lint-py && just test-py
  git add sidecar/src/kiln_sidecar/{rpc,__main__}.py sidecar/tests/test_rpc.py
  git commit -m "feat(sidecar): JSON-RPC over stdio with ping/version"
  ```

## Acceptance

- Pytest green.
- Stdin → stdout round-trip works against the installed `kiln-sidecar` script.
- No external JSON-RPC library added.

## Out of scope

- `execute` method — Ticket 04.
- Concurrency / threadsafety of the dispatcher — Ticket 04 introduces it when the kernel call blocks.
