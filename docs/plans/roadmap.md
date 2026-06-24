# Kiln MVP — Master Roadmap

> **For agentic workers:** REQUIRED SUB-SKILL — use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to work tickets task-by-task. Steps inside each ticket use checkbox (`- [ ]`) syntax.

**Goal.** Ship the MVP described in [`../../spec.md`](../../spec.md): a Tauri desktop harness that gates a Claude-driven IPython + MLflow data-science loop on declared, structured premise decisions — and renders the resulting runs in a comparison view that is legible enough to make keep/kill calls without dropping into implementation.

**Tech stack (already wired in Phase 0).**

| Layer       | Choice                                                            |
| ----------- | ----------------------------------------------------------------- |
| Desktop     | Tauri v2 (Rust core)                                              |
| Frontend    | SvelteKit + `adapter-static` + TypeScript                         |
| Sidecar     | Python 3.12 via `uv`; IPython (`jupyter_client` over ZMQ); MLflow |
| Transport   | Tauri IPC (control plane); Arrow IPC over localhost (bulk data)  |
| Tools       | `mise`, `just`, `prek`, `gitleaks`                                |
| Type policy | `ty` (Python), `typescript-eslint strictTypeChecked` — religious  |

**Hard architectural rules (carry into every ticket).**

1. **Big bytes never cross the Rust IPC.** DataFrames go webview ↔ Python directly via Arrow over a localhost socket.
2. **One sidecar per project**, supervised by Rust. The sidecar process hosts the IPython kernel **and** the Arrow server in the same process.
3. **Approval binds to declared decisions, not lines of code.** Declared decisions are persisted as MLflow run **tags** so the thing approved becomes the thing compared.
4. **Type rigour is non-negotiable.** No `Any`, no `# type: ignore`, no `@ts-ignore`. See [`../../CLAUDE.md`](../../CLAUDE.md).

---

## Phase 0 — Repo + tooling ✅ DONE

Shipped in the bootstrap commit. Artefacts: `.mise.toml`, `justfile`, `.pre-commit-config.yaml`, `tsconfig.json` (strict), `eslint.config.js` (strict-type-checked), `sidecar/pyproject.toml` (ruff + ty), scaffolded Tauri + SvelteKit + Python sidecar.

## Phase 1 — Python sidecar bootstrap

**Goal.** A stand-alone, type-safe `kiln-sidecar` process that boots an IPython kernel, exposes a JSON-RPC control surface, and lets a smoke client execute `x = 1; x + 1` and read `2` back.

**Tickets:** [`01-sidecar-deps`](./tickets/01-sidecar-deps.md) → [`02-kernel-lifecycle`](./tickets/02-kernel-lifecycle.md) → [`03-jsonrpc-control`](./tickets/03-jsonrpc-control.md) → [`04-execute-roundtrip`](./tickets/04-execute-roundtrip.md) → [`05-experiment-vs-inspection`](./tickets/05-experiment-vs-inspection.md)

## Phase 2 — Rust core ↔ sidecar plumbing

**Goal.** The Tauri Rust core spawns, supervises, and talks to the sidecar. A Tauri command `execute(code)` returns the sidecar's response. Crashes are caught and surfaced.

**Tickets:** [`10-sidecar-process-supervisor`](./tickets/10-sidecar-process-supervisor.md) → [`11-control-client`](./tickets/11-control-client.md) → [`12-tauri-execute-command`](./tickets/12-tauri-execute-command.md) → [`13-state-events`](./tickets/13-state-events.md)

## Phase 3 — Frontend skeleton + chat

**Goal.** The webview renders three panes (chat, code view stub, results stub) and a real chat that posts to a `chat` Tauri command. Claude integration stub returns canned responses — wire-up only.

**Tickets:** [`20-app-shell`](./tickets/20-app-shell.md) → [`21-chat-pane`](./tickets/21-chat-pane.md) → [`22-claude-client-stub`](./tickets/22-claude-client-stub.md)

## Phase 4 — Checkpoint object + premise gate

**Goal.** The `propose_experiment` tool-schema and the premise-gate modal. Required in-scope slots are validated at the tool boundary. A locked-frame summary is rendered. Approval writes the declared decisions as MLflow run tags.

**Tickets:** [`30-propose-experiment-schema`](./tickets/30-propose-experiment-schema.md) → [`31-slot-validation`](./tickets/31-slot-validation.md) → [`32-premise-gate-ui`](./tickets/32-premise-gate-ui.md) → [`33-mlflow-tag-write`](./tickets/33-mlflow-tag-write.md) → [`34-results-gate-ui`](./tickets/34-results-gate-ui.md)

## Phase 5 — DataFrame explorer (Arrow IPC, the hard one)

**Goal.** A custom kernel display hook registers DataFrames in a side store and emits a lightweight handle. The webview opens an Arrow IPC connection to the sidecar and pages the frame, with server-side sort / filter / paginate plus summary stats.

**Tickets:** [`40-arrow-server`](./tickets/40-arrow-server.md) → [`41-df-handle-hook`](./tickets/41-df-handle-hook.md) → [`42-df-explorer-viewer`](./tickets/42-df-explorer-viewer.md) → [`43-summary-stats`](./tickets/43-summary-stats.md)

## Phase 6 — Plot viewer

**Goal.** Intercept the kernel's `display_data` / `execute_result` MIME bundles (`image/png`, `text/html`) and route them to a plot panel in the webview.

**Tickets:** [`50-mime-router`](./tickets/50-mime-router.md) → [`51-plot-panel`](./tickets/51-plot-panel.md)

