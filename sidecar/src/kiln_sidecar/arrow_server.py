"""Local-only HTTP server that streams DataFrame pages as Arrow IPC.

The webview `fetch()`es ``/page`` and ``/summary`` and decodes the Arrow IPC
bytes with ``apache-arrow``; the bytes never travel through the Rust IPC control
plane (spec's hard rule). We serve Arrow over plain localhost HTTP rather than
Flight/gRPC because a Tauri webview cannot speak gRPC without a much heavier
client.

The server lives inside the *kernel* process (see ``df_display``): that is where
the DataFrames are registered, so it can serve their bytes with no extra copy.
"""

from __future__ import annotations

import json
import secrets
import threading
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import TYPE_CHECKING

from kiln_sidecar.df_pages import page_table, summarise, to_ipc

if TYPE_CHECKING:
    import pyarrow as pa


@dataclass(frozen=True, slots=True)
class FrameHandle:
    """Opaque, unguessable id for a registered table."""

    id: str


class FrameRegistry:
    """Thread-safe side store mapping handles to Arrow tables.

    The display hook (ticket 41) registers frames here; the HTTP server reads
    them back. Eviction is out of MVP scope — frames live for the process.
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


def _make_handler(registry: FrameRegistry) -> type[BaseHTTPRequestHandler]:
    """Build a request handler closed over `registry` (no global state)."""

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, format: str, *args: object) -> None:
            # Silence the default stderr access log; the sidecar drains stderr.
            # `format` matches the base signature (BaseHTTPRequestHandler).
            del format, args

        def do_POST(self) -> None:
            length = int(self.headers.get("content-length", "0") or "0")
            raw = self.rfile.read(length)
            try:
                loaded: object = json.loads(raw)
            except json.JSONDecodeError:
                self._fail(400, "invalid JSON body")
                return
            if not isinstance(loaded, dict):
                self._fail(400, "body must be a JSON object")
                return
            handle = loaded.get("handle")
            if not isinstance(handle, str):
                self._fail(400, "missing handle")
                return
            table = registry.get(handle)
            if table is None:
                self._fail(404, f"unknown handle: {handle}")
                return

            if self.path.startswith("/page"):
                offset_raw = loaded.get("offset", 0)
                limit_raw = loaded.get("limit", 1000)
                sort_by_raw = loaded.get("sortBy")
                sort_dir_raw = loaded.get("sortDir", "asc")
                offset = offset_raw if isinstance(offset_raw, int) else 0
                limit = limit_raw if isinstance(limit_raw, int) else 1000
                sort_by = sort_by_raw if isinstance(sort_by_raw, str) else None
                sort_dir = sort_dir_raw if isinstance(sort_dir_raw, str) else "asc"
                self._ok(to_ipc(page_table(table, offset, limit, sort_by, sort_dir)))
            elif self.path.startswith("/summary"):
                self._ok(to_ipc(summarise(table)))
            else:
                self._fail(404, "not found")

        def _ok(self, payload: bytes) -> None:
            self.send_response(200)
            self.send_header("content-type", "application/vnd.apache.arrow.stream")
            self.send_header("content-length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def _fail(self, code: int, message: str) -> None:
            body = message.encode("utf-8")
            self.send_response(code)
            self.send_header("content-type", "text/plain")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    return Handler


class ArrowServer:
    """Owns the lifecycle of the local HTTP server on a background thread."""

    def __init__(self, registry: FrameRegistry, host: str, port: int) -> None:
        self.registry = registry
        self._http = ThreadingHTTPServer((host, port), _make_handler(registry))
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        thread = threading.Thread(target=self._http.serve_forever, daemon=True)
        thread.start()
        self._thread = thread

    @property
    def port(self) -> int:
        return self._http.server_address[1]

    def shutdown(self) -> None:
        self._http.shutdown()
        self._http.server_close()
        if self._thread is not None:
            self._thread.join(timeout=5.0)
            self._thread = None
