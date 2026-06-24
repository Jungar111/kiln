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
