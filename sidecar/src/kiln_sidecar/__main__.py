"""Kiln sidecar entrypoint — line-delimited JSON-RPC 2.0 loop over stdio."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import cast

from kiln_sidecar.execute import Executor
from kiln_sidecar.kernel import Kernel
from kiln_sidecar.rpc import Dispatcher, JsonValue


def _json_metadata(metadata: dict[str, object]) -> dict[str, JsonValue]:
    """Coerce a display's metadata dict to a JSON-safe shape for the reply.

    The metadata originates from a JSON-decoded kernel iopub message, so it is
    already JsonValue-shaped at runtime; the round-trip both proves that and
    narrows the static type. Anything not JSON-encodable (never expected here)
    degrades to an empty object rather than crashing the execute reply.
    """
    try:
        encoded = json.dumps(metadata)
    except (TypeError, ValueError):
        return {}
    # json.loads of a JSON object yields a dict[str, JsonValue]; the dumps above
    # was of a dict, so the top level is always an object.
    return cast("dict[str, JsonValue]", json.loads(encoded))


def main() -> int:
    kernel = Kernel()
    kernel.start()
    executor = Executor(kernel)

    # Install the DataFrame display hook INSIDE the kernel process. jupyter_client
    # runs the kernel as a separate subprocess, so DataFrames live in the kernel's
    # memory — the Arrow server that serves their bytes must run there too. The
    # hook starts that server lazily; arrow_port() reports its port back here.
    install = executor.run(
        "import kiln_sidecar.df_display as _kiln_dfd; _kiln_dfd.install()",
        ephemeral=True,
    )
    if install.status != "ok":
        sys.stderr.write(f"df_display hook failed to install: {install.traceback}\n")
        sys.stderr.flush()

    dispatcher = Dispatcher()

    def arrow_port(_: dict[str, JsonValue]) -> JsonValue:
        # Ask the kernel for its Arrow server port. The port is printed and read
        # back from stdout so it crosses our existing execute roundtrip — no new
        # control channel. Bytes of DataFrames never travel this path.
        probe = executor.run(
            "import kiln_sidecar.df_display as _kiln_dfd; print(_kiln_dfd.arrow_port())",
            ephemeral=True,
        )
        port_text = probe.stdout.strip()
        if probe.status != "ok" or not port_text.isdigit():
            raise RuntimeError(
                f"could not read arrow_port from kernel: {probe.traceback or port_text}"
            )
        return {"port": int(port_text)}

    def execute(params: dict[str, JsonValue]) -> JsonValue:
        code = params.get("code")
        if not isinstance(code, str):
            raise ValueError("`code` must be a string")
        ephemeral_raw = params.get("ephemeral", False)
        ephemeral = ephemeral_raw if isinstance(ephemeral_raw, bool) else False
        result = executor.run(code, ephemeral=ephemeral)
        df: JsonValue = None
        if result.df is not None:
            df = {
                "handle": result.df.handle,
                "rows": result.df.rows,
                "cols": result.df.cols,
                "schema": list(result.df.schema),
            }
        return {
            "status": result.status,
            "stdout": result.stdout,
            "value": result.value,
            "traceback": result.traceback,
            "ephemeral": result.ephemeral,
            "df": df,
            # Rich MIME bundles (image/png, text/html, …) for the plot panel.
            # These are small enough to ride the control plane; DataFrames do
            # not — they stay on the direct Arrow path and are surfaced via the
            # df handle, not here.
            "displays": [
                {"mime": d.mime, "payload": d.payload, "metadata": _json_metadata(d.metadata)}
                for d in result.displays
            ],
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

    def list_runs(params: dict[str, JsonValue]) -> JsonValue:
        from dataclasses import asdict

        import mlflow

        from kiln_sidecar.mlflow_query import list_runs as query_runs

        names_raw = params.get("experiment_names")
        names = (
            [n for n in names_raw if isinstance(n, str)] if isinstance(names_raw, list) else None
        )
        limit_raw = params.get("limit", 50)
        limit = limit_raw if isinstance(limit_raw, int) else 50
        mlflow.set_tracking_uri(f"sqlite:///{Path.cwd() / 'mlruns.db'}")
        return [asdict(run) for run in query_runs(experiment_names=names, limit=limit)]

    dispatcher.register("execute", execute)
    dispatcher.register("approve_checkpoint", approve_checkpoint)
    dispatcher.register("close_run", close_run)
    dispatcher.register("list_runs", list_runs)
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
        # The Arrow server lives in the kernel process, so killing the kernel
        # tears it down with no separate shutdown needed.
        kernel.shutdown()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
