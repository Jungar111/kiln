from __future__ import annotations

import http.client
import json
from typing import TYPE_CHECKING

import pyarrow as pa
import pytest

from kiln_sidecar.arrow_server import ArrowServer, FrameRegistry
from kiln_sidecar.df_pages import page_table, summarise

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture
def server() -> Iterator[ArrowServer]:
    srv = ArrowServer(FrameRegistry(), host="127.0.0.1", port=0)
    srv.start()
    try:
        yield srv
    finally:
        srv.shutdown()


def _post(port: int, path: str, body: dict[str, object]) -> http.client.HTTPResponse:
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5.0)
    conn.request("POST", path, body=json.dumps(body), headers={"content-type": "application/json"})
    return conn.getresponse()


def _post_table(port: int, path: str, body: dict[str, object]) -> pa.Table:
    resp = _post(port, path, body)
    assert resp.status == 200, resp.read()
    return pa.ipc.open_stream(resp.read()).read_all()


def test_page_streams_arrow(server: ArrowServer) -> None:
    handle = server.registry.register(pa.table({"x": [1, 2, 3], "y": ["a", "b", "c"]}))
    table = _post_table(server.port, "/page", {"handle": handle.id, "offset": 0, "limit": 2})
    assert table.column_names == ["x", "y"]
    assert table.num_rows == 2


def test_page_sorts_descending(server: ArrowServer) -> None:
    handle = server.registry.register(pa.table({"x": [3, 1, 2]}))
    table = _post_table(
        server.port, "/page", {"handle": handle.id, "sortBy": "x", "sortDir": "desc"}
    )
    assert table.column("x").to_pylist() == [3, 2, 1]


def test_summary_reports_nulls_and_numeric_stats(server: ArrowServer) -> None:
    handle = server.registry.register(pa.table({"x": [1, 2, None], "label": ["a", "b", "c"]}))
    table = _post_table(server.port, "/summary", {"handle": handle.id})
    by_col = {row["column"]: row for row in table.to_pylist()}
    assert by_col["x"]["nulls"] == 1
    assert by_col["x"]["max"] == 2
    assert by_col["label"]["min"] is None


def test_unknown_handle_404s(server: ArrowServer) -> None:
    resp = _post(server.port, "/page", {"handle": "nope"})
    assert resp.status == 404


def test_port_is_assigned(server: ArrowServer) -> None:
    assert server.port > 0


def test_page_table_pure_slice() -> None:
    table = pa.table({"x": list(range(10))})
    sliced = page_table(table, offset=2, limit=3, sort_by=None, sort_dir="asc")
    assert sliced.column("x").to_pylist() == [2, 3, 4]


def test_summarise_pure() -> None:
    row = summarise(pa.table({"n": [1, 2, 3]})).to_pylist()[0]
    assert row["dtype"].startswith("int")
    assert row["mean"] == 2.0
