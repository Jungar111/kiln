"""Tiny JSON-RPC 2.0 dispatcher.

We deliberately do not pull in a JSON-RPC library — the surface is small
enough that we'd be paying dependency cost for almost no code. Conforms to
https://www.jsonrpc.org/specification.
"""

from __future__ import annotations

import json
from collections.abc import Callable
from typing import Final, cast

JsonScalar = str | int | float | bool | None
JsonValue = JsonScalar | list["JsonValue"] | dict[str, "JsonValue"]
Method = Callable[[dict[str, JsonValue]], JsonValue]

PARSE_ERROR: Final[int] = -32700
INVALID_REQUEST: Final[int] = -32600
METHOD_NOT_FOUND: Final[int] = -32601
INTERNAL_ERROR: Final[int] = -32603


class Dispatcher:
    def __init__(self) -> None:
        self._methods: dict[str, Method] = {
            "ping": _ping,
            "version": _version,
        }

    def register(self, name: str, fn: Method) -> None:
        self._methods[name] = fn

    def handle(self, raw: str) -> str:
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError:
            return _error(None, PARSE_ERROR, "parse error")

        if not isinstance(parsed, dict):
            return _error(None, INVALID_REQUEST, "request must be a JSON object")

        # json.loads only yields JsonValue-typed values; the isinstance check
        # above confirms the top level is an object, so the contents conform.
        request: dict[str, JsonValue] = cast("dict[str, JsonValue]", parsed)

        request_id = request.get("id")
        method_name = request.get("method")
        if not isinstance(method_name, str):
            return _error(request_id, INVALID_REQUEST, "method must be a string")

        method = self._methods.get(method_name)
        if method is None:
            return _error(request_id, METHOD_NOT_FOUND, f"unknown method {method_name!r}")

        params_raw = request.get("params", {})
        # Only by-name (object) params are supported. By-position (array) params
        # are intentionally ignored — the trusted single client always sends
        # named params, so non-object params coerce to an empty mapping.
        params: dict[str, JsonValue] = params_raw if isinstance(params_raw, dict) else {}

        try:
            result = method(params)
        except Exception as exc:
            return _error(request_id, INTERNAL_ERROR, str(exc))
        return json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result})


def _ping(_: dict[str, JsonValue]) -> JsonValue:
    return "pong"


def _version(_: dict[str, JsonValue]) -> JsonValue:
    from kiln_sidecar import __version__

    return {"sidecar": __version__, "kernel": "ipython"}


def _error(request_id: JsonValue, code: int, message: str) -> str:
    return json.dumps(
        {"jsonrpc": "2.0", "id": request_id, "error": {"code": code, "message": message}}
    )
