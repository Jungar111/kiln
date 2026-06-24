# Ticket 06 — Harden the Phase 1 contract: error-path + guard tests

**Phase:** 1 (sidecar bootstrap) — follow-up
**Depends on:** [03](./03-jsonrpc-control.md), [04](./04-execute-roundtrip.md)
**Blocks (soft):** [11](./11-control-client.md), [12](./12-tauri-execute-command.md) — the Rust control client will depend on this wire contract; pin it with tests before Rust starts asserting against it.

## Why this exists

Surfaced by the Phase 1 final whole-branch review. The happy paths and the
`execute`/`ephemeral` round-trip are tested, but three pure, cheap branches
that define the JSON-RPC contract are currently unexercised. They are trivial
to cover and locking them in now prevents a silent contract drift once the
Rust side (tickets 11/12) starts depending on the exact envelopes.

**This is test-only. No production code changes** — if a test reveals a real
bug, stop and raise it rather than editing behaviour under cover of a "test"
ticket.

## Gaps to close

1. **`Kernel.start()` double-start guard** (`kernel.py`) — asserts `RuntimeError`
   on a second `start()`.
2. **`Dispatcher` error envelopes** (`rpc.py`) — parse error (`-32700`),
   invalid request / non-string `method` (`-32600`), method-not-found
   (`-32601`); each returns the JSON-RPC error shape with the request `id`
   echoed (or `null` where no id is recoverable).
3. **`version` method** (`rpc.py`) — returns `{"sidecar": <__version__>, "kernel": ...}`.

## Files

- Modify: `sidecar/tests/test_kernel.py` (add the double-start case).
- Modify: `sidecar/tests/test_rpc.py` (add the error-envelope + `version` cases).

## Acceptance

- `just test-py` green; `just lint-py` green; no suppressions.
- No change to any file under `sidecar/src/`.

## Out of scope

- Any new RPC method or behaviour change — those get their own tickets.
