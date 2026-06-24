# Ticket 13 — Sidecar lifecycle events + visible status

**Phase:** 2
**Depends on:** [12](./12-tauri-execute-command.md)
**Blocks:** [20](./20-app-shell.md)

## Goal

Surface sidecar liveness to the webview. The Rust core fires Tauri events (`sidecar:starting`, `sidecar:ready`, `sidecar:exited`) and the frontend renders a small status pill. When the sidecar dies, the next `execute` returns a typed error instead of hanging.

## Files

- Modify: `src-tauri/src/sidecar.rs` — add a `wait()` task that emits exit.
- Modify: `src-tauri/src/lib.rs` — emit on spawn / ready / exit.
- Modify: `src-tauri/src/sidecar_client.rs` — close pending oneshots when stdout ends.
- Create: `src/lib/sidecar-status.svelte.ts` + a small pill component in `src/routes/+page.svelte`.

## Steps

- [ ] **1. Failing test (Rust).** Add a unit test that spawns, sends `ping`, kills the process, then asserts the next `execute` resolves to `Err(RpcError { code: -32099, .. })` within a 1s timeout.

- [ ] **2. Implement the cleanup loop.** When the stdout reader sees EOF, drain `pending` and send `Err(RpcError { code: -32099, message: "sidecar exited" })` to every waiter.

- [ ] **3. Add events in `lib.rs`.**

  ```rust
  app.emit("sidecar:starting", ()).ok();
  // after attach:
  app.emit("sidecar:ready", ()).ok();
  // in the child's wait task:
  app.emit("sidecar:exited", &status_code).ok();
  ```

- [ ] **4. Frontend pill.**

  ```ts
  // src/lib/sidecar-status.svelte.ts
  import { listen } from '@tauri-apps/api/event';

  export type SidecarStatus = 'starting' | 'ready' | 'exited';

  export function createSidecarStatus(): {
    get value(): SidecarStatus;
  } {
    let value: SidecarStatus = $state('starting');
    void listen<void>('sidecar:ready', () => { value = 'ready'; });
    void listen<number>('sidecar:exited', () => { value = 'exited'; });
    return { get value() { return value; } };
  }
  ```

  Render in `+page.svelte`:

  ```svelte
  <script lang="ts">
    import { createSidecarStatus } from '$lib/sidecar-status.svelte';
    const status = createSidecarStatus();
  </script>
  <aside class="status pill status-{status.value}">sidecar: {status.value}</aside>
  ```

- [ ] **5. Manual smoke test.**

  - `just dev`. Verify pill shows "ready".
  - From a terminal: `pkill -f kiln-sidecar`. Pill flips to "exited".
  - Click `execute`. Expect an error toast (not a hang).

- [ ] **6. Lint + commit.**

## Acceptance

- The Rust test (step 1) green.
- Manual smoke check passes.
- A killed sidecar resolves pending RPC calls with errors instead of leaving them pending.

## Out of scope

- Auto-restart of the sidecar — fast-follow.
- Distinguishing crash vs deliberate shutdown — Ticket 71's domain.
