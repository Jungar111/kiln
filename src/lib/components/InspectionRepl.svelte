<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  // Mirrors `ExecuteResponse` in `src-tauri/src/sidecar_client.rs`. Every line
  // typed here runs with `ephemeral: true` so the poke never lands in the run.
  type ExecuteResponse = {
    readonly status: 'ok' | 'error';
    readonly stdout: string;
    readonly value: string | null;
    readonly traceback: string | null;
    readonly ephemeral: boolean;
  };

  type Entry = { readonly id: number; readonly code: string; readonly response: ExecuteResponse };

  // State lives on the component instance, so unmounting the gate (decline /
  // kill / restart) discards the session — exactly the spec's "ephemeral" intent.
  let history = $state<Entry[]>([]);
  let draft = $state('');
  let nextId = 0;
  let running = $state(false);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const code = draft.trim();
    if (code === '' || running) return;
    draft = '';
    running = true;
    try {
      const response = await invoke<ExecuteResponse>('execute', { code, ephemeral: true });
      history = [...history, { id: nextId++, code, response }];
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      history = [
        ...history,
        {
          id: nextId++,
          code,
          response: {
            status: 'error',
            stdout: '',
            value: null,
            traceback: message,
            ephemeral: true,
          },
        },
      ];
    } finally {
      running = false;
    }
  }
</script>

<details class="inspect">
  <summary>inspect kernel state</summary>
  <ol>
    {#each history as entry (entry.id)}
      <li>
        <pre class="in">› {entry.code}</pre>
        {#if entry.response.stdout}<pre>{entry.response.stdout}</pre>{/if}
        {#if entry.response.value !== null}<pre class="value">{entry.response.value}</pre>{/if}
        {#if entry.response.traceback}<pre class="error">{entry.response.traceback}</pre>{/if}
      </li>
    {/each}
  </ol>
  <form onsubmit={submit}>
    <input bind:value={draft} placeholder="ephemeral cell" disabled={running} />
    <button type="submit" disabled={running}>run</button>
  </form>
  <p class="hint">
    Calls here run with <code>ephemeral=true</code> — they will not appear in the logged run.
  </p>
</details>

<style>
  .inspect {
    border: 1px solid #333;
    border-radius: 8px;
    padding: 8px 12px;
    background: #141414;
  }
  summary {
    cursor: pointer;
    color: #bbb;
    font-size: 13px;
  }
  ol {
    list-style: none;
    margin: 8px 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 240px;
    overflow: auto;
  }
  li {
    border-left: 2px solid #2a2a2a;
    padding-left: 8px;
  }
  pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 12px;
  }
  .in {
    color: #9ad;
  }
  .value {
    color: #ddd;
  }
  .error {
    color: #e0a4a4;
  }
  form {
    display: flex;
    gap: 6px;
  }
  input {
    flex: 1;
    background: #1a1a1a;
    color: #e6e6e6;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 6px 8px;
    font: inherit;
  }
  input:disabled {
    opacity: 0.6;
  }
  button {
    border: none;
    border-radius: 6px;
    padding: 6px 14px;
    cursor: pointer;
    font: inherit;
    background: #2a2a2a;
    color: #ddd;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
  .hint {
    margin: 8px 0 0;
    font-size: 11px;
    color: #888;
  }
  .hint code {
    color: #9ad;
  }
</style>
