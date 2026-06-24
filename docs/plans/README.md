# Kiln implementation plan

This directory is the implementation plan for the Kiln MVP described in [`../../spec.md`](../../spec.md).

- **[`roadmap.md`](./roadmap.md)** — the master plan. Phases, dependency graph, glossary, ticket index, risk register.
- **[`tickets/`](./tickets/)** — one file per ticket, self-contained for handoff to a fresh agent.

The intended workflow: pick the lowest-numbered open ticket, follow it step-by-step, commit per the ticket's commit boundary, then take the next one.

Tickets follow [`superpowers:test-driven-development`](https://github.com/anthropic-experimental/claude-superpowers) discipline: failing test first, minimal implementation, commit.

> **Note:** this repository is public. See [`../../CLAUDE.md`](../../CLAUDE.md) for the type-safety and "no secrets" policies that apply to every ticket.
