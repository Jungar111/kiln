"""Sync wrapper around the IPython kernel's execute reply.

We use execute_interactive because it folds the iopub stream messages
(stdout/display) and the shell reply into a single callback, which fits
the request/response shape of the JSON-RPC surface.
"""

from __future__ import annotations

import io
from dataclasses import dataclass
from typing import TYPE_CHECKING, Final, Literal

if TYPE_CHECKING:
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
            # wait_for_ready blocks until the kernel's ZMQ channels are fully
            # initialised and the kernel_info_reply handshake is complete.
            # Without this, execute_interactive races with channel setup and times out.
            # Kept inside the try so a timeout still runs stop_channels() in finally.
            client.wait_for_ready(timeout=60.0)
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
            # execute_interactive returns Mapping[str, object]; we must narrow
            # before accessing nested keys to satisfy ty's type checker.
            content = reply.get("content")
            if isinstance(content, dict):
                status_raw = content.get("status", "error")
                status_ok = isinstance(status_raw, str) and status_raw == "ok"
            else:
                status_ok = False

            return ExecuteResult(
                status="ok" if status_ok and traceback is None else "error",
                stdout=stdout_buf.getvalue(),
                value=value,
                traceback=traceback,
            )
        finally:
            client.stop_channels()
