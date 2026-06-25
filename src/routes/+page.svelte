<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createSidecarStatus } from '$lib/sidecar-status.svelte';

  type ExecuteResponse = {
    readonly status: 'ok' | 'error';
    readonly stdout: string;
    readonly value: string | null;
    readonly traceback: string | null;
    readonly ephemeral: boolean;
  };

  const status = createSidecarStatus();

  let code = $state('1 + 1');
  let reply = $state<ExecuteResponse | null>(null);
  let error = $state<string | null>(null);

  async function run(): Promise<void> {
    error = null;
    try {
      reply = await invoke<ExecuteResponse>('execute', { code, ephemeral: false });
    } catch (err) {
      // Surface the typed error message (including -32099 "sidecar exited" errors).
      error = err instanceof Error ? err.message : String(err);
    }
  }
</script>

<aside class="status pill status-{status.value}">sidecar: {status.value}</aside>

<main>
  <textarea bind:value={code} rows="4"></textarea>
  <button type="button" onclick={run}>execute</button>
  {#if error}<pre class="error">{error}</pre>{/if}
  {#if reply}<pre>{JSON.stringify(reply, null, 2)}</pre>{/if}
</main>
