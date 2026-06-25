"""Kiln sidecar entrypoint — line-delimited JSON-RPC 2.0 loop over stdio."""

from __future__ import annotations

import sys
from pathlib import Path

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
        ephemeral_raw = params.get("ephemeral", False)
        ephemeral = ephemeral_raw if isinstance(ephemeral_raw, bool) else False
        result = executor.run(code, ephemeral=ephemeral)
        return {
            "status": result.status,
            "stdout": result.stdout,
            "value": result.value,
            "traceback": result.traceback,
            "ephemeral": result.ephemeral,
        }

    def approve_checkpoint(params: dict[str, JsonValue]) -> JsonValue:
        # mlflow is heavy; import lazily so it never delays sidecar startup.
        import mlflow

        from kiln_sidecar.checkpoint import ProposeExperiment
        from kiln_sidecar.mlflow_runs import start_run_with_decisions

        proposal_raw = params.get("proposal")
        if not isinstance(proposal_raw, dict):
            raise ValueError("`proposal` must be an object")
        mlflow.set_tracking_uri(f"sqlite:///{Path.cwd() / 'mlruns.db'}")
        proposal = ProposeExperiment.model_validate(proposal_raw)
        return {"run_id": start_run_with_decisions(proposal)}

    dispatcher.register("execute", execute)
    dispatcher.register("approve_checkpoint", approve_checkpoint)
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
