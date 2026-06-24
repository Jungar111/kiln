"""The sidecar runtime imports must succeed.

These are the headline native packages. If any of these fails to import on
a clean install we want to know *before* anything else stops working.
"""

from __future__ import annotations

import importlib


def test_runtime_imports() -> None:
    for name in (
        "ipykernel",
        "jupyter_client",
        "zmq",
        "mlflow",
        "pyarrow",
        "pyarrow.flight",
    ):
        importlib.import_module(name)
