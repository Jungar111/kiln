from __future__ import annotations

import json

from kiln_sidecar.rpc import Dispatcher


def test_ping_returns_pong() -> None:
    dispatcher = Dispatcher()
    raw = dispatcher.handle(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "ping"}))
    assert json.loads(raw) == {"jsonrpc": "2.0", "id": 1, "result": "pong"}


def test_unknown_method_returns_error() -> None:
    dispatcher = Dispatcher()
    raw = dispatcher.handle(json.dumps({"jsonrpc": "2.0", "id": 7, "method": "no.such"}))
    payload = json.loads(raw)
    assert payload["error"]["code"] == -32601  # method not found
    assert payload["id"] == 7
