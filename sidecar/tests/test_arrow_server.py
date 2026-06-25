from __future__ import annotations

from typing import TYPE_CHECKING

import pyarrow as pa
import pyarrow.flight as fl
import pytest

from kiln_sidecar.arrow_server import ArrowServer, FrameRegistry

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture
def server() -> Iterator[ArrowServer]:
    registry = FrameRegistry()
    srv = ArrowServer(registry, host="127.0.0.1", port=0)
    srv.start()
    try:
        yield srv
    finally:
        srv.shutdown()


def test_registered_frame_is_streamed(server: ArrowServer) -> None:
    handle = server.registry.register(pa.table({"x": [1, 2, 3], "y": ["a", "b", "c"]}))
    client = fl.connect(f"grpc+tcp://127.0.0.1:{server.port}")
    reader = client.do_get(fl.Ticket(handle.encode()))
    table = reader.read_all()
    assert table.num_rows == 3
    assert table.column_names == ["x", "y"]


def test_unknown_handle_raises(server: ArrowServer) -> None:
    client = fl.connect(f"grpc+tcp://127.0.0.1:{server.port}")
    with pytest.raises(fl.FlightError):
        reader = client.do_get(fl.Ticket(b"does-not-exist"))
        reader.read_all()


def test_port_is_assigned(server: ArrowServer) -> None:
    # port=0 asks the OS for a free port; the server must surface the real one.
    assert server.port > 0
