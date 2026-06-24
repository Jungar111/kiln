"""Owns the lifecycle of a single IPython kernel.

jupyter_client's KernelManager already covers spawning, ZMQ wiring,
and process supervision. We wrap it so the rest of the codebase only
ever sees our typed surface — the upstream API is broad and untyped.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Final, Protocol, cast

from jupyter_client.manager import KernelManager

if TYPE_CHECKING:
    from collections.abc import Callable, Mapping


class _KernelClientProtocol(Protocol):
    """Describes the subset of KernelClient we actually call in Executor.

    KernelClient ships without complete py.typed stubs. This Protocol pins
    exactly what Executor needs so ty can check our call sites.
    """

    def start_channels(self) -> None: ...

    def stop_channels(self) -> None: ...

    def wait_for_ready(self, timeout: float | None = None) -> None: ...

    def execute_interactive(
        self,
        code: str,
        *,
        output_hook: Callable[[dict[str, object]], None],
        timeout: float,
    ) -> Mapping[str, object]: ...


class _KernelManagerProtocol(Protocol):
    """Describes the subset of KernelManager we actually call.

    KernelManager ships without complete py.typed stubs; ty cannot verify
    the attribute and method signatures below at import time. This Protocol
    captures exactly what Kernel uses so ty can check our call sites, while
    the cast() at construction time asserts the runtime invariant: the object
    returned by KernelManager() always implements these methods.
    """

    def start_kernel(self) -> None: ...

    def is_alive(self) -> bool: ...

    def shutdown_kernel(self, *, now: bool) -> None: ...

    def client(self) -> _KernelClientProtocol: ...


class Kernel:
    """Wraps a jupyter_client.KernelManager to expose a minimal typed surface."""

    KERNEL_NAME: Final[str] = "python3"

    def __init__(self) -> None:
        self._manager: _KernelManagerProtocol | None = None

    def start(self) -> None:
        """Start the IPython kernel subprocess. Raises if already started."""
        if self._manager is not None:
            raise RuntimeError("kernel is already started")
        raw = KernelManager(kernel_name=self.KERNEL_NAME)
        # cast: KernelManager always exposes start_kernel / is_alive / shutdown_kernel
        # at runtime; the missing stubs are an upstream packaging gap, not an API gap.
        manager = cast("_KernelManagerProtocol", raw)
        manager.start_kernel()
        self._manager = manager

    def is_alive(self) -> bool:
        """Return True if the kernel subprocess is running."""
        return self._manager is not None and self._manager.is_alive()

    def require_manager(self) -> _KernelManagerProtocol:
        """Return the manager or raise if the kernel has not been started."""
        if self._manager is None:
            raise RuntimeError("kernel is not started")
        return self._manager

    def shutdown(self) -> None:
        """Terminate the kernel subprocess immediately. No-op if not running."""
        if self._manager is None:
            return
        self._manager.shutdown_kernel(now=True)
        self._manager = None
