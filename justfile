set shell := ["bash", "-cu"]
set positional-arguments

# Workspace layout:
#   src/, src-tauri/    Tauri v2 + SvelteKit-static frontend (root)
#   sidecar/            uv-managed Python sidecar
#   docs/plans/         master plan + per-ticket handoffs

default:
    @just --list --unsorted

# --- bootstrap ------------------------------------------------------------

# One-time setup: install mise tools, activate pnpm via corepack, install JS + Python deps.
bootstrap:
    mise install
    corepack enable pnpm
    just install

# Install both halves of the workspace.
install: install-js install-py

install-js:
    pnpm install --frozen-lockfile=false

install-py:
    cd sidecar && uv sync --group dev

# --- dev / run ------------------------------------------------------------

# Launch the Tauri app in dev mode (vite + cargo run, hot reload).
dev:
    pnpm tauri dev

# Run the Python sidecar standalone (useful while iterating on it).
sidecar *ARGS:
    cd sidecar && uv run kiln-sidecar "$@"

# Open an IPython REPL inside the sidecar venv (for ad-hoc poking).
repl:
    cd sidecar && uv run --with ipython ipython

# --- build ----------------------------------------------------------------

build: build-js build-tauri

build-js:
    pnpm build

build-tauri:
    pnpm tauri build

# --- test -----------------------------------------------------------------

test: test-rs test-py test-js

test-rs:
    cargo test --manifest-path src-tauri/Cargo.toml

test-py:
    cd sidecar && uv run pytest

test-js:
    pnpm check
    pnpm lint

# --- lint / format --------------------------------------------------------

lint: lint-rs lint-py lint-js

lint-rs:
    cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

lint-py:
    cd sidecar && uv run ruff check . && uv run ty check

lint-js: lint-js-check lint-js-eslint

lint-js-check:
    pnpm check

lint-js-eslint:
    pnpm lint

fmt: fmt-rs fmt-py fmt-js

fmt-rs:
    cargo fmt --manifest-path src-tauri/Cargo.toml

fmt-py:
    cd sidecar && uv run ruff format . && uv run ruff check --fix .

fmt-js:
    pnpm prettier --write .
    pnpm lint:fix

# --- hooks ----------------------------------------------------------------

hooks-install:
    prek install --install-hooks

hooks-run:
    prek run --all-files

# --- clean ----------------------------------------------------------------

clean:
    rm -rf node_modules build dist .svelte-kit src-tauri/target sidecar/.venv sidecar/.pytest_cache sidecar/.ruff_cache sidecar/.mypy_cache
