/**
 * Rune-based store that tracks the sidecar process lifecycle by listening for
 * Tauri events emitted by the Rust core (`sidecar:starting`, `sidecar:ready`,
 * `sidecar:exited`).
 *
 * Note: `core:default` bundles `core:event:default`, so no capability change
 * is needed — the existing config already permits `listen`.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type SidecarStatus = 'starting' | 'ready' | 'exited';

export type SidecarStatusStore = {
  readonly value: SidecarStatus;
};

/**
 * Creates a reactive store that mirrors the current sidecar status.
 * Call this once at component initialisation — it registers event listeners
 * that drive a `$state` rune and never need to be torn down for the app
 * lifetime.
 */
export function createSidecarStatus(): SidecarStatusStore {
  let value: SidecarStatus = $state('starting');

  // Read through a function so TypeScript keeps the full `SidecarStatus` union
  // at every call site (a bare `let` initialised to a literal would be narrowed
  // to `'starting'`, making the guard comparison below "always true").
  const current = (): SidecarStatus => value;

  // listen() returns a Promise<UnlistenFn>; we intentionally don't await it
  // here (we're in a sync factory) — the listener is registered before the
  // first microtask tick, which is fast enough relative to Tauri event
  // delivery. The `void` suppresses the floating-promise lint rule.
  void listen<null>('sidecar:ready', () => {
    value = 'ready';
  });

  void listen<null>('sidecar:exited', () => {
    value = 'exited';
  });

  // Seed readiness on init. The `sidecar:ready` event may be emitted by the
  // Rust core BEFORE the listener above is live, in which case the pill would
  // be stuck on 'starting' forever. A `ping` resolves iff the client is already
  // managed (i.e. ready), so we use it to deterministically catch that race.
  // Both orderings are covered: if `ready` already fired, ping resolves and we
  // seed 'ready'; if the client isn't managed yet, ping rejects and the live
  // listener picks up the later `ready` event.
  void (async () => {
    try {
      await invoke('ping');
      // Guard: don't clobber an 'exited' that may have already arrived.
      if (current() === 'starting') value = 'ready';
    } catch {
      // Sidecar not ready/managed yet — the 'sidecar:ready' listener updates us.
    }
  })();

  return {
    get value() {
      return value;
    },
  };
}
