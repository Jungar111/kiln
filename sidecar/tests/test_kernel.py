from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kiln_sidecar.kernel import Kernel

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture
def kernel() -> Iterator[Kernel]:
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
