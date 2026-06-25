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


def _install_df_hook(executor: Executor) -> None:
    # In the running app this runs once at sidecar boot (see __main__).
    install = executor.run(
        "import kiln_sidecar.df_display as _dfd; _dfd.install()",
        ephemeral=True,
    )
    assert install.status == "ok", install.traceback


def test_dataframe_becomes_a_handle(executor: Executor) -> None:
    _install_df_hook(executor)
    result = executor.run("import pandas as pd; df = pd.DataFrame({'a': [1, 2, 3]}); df")
    assert result.status == "ok", result.traceback
    assert result.df is not None
    assert result.df.rows == 3
    assert result.df.cols == 1
    assert result.df.handle  # opaque, non-empty id
    assert result.df.schema == ("a",)


def test_dataframe_default_html_repr_is_suppressed(executor: Executor) -> None:
    # The whole point of the hook: the giant default HTML repr must NOT be sent.
    _install_df_hook(executor)
    result = executor.run("import pandas as pd; pd.DataFrame({'a': [1, 2, 3]})")
    assert result.df is not None
    # The plain-text value, if present, must not carry the DataFrame's HTML table.
    assert "<table" not in (result.value or "")


def test_non_dataframe_value_has_no_handle(executor: Executor) -> None:
    _install_df_hook(executor)
    result = executor.run("1 + 1")
    assert result.df is None
    assert result.value == "2"


def test_polars_dataframe_becomes_a_handle(executor: Executor) -> None:
    # polars is a dev dependency, so this path is always exercised here. In
    # production it is optional — the hook registers it by name and never
    # imports polars unless a polars frame is actually displayed.
    pytest.importorskip("polars")
    _install_df_hook(executor)
    result = executor.run("import polars as pl; pl.DataFrame({'a': [1, 2], 'b': [3, 4]})")
    assert result.status == "ok", result.traceback
    assert result.df is not None
    assert result.df.rows == 2
    assert result.df.cols == 2
    assert result.df.schema == ("a", "b")
