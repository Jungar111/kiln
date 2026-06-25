"""Kiln sidecar entrypoint — line-delimited JSON-RPC 2.0 loop over stdio."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import cast

from kiln_sidecar.arrow_server import ArrowServer, FrameRegistry
from kiln_sidecar.execute import Executor
from kiln_sidecar.kernel import Kernel
from kiln_sidecar.rpc import Dispatcher, JsonValue


def main() -> int:
    kernel = Kernel()
    kernel.start()
    executor = Executor(kernel)

    # Local-only Arrow Flight server. DataFrames stream from here straight to the
    # webview — their bytes never cross the Rust IPC control plane.
    registry = FrameRegistry()
    arrow_server = ArrowServer(registry, host="127.0.0.1", port=0)
    arrow_server.start()

    dispatcher = Dispatcher()

    def arrow_port(_: dict[str, JsonValue]) -> JsonValue:
        return {"port": arrow_server.port}

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

    def close_run(params: dict[str, JsonValue]) -> JsonValue:
        import mlflow

        from kiln_sidecar.mlflow_runs import VERDICTS, Verdict
        from kiln_sidecar.mlflow_runs import close_run as close

        run_id = params.get("run_id")
        verdict = params.get("verdict")
        if not isinstance(run_id, str) or verdict not in VERDICTS:
            raise ValueError("close_run needs run_id:str and verdict in keep|kill|iterate")
        mlflow.set_tracking_uri(f"sqlite:///{Path.cwd() / 'mlruns.db'}")
        # verdict is one of VERDICTS, i.e. a Verdict, proven by the guard above.
        close(run_id, cast("Verdict", verdict))
        return {"run_id": run_id, "verdict": verdict}

    dispatcher.register("execute", execute)
    dispatcher.register("approve_checkpoint", approve_checkpoint)
    dispatcher.register("close_run", close_run)
    dispatcher.register("arrow_port", arrow_port)
    try:
        for line in sys.stdin:
            stripped = line.strip()
            if not stripped:
                continue
            sys.stdout.write(dispatcher.handle(stripped))
            sys.stdout.write("\n")
            sys.stdout.flush()
    finally:
        arrow_server.shutdown()
        kernel.shutdown()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