## Phase 7 — Run comparison (the keep/kill surface)

**Goal.** Query MLflow via `MlflowClient.search_runs()`, render an inline comparison of params + metrics + declared-decision tags as a first-class diff (e.g. `A: temporal split / B: random split`).

**Tickets:** [`60-mlflow-query`](./tickets/60-mlflow-query.md) → [`61-comparison-view`](./tickets/61-comparison-view.md) → [`62-decision-diff`](./tickets/62-decision-diff.md)

## Phase 8 — REPL access + drift detection

**Goal.** A REPL surface that runs only at checkpoint moments, marked ephemeral (no MLflow side effect). Drift detection re-fires a premise gate when a locked slot value would change.

**Tickets:** [`70-repl-panel`](./tickets/70-repl-panel.md) → [`71-experiment-vs-inspection-enforcement`](./tickets/71-experiment-vs-inspection-enforcement.md) → [`72-drift-detector`](./tickets/72-drift-detector.md)

## Phase 9 — Release plumbing (deferred)

**Goal.** Cut tagged releases without ceremony: a single `just release` bumps versions across `package.json` / Rust / Tauri / Python in lockstep, regenerates `CHANGELOG.md`, tags, and pushes; the tag triggers a matrix CI build that produces signed `.dmg` / `.msi` / `.AppImage` and attaches them to a GitHub release. Stays in plan but is not blocking on the MVP — the MVP ships value before installers.

**Tickets:** [`80-pyinstaller-sidecar`](./tickets/80-pyinstaller-sidecar.md) → [`81-tauri-action-release`](./tickets/81-tauri-action-release.md) → [`82-cocogitto-versioning`](./tickets/82-cocogitto-versioning.md)

---

## Dependency graph

```
Phase 0 ──▶ Phase 1 ──▶ Phase 2 ──▶ Phase 3 ──┬──▶ Phase 4 ──▶ Phase 7
                                              │           │
                                              │           └──▶ Phase 8
                                              │
                                              └──▶ Phase 5 ──▶ Phase 6

Phase 9 ◀── (Phases 1-8 must be feature-complete enough to ship)
```

- Phases 1 → 2 → 3 are strictly sequential.
- Once Phase 3 lands, Phases 4 and 5 are parallelisable across two agents.
- Phase 6 depends on Phase 5's MIME-routing plumbing.
- Phase 7 depends on Phase 4's tag-writing.
- Phase 8 depends on Phase 4's checkpoint object.
- Phase 9 is **deferred**: it does not block any earlier ticket and is picked up only when an installable build becomes the next concrete user need.

---

## What is explicitly **out of scope** for MVP

Repeating the spec verbatim so it doesn't get blurred during execution:

- Worktrees / parallel-experiment + parallel-agent workflows.
- Promotion (session → clean script). Fast-follow, not MVP.
- A DS linter that auto-raises slots from static code analysis. Slots must accept it later.
- Declaration-vs-implementation contract checking ("Claude said temporal split — does the code do `shuffle=False`?").
- Variable inspector + staleness indicator as polished surfaces (ship minimal or skip).
- Remote / cloud data, data connectors, environment-management UI.
- Drift-detection sophistication beyond "a locked slot value would change → re-gate."

If a ticket is tempted to grow into one of these, stop and add a follow-up ticket; do not expand in place.

---

## Glossary (for handoff agents)

- **Premise gate** — the heavy, structured review *before* a chunk of Claude-driven exploration. Approval = the human agreed on the **frame** (target, validation strategy, feature provenance, metric, population). Approval does **not** mean line-by-line code review.
- **Results gate** — the lighter post-compute review: keep / kill / iterate.
- **Drift** — Claude's actions would change a locked slot value. Triggers a re-gate.
- **Checkpoint object** — the `propose_experiment` tool-call payload that the harness fills with structured slots before rendering the gate.
- **Slot** — a single labelled, required-when-in-scope decision (validation strategy, target definition, etc.).
- **Experiment commands** — Claude's logged, reproducible kernel actions. Land in the MLflow run.
- **Inspection commands** — human pokes via the REPL. Ephemeral; do not land in the run or any promoted script.
- **Sidecar** — the Python process. One per project. Hosts the IPython kernel + Arrow server + thin MLflow wrapper.

## Risk register (from [`../../spec.md`](../../spec.md) §10, restated as gates)

| Risk | Concrete ticket-level guardrail |
| --- | --- |
| Disempowered-reviewer trap | Required slots cannot be empty (ticket 31); auto-flags from harness independent of Claude (deferred — Phase 7 ticket 62 places the hook) |
| Checkpoint fatigue | Premise-level only; no per-fit gates anywhere in tickets 30–34 |
| Premise drift | Drift detector ticket 72 compares against locked slot values, re-fires gate |
| Self-grading weakness | Phase 7 keeps the comparison view independent of Claude's summary |
| Long-running runs block the kernel | Decided up front in ticket 02: kernel `execute_interactive` runs on an async task in the sidecar; UI keeps polling. Background queue is **out of MVP scope** |

## Open questions (deferred decisions)

The spec leaves four open questions (§11). They are surfaced inside the tickets that own them:

- **Drift detection mechanism details** → ticket 72.
- **Long-running execution model** → ticket 02 decides "async task in sidecar, no queue" for MVP.
- **Checkpoint UX detail** (what approval visually commits to; how the results gate reads keep/kill/iterate) → tickets 32 and 34.
- **Promotion design** → out of MVP; placeholder noted in ticket 71 so its constraints (the experiment/inspection separation) are not painted into a corner.
