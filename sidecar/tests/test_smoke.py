"""Smoke test — verifies the sidecar package is importable.

Real test suite begins in Phase 1.
"""

from __future__ import annotations

from kiln_sidecar import __version__


def test_version_is_set() -> None:
    assert __version__ == "0.1.0"
