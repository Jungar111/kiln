# Kiln

> _A studio where a professional data scientist's findings become production-grade and stay findable — instead of scattered across CSVs, screenshots, and chat scrollback._

Kiln is a cross-platform desktop harness for **Claude-driven data-science experimentation**. Claude writes experiments, runs the IPython REPL, and logs to MLflow. The human operates one level up — reading code, reading results, and making architectural and data-science decisions, not implementation decisions.

**Status:** very early. The full product spec is in [`spec.md`](./spec.md); the implementation plan is in [`docs/plans/`](./docs/plans/).

**Public & open source.** This repository is open to the world. Do not commit secrets — `gitleaks` runs on every commit via `prek`.

---

## Architecture

```
┌────────────────────────────────┐
│  Tauri webview (SvelteKit)     │  control plane + bulk data viewer
│  • chat + premise gate         │
│  • DataFrame explorer          │
│  • plot viewer                 │
│  • run comparison              │
└──┬───────────────────────────┬─┘
   │ Tauri IPC (control plane)│ Arrow IPC (bulk data, direct)
   ▼                          ▼
┌─────────────────────┐  ┌──────────────────────────────┐
│  Rust core (Tauri)  │──▶  Python sidecar (uv venv)    │
│  • process lifecycle│  │  • IPython kernel (ZMQ)      │
│  • kernel client    │  │  • MLflow (sqlite backend)   │
│  • checkpoint routing│ │  • Arrow IPC server          │
└─────────────────────┘  └──────────────────────────────┘
```

## Toolchain

Everything is pinned in `.mise.toml` and orchestrated through `just`.

| Concern       | Tool                                            |
| ------------- | ----------------------------------------------- |
| Versions      | [mise](https://mise.jdx.dev)                    |
| Commands      | [just](https://github.com/casey/just)           |
| Hooks         | [prek](https://github.com/j178/prek)            |
| Desktop shell | [Tauri v2](https://tauri.app)                   |
| Frontend      | SvelteKit + `adapter-static`, TypeScript        |
| Sidecar       | Python 3.12 via [uv](https://docs.astral.sh/uv) |
| Tracking      | [MLflow](https://mlflow.org) (local sqlite)     |

## Getting started

```bash
# 1. install pinned tooling (python, node, rust, uv, just, prek)
mise install

# 2. one-time bootstrap (corepack + pnpm install + uv sync + prek hooks)
just bootstrap
just hooks-install

# 3. run the desktop app
just dev
```

## Layout

```
.
├── src/                # SvelteKit frontend
├── src-tauri/          # Rust core (Tauri v2)
├── sidecar/            # Python sidecar (uv project)
├── docs/plans/         # Implementation plan + per-ticket handoffs
├── spec.md             # Product spec
├── justfile            # All workspace commands
├── .mise.toml          # Pinned toolchain
└── .pre-commit-config.yaml
```

## Common commands

```bash
just dev            # Tauri dev (hot reload)
just build          # production bundle
just test           # rust + python + svelte-check
just lint           # clippy + ruff + mypy + svelte-check
just fmt            # rustfmt + ruff format + prettier
just hooks-run      # run every prek hook against every file
just sidecar        # run the Python sidecar standalone
just repl           # IPython REPL inside the sidecar venv
```

## Contributing

Read [`CLAUDE.md`](./CLAUDE.md) first — it covers the conventions, the public-repo discipline, and the per-ticket workflow.

## License

MIT — see [`LICENSE`](./LICENSE) (TODO: add).
