# CLAUDE.md — Kiln

> **⚠️ THIS REPOSITORY IS PUBLIC AND OPEN SOURCE.**
> Source lives at https://github.com/Jungar111/kiln.
> **Anything committed here is visible to the world — forever, even after deletion.**
>
> - Never commit secrets, API keys, tokens, internal URLs, customer data, or anything you wouldn't put on a billboard.
> - `gitleaks` runs as a `prek` hook on every commit (see `.pre-commit-config.yaml`). Do not `--no-verify` past it.
> - If a secret slips in, **rotate it immediately** — rewriting git history does not undo public exposure.
> - The repo is intentionally `.env`-aware (see `.gitignore`); never disable those lines.

## What this is

Kiln is a desktop harness (Tauri + SvelteKit + Python sidecar) for Claude-driven data-science experimentation. The product thesis lives in [`spec.md`](./spec.md). The implementation plan lives in [`docs/plans/`](./docs/plans/).

## Toolchain (managed by `mise`)

All versions are pinned in `.mise.toml`. Run `just bootstrap` once to install everything.

| Layer    | Tool                                          |
| -------- | --------------------------------------------- |
| Versions | `mise`                                        |
| Commands | `just` (see `justfile`)                       |
| Hooks    | `prek` (drop-in `pre-commit` replacement)     |
| Desktop  | Tauri v2 (Rust core)                          |
| Frontend | SvelteKit + adapter-static, TypeScript        |
| Sidecar  | Python 3.12 via `uv` (IPython, MLflow, Arrow) |

## How to work in this repo

1. `just bootstrap` — one-time setup.
2. `just dev` — runs the Tauri app with hot reload.
3. `just lint` / `just test` / `just fmt` — before committing.
4. `just hooks-run` — runs every prek hook against every file; CI parity.

## Type-safety policy (NON-NEGOTIABLE)

We treat Python as a strongly typed language. We treat TypeScript with the same rigour. **There are no exceptions.** Hooks are configured to fail the commit if any of the below appear.

### Python (`sidecar/`)

- **Type checker:** [`ty`](https://github.com/astral-sh/ty) (Astral). Run as `uv run ty check`. Wired into `just lint-py` and the prek hook `ty-check`.
- **Lint:** `ruff` with `ANN` (flake8-annotations) enabled — every function gets argument and return annotations. `ANN401` bans `Any`.
- **Escapes are banned** (enforced by both ruff rules and a `pygrep` prek hook):
  - No `# noqa` (any form).
  - No `# type: ignore` (and `respect-type-ignore-comments = false` in `[tool.ty.overrides.analysis]`).
  - No `# pyright: ignore` / `# ty: ignore`.
- Use `typing.Protocol`, `TypedDict`, `Literal`, `Final`, `NewType`. Reach for `cast()` only when you have proven the runtime invariant in nearby code — and explain why in a one-line comment.

### TypeScript (`src/`)

- **`tsconfig.json`** has the full strict family on: `strict`, `noImplicitAny`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noImplicitOverride`, `noImplicitReturns`, `useUnknownInCatchVariables`.
- **ESLint** runs `typescript-eslint` `strictTypeChecked` + `stylisticTypeChecked` with `--max-warnings 0`.
- **Banned (prek hooks fail on these):**
  - `any` (the explicit type).
  - `@ts-ignore`, `@ts-nocheck`, `@ts-expect-error`.
  - `// eslint-disable*` of any flavour for type rules.
  - `!` non-null assertions.
- Prefer `unknown` + narrowing, `Result`-style discriminated unions, exhaustive `switch` checks.

If a third-party type is genuinely wrong, the fix is a local `.d.ts` declaration, not a suppression.

### Rust (`src-tauri/`)

- `cargo clippy -- -D warnings` is the floor. No `#[allow(...)]` without a one-line justification comment.

## Conventions for Claude

- Pick tickets out of `docs/plans/tickets/` in numeric order unless told otherwise. Each ticket is self-contained for handoff to a fresh agent.
- Tests-first (`superpowers:test-driven-development`) for anything non-trivial in `sidecar/` or `src-tauri/`.
- Frequent, small commits with conventional-commit-style messages (`feat:`, `fix:`, `chore:`, `docs:`).
- Never `git push --force` to `main`. Never bypass pre-commit hooks (`--no-verify`).
- The Python sidecar is the **only** place that may import `mlflow`, `ipykernel`, `pyarrow`. The Rust core never speaks those protocols directly — it talks to the sidecar over the documented IPC.
- DataFrames take the **direct** webview ↔ Python Arrow path. Never marshal a DataFrame through a Tauri command.
