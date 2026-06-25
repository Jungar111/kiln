"""Local-only Arrow Flight server.

The webview opens a gRPC client to this port and streams DataFrame
pages directly — bytes never travel through the Rust IPC control plane.

`pyarrow.flight` ships without complete py.typed stubs, so the few upstream
call sites are pinned behind narrow Protocols and a single ``cast`` at
construction time (the same pattern ``kernel.py`` uses for ``jupyter_client``).
"""

from __future__ import annotations

import secrets
import threading
from dataclasses import dataclass
from typing import Protocol, cast

import pyarrow as pa
import pyarrow.flight as fl


@dataclass(frozen=True, slots=True)
class FrameHandle:
    """Opaque, unguessable id for a registered table."""

    id: str

    def encode(self) -> bytes:
        return self.id.encode("utf-8")


class FrameRegistry:
    """Thread-safe side store mapping handles to Arrow tables.

    The display hook (ticket 41) registers frames here; the Flight server and
    the HTTP page server (ticket 42) read them back out. Eviction is out of MVP
    scope — frames live for the process lifetime.
    """

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


class _FlightServerProtocol(Protocol):
    """The subset of FlightServerBase we drive after construction."""

    @property
    def port(self) -> int: ...

    def serve(self) -> None: ...

    def shutdown(self) -> None: ...

    def wait(self) -> None: ...


class _FlightServer(fl.FlightServerBase):
    def __init__(self, location: str, registry: FrameRegistry) -> None:
        super().__init__(location)
        self._registry = registry

    def do_get(
        self,
        context: fl.ServerCallContext,
        ticket: fl.Ticket,
    ) -> fl.RecordBatchStream:
        handle = ticket.ticket.decode("utf-8")
        table = self._registry.get(handle)
        if table is None:
            raise fl.FlightServerError(f"unknown handle: {handle}")
        return fl.RecordBatchStream(table)


class ArrowServer:
    """Owns the lifecycle of the local Flight server on a background thread."""

    def __init__(self, registry: FrameRegistry, host: str, port: int) -> None:
        self.registry = registry
        self._location = f"grpc+tcp://{host}:{port}"
        self._server: _FlightServerProtocol | None = None
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        # cast: FlightServerBase exposes port/serve/shutdown/wait at runtime; the
        # missing attributes are an upstream stubs gap, not a real API gap.
        server = cast("_FlightServerProtocol", _FlightServer(self._location, self.registry))
        thread = threading.Thread(target=server.serve, daemon=True)
        thread.start()
        self._server = server
        self._thread = thread

    @property
    def port(self) -> int:
        if self._server is None:
            raise RuntimeError("arrow server is not started")
        return self._server.port

    def shutdown(self) -> None:
        if self._server is not None:
            self._server.shutdown()
            self._server.wait()
            self._server = None
        if self._thread is not None:
            self._thread.join(timeout=5.0)
            self._thread = None
