"""Sync wrapper around the IPython kernel's execute reply.

We use execute_interactive because it folds the iopub stream messages
(stdout/display) and the shell reply into a single callback, which fits
the request/response shape of the JSON-RPC surface.
"""

from __future__ import annotations

import io
import json
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Final, Literal

if TYPE_CHECKING:
    from kiln_sidecar.kernel import Kernel

Status = Literal["ok", "error"]

# Custom MIME the df_display hook emits instead of the giant default HTML repr.
# Kept in sync with kiln_sidecar.df_display.MIME.
DF_MIME: Final[str] = "application/vnd.kiln.df+json"


@dataclass(frozen=True, slots=True)
class Display:
    """One MIME rendering of a `display_data` / `execute_result` bundle.

    The kernel may emit several MIME types for the same output (matplotlib emits
    PNG + SVG + text; plotly emits HTML + PNG). We fan every one out and let the
    viewer (Ticket 51) pick the highest-fidelity rendering it can render.

    `payload` is the raw MIME value as the kernel serialised it: base64 for
    binary types (e.g. `image/png`), plain text for `text/html` / `text/plain`.
    """

    mime: str
    payload: str
    metadata: dict[str, object]


@dataclass(frozen=True, slots=True)
class DfHandle:
    """A lightweight pointer to a DataFrame registered in the Arrow server.

    The bytes are paged directly over the local Arrow socket by the viewer;
    only this handle crosses the control plane.
    """

    handle: str
    rows: int
    cols: int
    schema: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class ExecuteResult:
    status: Status
    stdout: str
    value: str | None
    traceback: str | None
    # Marks human REPL pokes (inspection) vs Claude's logged actions (experiment).
    # Ticket 71: ephemeral cells run with MLflow autolog suppressed in the kernel.
    ephemeral: bool
    # Present when the cell evaluated to a DataFrame (the display hook fired).
    df: DfHandle | None = None
    # Every rich MIME bundle the kernel emitted (image/png, text/html, …),
    # minus the DataFrame handle MIME (already surfaced as `df`). The viewer
    # picks the best rendering per display. Default factory so the frozen
    # dataclass keeps a per-instance list.
    displays: list[Display] = field(default_factory=list)


# Prepended to every cell so the MLflow autolog gate is installed *in the kernel*
# (where autolog patches and `.fit` actually run) and the per-cell ephemeral flag
# is set before the user's code runs. The gate install is idempotent and cheap
# after the first call. Each cell sets the flag explicitly — ephemeral cells set
# True, normal cells set False — so no reset statement is needed (which keeps the
# user's trailing expression as the last top-level statement, preserving the
# `execute_result` value IPython echoes). The setup lines run before the user
# code; a literal newline then re-establishes column 0 for the user's first line.
_PREAMBLE_TEMPLATE: Final[str] = (
    "import kiln_sidecar.autolog_gate as _kiln_gate\n"
    "_kiln_gate.install_autolog_gate()\n"
    "_kiln_gate.is_ephemeral.set({ephemeral})\n"
    "del _kiln_gate\n"
)


def _with_ephemeral_preamble(code: str, *, ephemeral: bool) -> str:
    """Inject the autolog-gate setup + per-cell ephemeral flag ahead of `code`."""
    return _PREAMBLE_TEMPLATE.format(ephemeral=ephemeral) + code


class Executor:
    _TIMEOUT_SECONDS: Final[float] = 600.0

    def __init__(self, kernel: Kernel) -> None:
        self._kernel = kernel

    def run(self, code: str, *, ephemeral: bool = False) -> ExecuteResult:
        manager = self._kernel.require_manager()
        client = manager.client()
        client.start_channels()
        code = _with_ephemeral_preamble(code, ephemeral=ephemeral)
        try:
            # wait_for_ready blocks until the kernel's ZMQ channels are fully
            # initialised and the kernel_info_reply handshake is complete.
            # Without this, execute_interactive races with channel setup and times out.
            # Kept inside the try so a timeout still runs stop_channels() in finally.
            client.wait_for_ready(timeout=60.0)
            stdout_buf = io.StringIO()
            value: str | None = None
            traceback: str | None = None
            df: DfHandle | None = None
            displays: list[Display] = []

            def on_iopub(msg: dict[str, object]) -> None:
                nonlocal value, traceback, df
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
                elif msg_type in ("execute_result", "display_data"):
                    data = content.get("data", {})
                    if isinstance(data, dict):
                        plain = data.get("text/plain")
                        if isinstance(plain, str):
                            value = plain
                        parsed = _parse_df_handle(data.get(DF_MIME))
                        if parsed is not None:
                            df = parsed
                        # Fan out every MIME rendering for the viewer. The DF
                        # handle MIME is excluded — it's already surfaced as `df`,
                        # and the plot panel must never try to render it.
                        displays.extend(_collect_displays(data, content.get("metadata")))
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
                ephemeral=ephemeral,
                df=df,
                displays=displays,
            )
        finally:
            client.stop_channels()


def _collect_displays(data: object, metadata_raw: object) -> list[Display]:
    """Fan a MIME bundle out into one `Display` per rendering.

    The DataFrame handle MIME is dropped (it's surfaced separately as `df`).
    Non-string keys/values are skipped/empty-coerced so a malformed bundle
    degrades to fewer displays rather than raising in the iopub callback.
    """
    if not isinstance(data, dict):
        return []
    metadata: dict[str, object] = {}
    if isinstance(metadata_raw, dict):
        for key, value in metadata_raw.items():
            if isinstance(key, str):
                metadata[key] = value
    displays: list[Display] = []
    for mime, payload in data.items():
        if not isinstance(mime, str) or mime == DF_MIME:
            continue
        displays.append(
            Display(
                mime=mime,
                payload=payload if isinstance(payload, str) else "",
                metadata=metadata,
            )
        )
    return displays


def _parse_df_handle(raw: object) -> DfHandle | None:
    """Parse a kiln DataFrame handle payload from an iopub MIME bundle.

    The display hook emits the payload as a JSON string under DF_MIME. Returns
    None for anything that does not look like a well-formed handle, so a
    malformed payload degrades to "no DataFrame" rather than raising.
    """
    if not isinstance(raw, str):
        return None
    try:
        loaded: object = json.loads(raw)
    except json.JSONDecodeError:
        return None
    if not isinstance(loaded, dict):
        return None
    handle = loaded.get("kiln/handle")
    rows = loaded.get("rows")
    cols = loaded.get("cols")
    schema = loaded.get("schema")
    if not isinstance(handle, str) or not handle:
        return None
    if not isinstance(rows, int) or not isinstance(cols, int):
        return None
    if not isinstance(schema, list):
        return None
    return DfHandle(
        handle=handle,
        rows=rows,
        cols=cols,
        schema=tuple(str(col) for col in schema),
    )
