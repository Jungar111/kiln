# Ticket 40 — Arrow IPC server inside the sidecar

**Phase:** 5 (DataFrame explorer)
**Depends on:** [04](./04-execute-roundtrip.md)
**Blocks:** [41](./41-df-handle-hook.md), [42](./42-df-explorer-viewer.md)

## Goal

Start an Arrow IPC server inside the sidecar process that serves pages of a registered DataFrame over a **localhost socket**, with **zero copies** of the bytes through Rust. The webview will connect to this socket directly (Ticket 42). This ticket implements the server; later tickets put real DataFrames in front of it.

> Spec §6 — *"Big bytes (DataFrames) take the direct Python ↔ webview path. Rust IPC is control-plane only."*

## Why Arrow Flight (and why local-only)

- **Arrow Flight** is the standard way to stream Arrow record batches over gRPC. We pin it to `localhost` and a random port; no auth, no TLS — the process is owned by the user.
- The alternative — Arrow IPC over a raw socket — saves one dependency but loses cursoring / per-call filtering. Flight gives us `do_get`, `do_action`, and ticketed streams for free.
- Page size defaults to 50_000 rows; the viewer can override per request.

## Files

- Create: `sidecar/src/kiln_sidecar/arrow_server.py`.
- Create: `sidecar/tests/test_arrow_server.py`.
- Modify: `sidecar/src/kiln_sidecar/__main__.py` — start the server, expose `arrow_port` via a new `arrow_port` RPC method.

## Steps

- [ ] **1. Failing test.**

  ```python
  # sidecar/tests/test_arrow_server.py
  from __future__ import annotations

  import pyarrow as pa
  import pyarrow.flight as fl
  import pytest

  from kiln_sidecar.arrow_server import ArrowServer, FrameRegistry


  @pytest.fixture
  def server() -> ArrowServer:
      registry = FrameRegistry()
      srv = ArrowServer(registry, host="127.0.0.1", port=0)
      srv.start()
      yield srv
      srv.shutdown()


  def test_registered_frame_is_streamed(server: ArrowServer) -> None:
      handle = server.registry.register(pa.table({"x": [1, 2, 3], "y": ["a", "b", "c"]}))
      client = fl.connect(("grpc+tcp", "127.0.0.1", server.port))
      reader = client.do_get(fl.Ticket(handle.encode()))
      table = reader.read_all()
      assert table.num_rows == 3
      assert table.column_names == ["x", "y"]
  ```

- [ ] **2. Implement `FrameRegistry` and `ArrowServer`.**

  ```python
  # sidecar/src/kiln_sidecar/arrow_server.py
  """Local-only Arrow Flight server.

  The webview opens a gRPC client to this port and streams DataFrame
  pages directly — bytes never travel through Rust IPC.
  """

  from __future__ import annotations

  import secrets
  import threading
  from collections.abc import Iterator
  from dataclasses import dataclass

  import pyarrow as pa
  import pyarrow.flight as fl


  @dataclass(frozen=True, slots=True)
  class FrameHandle:
      id: str

      def encode(self) -> bytes:
          return self.id.encode("utf-8")


  class FrameRegistry:
      def __init__(self) -> None:
          self._frames: dict[str, pa.Table] = {}
          self._lock = threading.Lock()

      def register(self, table: pa.Table) -> FrameHandle:
          handle = secrets.token_hex(8)
          with self._lock:
              self._frames[handle] = table
          return FrameHandle(handle)

      def get(self, handle: str) -> pa.Table | None:
          with self._lock:
              return self._frames.get(handle)


  class _FlightServer(fl.FlightServerBase):
      def __init__(self, location: str, registry: FrameRegistry) -> None:
          super().__init__(location)
          self._registry = registry

      def do_get(
          self,
          context: fl.ServerCallContext,
          ticket: fl.Ticket,
      ) -> fl.FlightDataStream:
          handle = ticket.ticket.decode("utf-8")
          table = self._registry.get(handle)
          if table is None:
              raise fl.FlightUnavailableError(f"unknown handle: {handle}")
          return fl.RecordBatchStream(table)


  class ArrowServer:
      def __init__(self, registry: FrameRegistry, host: str, port: int) -> None:
          self.registry = registry
          self._location = f"grpc+tcp://{host}:{port}"
          self._server: _FlightServer | None = None
          self._thread: threading.Thread | None = None

      def start(self) -> None:
          self._server = _FlightServer(self._location, self.registry)
          self._thread = threading.Thread(target=self._server.serve, daemon=True)
          self._thread.start()

      @property
      def port(self) -> int:
          assert self._server is not None
          return self._server.port

      def shutdown(self) -> None:
          if self._server is not None:
              self._server.shutdown()
              self._server = None
  ```

- [ ] **3. Plumb the port to RPC.** In `__main__.py`:

  ```python
  registry = FrameRegistry()
  arrow_server = ArrowServer(registry, host="127.0.0.1", port=0)
  arrow_server.start()
  dispatcher.register("arrow_port", lambda _: {"port": arrow_server.port})
  ```

- [ ] **4. Run pytest + lint + commit.**

  ```sh
  just lint-py && just test-py
  git commit -m "feat(sidecar): Arrow Flight server for DataFrame pages"
  ```

## Acceptance

- `do_get` round-trips a registered table.
- The server runs on a random port; the port is queryable via the new `arrow_port` RPC method.
- Shutdown is clean (no thread leak across tests).

## Out of scope

- Pagination / filter / sort — Ticket 42 layers them on with Flight `do_action`.
- Custom display hook in the kernel — Ticket 41.
- Cross-process zero-copy (memory mapping) — out of MVP; we accept the gRPC serialise/deserialise cost on localhost.
