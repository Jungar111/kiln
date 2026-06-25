"""Pure DataFrame paging + summary logic over Arrow tables.

Kept separate from the HTTP server so the slicing/sorting/summary behaviour is
testable without a socket.
"""

from __future__ import annotations

from typing import Protocol, cast

import pyarrow as pa
import pyarrow.compute as pc


class _Scalar(Protocol):
    def as_py(self) -> object: ...


class _Reductions(Protocol):
    """The pyarrow.compute reductions we call.

    ``pyarrow.compute`` generates these functions at runtime, so the published
    stubs don't declare them. Same Protocol+cast pattern as ``kernel.py`` and
    ``df_display.py`` use for other incomplete pyarrow stubs.
    """

    def min(self, values: object, /) -> _Scalar: ...
    def max(self, values: object, /) -> _Scalar: ...
    def mean(self, values: object, /) -> _Scalar: ...


_reduce = cast("_Reductions", pc)


def page_table(
    table: pa.Table,
    offset: int,
    limit: int,
    sort_by: str | None,
    sort_dir: str,
) -> pa.Table:
    """Return `limit` rows from `offset`, optionally sorted by one column."""
    if sort_by is not None and sort_by in table.column_names:
        order = "descending" if sort_dir == "desc" else "ascending"
        table = table.sort_by([(sort_by, order)])
    return table.slice(max(offset, 0), max(limit, 0))


def summarise(table: pa.Table) -> pa.Table:
    """Per-column dtype, null count, and (numeric) min/max/mean.

    The bar from spec §5.2: enough to smell a degenerate distribution or a
    leakage column. Distribution sparklines are a deliberate fast-follow.
    """
    rows: list[dict[str, object]] = []
    for name in table.column_names:
        col = table.column(name)
        is_numeric = pa.types.is_integer(col.type) or pa.types.is_floating(col.type)
        rows.append(
            {
                "column": name,
                "dtype": str(col.type),
                "nulls": col.null_count,
                "min": _reduce.min(col).as_py() if is_numeric else None,
                "max": _reduce.max(col).as_py() if is_numeric else None,
                "mean": _reduce.mean(col).as_py() if is_numeric else None,
            }
        )
    return pa.Table.from_pylist(rows)


def to_ipc(table: pa.Table) -> bytes:
    """Serialise a table to an Arrow IPC stream the webview can `tableFromIPC`."""
    sink = pa.BufferOutputStream()
    with pa.ipc.new_stream(sink, table.schema) as writer:
        writer.write_table(table)
    return sink.getvalue().to_pybytes()
