"""Sidecar entrypoint.

Real implementation lands in the Phase 1 tickets (see
docs/plans/tickets/). This stub exists so `uv run kiln-sidecar` succeeds
during dev-env verification.
"""

from __future__ import annotations

import sys


def main() -> int:
    print(
        "kiln-sidecar: stub. Replace in ticket 11 (sidecar bootstrap).",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
