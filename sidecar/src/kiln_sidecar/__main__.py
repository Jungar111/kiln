"""Kiln sidecar entrypoint — line-delimited JSON-RPC 2.0 loop over stdio."""

from __future__ import annotations

import sys

from kiln_sidecar.rpc import Dispatcher


def main() -> int:
    dispatcher = Dispatcher()
    for line in sys.stdin:
        stripped = line.strip()
        if not stripped:
            continue
        sys.stdout.write(dispatcher.handle(stripped))
        sys.stdout.write("\n")
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
