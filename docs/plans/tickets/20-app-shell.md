# Ticket 20 — App shell: three-pane layout

**Phase:** 3 (frontend skeleton + chat)
**Depends on:** [13](./13-state-events.md)
**Blocks:** [21](./21-chat-pane.md), [30](./30-propose-experiment-schema.md), [40](./40-arrow-server.md)

## Goal

Replace the Tauri starter `+page.svelte` with the real Kiln shell: three resizable panes (Chat / CodeView / Results) plus the sidecar-status pill from Ticket 13. The panes are stubs (each shows its name) — the real content lands in later tickets.

## Why this is a small standalone ticket

Layout decisions ripple through every later UI ticket. Locking the three-pane shape now means the dataframe explorer, plot viewer, run comparison, and REPL each know exactly which slot they fit into.

## Files

- Create: `src/lib/components/AppShell.svelte`.
- Create: `src/lib/components/ChatPane.svelte` (stub).
- Create: `src/lib/components/CodeViewPane.svelte` (stub).
- Create: `src/lib/components/ResultsPane.svelte` (stub).
- Modify: `src/routes/+page.svelte` to render `<AppShell />`.
- Modify: `src/routes/+layout.svelte` (create if absent) — global styles.

## Steps

- [ ] **1. Failing visual check.** `just dev` — confirm the starter UI is still there.

- [ ] **2. Create the four components.**

  ```svelte
  <!-- src/lib/components/AppShell.svelte -->
  <script lang="ts">
    import ChatPane from './ChatPane.svelte';
    import CodeViewPane from './CodeViewPane.svelte';
    import ResultsPane from './ResultsPane.svelte';
    import { createSidecarStatus } from '$lib/sidecar-status.svelte';

    const status = createSidecarStatus();
  </script>

  <div class="shell" data-status={status.value}>
    <header><span class="pill">sidecar: {status.value}</span></header>
    <main class="panes">
      <section class="pane chat"><ChatPane /></section>
      <section class="pane code"><CodeViewPane /></section>
      <section class="pane results"><ResultsPane /></section>
    </main>
  </div>

  <style>
    .shell { display: grid; grid-template-rows: 32px 1fr; height: 100vh; }
    .panes { display: grid; grid-template-columns: 1fr 1.2fr 1.4fr; gap: 1px; background: #2a2a2a; }
    .pane { background: #1e1e1e; color: #e6e6e6; padding: 12px; overflow: auto; }
    header { display: flex; align-items: center; padding: 0 12px; background: #111; color: #ccc; }
    .pill { font-size: 12px; padding: 2px 8px; border-radius: 999px; background: #2a2a2a; }
  </style>
  ```

  The three stub panes render only `<h2>Chat</h2>` / etc — placeholder text included to make resize behaviour visible.

- [ ] **3. Update `+page.svelte`.**

  ```svelte
  <script lang="ts">
    import AppShell from '$lib/components/AppShell.svelte';
  </script>
  <AppShell />
  ```

- [ ] **4. Smoke test.**

  ```sh
  just dev
  ```

  Confirm the three panes are visible and the status pill flips to "ready".

- [ ] **5. svelte-check + eslint + commit.**

  ```sh
  just lint
  git commit -m "feat(ui): three-pane Kiln shell with sidecar status pill"
  ```

## Acceptance

- Pane layout matches the spec's surfaces priority (Chat / CodeView / Results).
- No `any` in any component.
- `just lint` is green.

## Out of scope

- Resize handles — fast-follow once a real panel cries out for it.
- Theming / design system — Ticket placeholder; see [`../../CLAUDE.md`](../../CLAUDE.md) for invoking the `frontend-design` skill when the visual pass happens.
