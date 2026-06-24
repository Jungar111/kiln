"""Kiln sidecar entrypoint — line-delimited JSON-RPC 2.0 loop over stdio."""

from __future__ import annotations

import sys

from kiln_sidecar.execute import Executor
from kiln_sidecar.kernel import Kernel
from kiln_sidecar.rpc import Dispatcher, JsonValue


def main() -> int:
    kernel = Kernel()
    kernel.start()
    executor = Executor(kernel)
    dispatcher = Dispatcher()

    def execute(params: dict[str, JsonValue]) -> JsonValue:
        code = params.get("code")
        if not isinstance(code, str):
            raise ValueError("`code` must be a string")
        result = executor.run(code)
        return {
            "status": result.status,
            "stdout": result.stdout,
            "value": result.value,
            "traceback": result.traceback,
        }

    dispatcher.register("execute", execute)
    try:
        for line in sys.stdin:
            stripped = line.strip()
            if not stripped:
                continue
            sys.stdout.write(dispatcher.handle(stripped))
            sys.stdout.write("\n")
            sys.stdout.flush()
    finally:
        kernel.shutdown()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
