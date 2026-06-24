# Ticket 12 — Expose `execute` and `ping` as Tauri commands

**Phase:** 2
**Depends on:** [11](./11-control-client.md)
**Blocks:** [21](./21-chat-pane.md), [22](./22-claude-client-stub.md)

## Goal

Mount the `SidecarClient` as Tauri-managed state, register two Tauri commands (`ping`, `execute`), and prove with a Vitest-style svelte-check (or a hand smoke check) that `invoke('execute', { code, ephemeral })` returns the structured reply.

## Files

- Modify: `src-tauri/src/lib.rs` — register the commands, attach managed state.
- Create: `src-tauri/src/commands.rs`.
- Modify: `src-tauri/capabilities/default.json` — permit `ping` + `execute`.
- Modify: `src/routes/+page.svelte` — replace the "Greet" demo with a button calling `execute`.

## Steps

- [ ] **1. Failing UI smoke.** Replace the body of `+page.svelte` with a minimal call to `execute('1+1')` and render the result. Run `just dev`. Expect "command not found" — that's the failure we are fixing.

- [ ] **2. Implement the commands.**

  ```rust
  // src-tauri/src/commands.rs
  use serde::Serialize;
  use crate::sidecar_client::{ExecuteResponse, SidecarClient};
  use tauri::State;

  #[derive(Debug, Serialize)]
  pub struct ExecuteCommandError { pub code: i64, pub message: String }

  #[tauri::command]
  pub async fn ping(client: State<'_, SidecarClient>) -> Result<String, ExecuteCommandError> {
      client.ping().await.map_err(|e| ExecuteCommandError { code: e.code, message: e.message })
  }

  #[tauri::command]
  pub async fn execute(
      code: String,
      ephemeral: bool,
      client: State<'_, SidecarClient>,
  ) -> Result<ExecuteResponse, ExecuteCommandError> {
      client
          .execute(&code, ephemeral)
          .await
          .map_err(|e| ExecuteCommandError { code: e.code, message: e.message })
  }
  ```

- [ ] **3. Wire into `lib.rs`.**

  ```rust
  pub mod commands;
  pub mod sidecar;
  pub mod sidecar_client;

  #[cfg_attr(mobile, tauri::mobile_entry_point)]
  pub fn run() {
      tauri::Builder::default()
          .setup(|app| {
              let handle = app.handle().clone();
              tauri::async_runtime::spawn(async move {
                  let repo_root = std::env::current_dir().expect("cwd");
                  let mut sidecar = sidecar::Sidecar::spawn(&repo_root).await.expect("spawn sidecar");
                  let client = sidecar_client::SidecarClient::attach(&mut sidecar);
                  handle.manage(client);
                  handle.manage(sidecar);
              });
              Ok(())
          })
          .plugin(tauri_plugin_opener::init())
          .invoke_handler(tauri::generate_handler![commands::ping, commands::execute])
          .run(tauri::generate_context!())
          .expect("error while running tauri application");
  }
  ```

- [ ] **4. Update `+page.svelte`.**

  ```svelte
  <script lang="ts">
    import { invoke } from '@tauri-apps/api/core';

    type ExecuteResponse = {
      readonly status: 'ok' | 'error';
      readonly stdout: string;
      readonly value: string | null;
      readonly traceback: string | null;
      readonly ephemeral: boolean;
    };

    let code = $state('1 + 1');
    let reply = $state<ExecuteResponse | null>(null);
    let error = $state<string | null>(null);

    async function run(): Promise<void> {
      error = null;
      try {
        reply = await invoke<ExecuteResponse>('execute', { code, ephemeral: false });
      } catch (err) {
        error = err instanceof Error ? err.message : String(err);
      }
    }
  </script>

  <main>
    <textarea bind:value={code} rows="4"></textarea>
    <button type="button" onclick={run}>execute</button>
    {#if error}<pre class="error">{error}</pre>{/if}
    {#if reply}<pre>{JSON.stringify(reply, null, 2)}</pre>{/if}
  </main>
  ```

- [ ] **5. Smoke test.**

  ```sh
  just dev
  ```

  Click `execute`. Expect `{ "value": "2", ... }`.

- [ ] **6. Lint + commit.**

  ```sh
  just lint
  git commit -m "feat(core): Tauri commands ping/execute backed by the sidecar"
  ```

## Acceptance

- Click → response in the webview.
- `just lint` is green for **all** of rust, py, ts.
- No `any` in the TypeScript handler; no `unwrap()` in the Rust commands.

## Out of scope

- A real chat surface — Ticket 21.
- Streaming stdout — fast-follow.
