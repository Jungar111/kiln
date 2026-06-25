"""DataFrame -> Arrow handle display hook, installed inside the IPython kernel.

When a cell evaluates to a ``pandas`` (or ``polars``) DataFrame, the kernel
must NOT emit the giant default HTML repr. Instead we:

1. Register the frame's Arrow table in a process-local :class:`FrameRegistry`.
2. Emit a tiny JSON handle under a custom MIME type
   (``application/vnd.kiln.df+json``).
3. Suppress the default ``text/html`` repr so the heavy table never crosses the
   wire — the viewer pages the frame over the local Arrow socket instead.

**Why the server lives here, in the kernel process.**
``jupyter_client`` runs the kernel as a *separate subprocess*, so the
DataFrames live in this process's memory — not the sidecar parent's. The Arrow
Flight / HTTP server must therefore run *inside the kernel* to serve those bytes
with zero copies. ``install()`` starts a single module-level server lazily and
``arrow_port()`` reports its port back to the parent over the control plane.

``pyarrow`` ships without complete py.typed stubs and ``IPython``'s formatter
machinery is dynamically typed; both are pinned behind narrow Protocols so the
call sites we drive are checked.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Final, Protocol, cast

import pyarrow as pa

from kiln_sidecar.arrow_server import ArrowServer, FrameRegistry

if TYPE_CHECKING:
    from collections.abc import Callable

MIME: Final[str] = "application/vnd.kiln.df+json"
HTML_MIME: Final[str] = "text/html"

# Process-local singletons. They live in the *kernel* process (see module
# docstring); the formatter closures register frames into `_REGISTRY` and the
# `ArrowServer` serves them straight to the webview.
_REGISTRY: Final[FrameRegistry] = FrameRegistry()
_SERVER: ArrowServer | None = None


class _Formatter(Protocol):
    """The subset of an IPython MIME formatter we drive."""

    def for_type_by_name(
        self,
        module: str,
        name: str,
        func: Callable[[object], object],
    ) -> object: ...


class _FormatterRegistry(Protocol):
    def get(self, key: str) -> _Formatter | None: ...

    def __getitem__(self, key: str) -> _Formatter: ...

    def __setitem__(self, key: str, value: _Formatter) -> None: ...


class _DisplayFormatter(Protocol):
    formatters: _FormatterRegistry


class _InteractiveShell(Protocol):
    display_formatter: _DisplayFormatter


def install() -> None:
    """Install the DataFrame display hook into the running kernel.

    Idempotent: starts the Arrow server once and registers the formatters once.
    Must be called from inside the kernel (where ``get_ipython()`` is truthy).
    """
    ip = _require_shell()
    _ensure_server()

    formatters = ip.display_formatter.formatters
    kiln_formatter = formatters.get(MIME)
    if kiln_formatter is None:
        kiln_formatter = _new_kiln_formatter(ip)
        formatters[MIME] = kiln_formatter
    html_formatter = formatters[HTML_MIME]

    kiln_formatter.for_type_by_name("pandas.core.frame", "DataFrame", _format_pandas)
    html_formatter.for_type_by_name("pandas.core.frame", "DataFrame", _suppress_html)
    # polars may not be installed; registering by name never imports it eagerly.
    kiln_formatter.for_type_by_name("polars.dataframe.frame", "DataFrame", _format_polars)
    html_formatter.for_type_by_name("polars.dataframe.frame", "DataFrame", _suppress_html)


def arrow_port() -> int:
    """Return the port the in-kernel Arrow server is listening on.

    Starts the server if it is not yet running, so ``arrow_port`` works even
    before any DataFrame has been displayed.
    """
    return _ensure_server().port


def registry() -> FrameRegistry:
    """Return the process-local registry (used by the page/summary servers)."""
    return _REGISTRY


def _ensure_server() -> ArrowServer:
    global _SERVER
    if _SERVER is None:
        server = ArrowServer(_REGISTRY, host="127.0.0.1", port=0)
        server.start()
        _SERVER = server
    return _SERVER


def _format_pandas(obj: object) -> object:
    import pandas as pd

    if not isinstance(obj, pd.DataFrame):
        return None
    # preserve_index=False keeps the schema aligned with the visible columns.
    handle = _REGISTRY.register(pa.Table.from_pandas(obj, preserve_index=False))
    return _payload(handle.id, obj.shape[0], obj.shape[1], [str(c) for c in obj.columns])


def _format_polars(obj: object) -> object:
    import polars as pl

    if not isinstance(obj, pl.DataFrame):
        return None
    handle = _REGISTRY.register(obj.to_arrow())
    return _payload(handle.id, obj.height, obj.width, list(obj.columns))


def _suppress_html(obj: object) -> object:
    # Returning None drops the text/html MIME from the bundle, so the heavy
    # default repr never travels to the webview.
    return None


def _payload(handle: str, rows: int, cols: int, columns: list[str]) -> str:
    return json.dumps({"kiln/handle": handle, "rows": rows, "cols": cols, "schema": columns})


def _require_shell() -> _InteractiveShell:
    from IPython.core.getipython import get_ipython

    ip = get_ipython()
    if ip is None:
        raise RuntimeError("display hook must be installed inside a running kernel")
    # cast: get_ipython() returns the InteractiveShell singleton, which always
    # exposes display_formatter.formatters at runtime; the stubs are incomplete.
    return cast("_InteractiveShell", ip)


def _new_kiln_formatter(ip: _InteractiveShell) -> _Formatter:
    from IPython.core.formatters import BaseFormatter

    class _KilnDfFormatter(BaseFormatter):
        format_type = MIME
        print_method = "_repr_kiln_df_"
        _return_type = str

    # cast: BaseFormatter implements for_type_by_name at runtime; the stubs do
    # not surface it. parent wires it into the display machinery.
    return cast("_Formatter", _KilnDfFormatter(parent=ip))
