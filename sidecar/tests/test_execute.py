from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kiln_sidecar.execute import ExecuteResult, Executor
from kiln_sidecar.kernel import Kernel

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture
def executor() -> Iterator[Executor]:
    kernel = Kernel()
    kernel.start()
    ex = Executor(kernel)
    try:
        yield ex
    finally:
        kernel.shutdown()


def test_expression_returns_value(executor: Executor) -> None:
    result = executor.run("1 + 1")
    assert result == ExecuteResult(
        status="ok", stdout="", value="2", traceback=None, ephemeral=False
    )


def test_print_captures_stdout(executor: Executor) -> None:
    result = executor.run("print('hi')")
    assert result.stdout.strip() == "hi"
    assert result.value is None


def test_exception_returns_error(executor: Executor) -> None:
    result = executor.run("raise ValueError('boom')")
    assert result.status == "error"
    assert "ValueError" in (result.traceback or "")


def test_ephemeral_flag_threads_through(executor: Executor) -> None:
    result = executor.run("x = 1", ephemeral=True)
    assert result.ephemeral is True

    result = executor.run("y = 2")
    assert result.ephemeral is False
