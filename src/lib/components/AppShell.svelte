<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ChatPane from './ChatPane.svelte';
  import CodeViewPane from './CodeViewPane.svelte';
  import ResultsPane from './ResultsPane.svelte';
  import PremiseGate from './PremiseGate.svelte';
  import ResultsGate from './ResultsGate.svelte';
  import { createSidecarStatus } from '$lib/sidecar-status.svelte';
  import { createCheckpointStore } from '$lib/checkpoint-store.svelte';
  import { chat } from '$lib/chat-store.svelte';
  import { toMessage } from '$lib/errors';
  import type { ProposeExperiment, Verdict } from '$lib/checkpoint-types';

  const status = createSidecarStatus();
  const ckpt = createCheckpointStore();

  // The run awaiting a keep/kill/iterate verdict. Set on approve; cleared once a
  // verdict is recorded.
  // ponytail: the results gate currently appears right after approval. Firing it
  // on autolog completion waits on the experiment-execution loop (later phase).
  let activeRunId = $state<string | null>(null);

  async function approve(proposal: ProposeExperiment): Promise<void> {
    try {
      const { run_id } = await invoke<{ run_id: string }>('approve_checkpoint', { proposal });
      chat.note(`Run started: ${run_id}`);
      activeRunId = run_id;
    } catch (err) {
      chat.note(`⚠️ Approve failed: ${toMessage(err)}`);
    } finally {
      ckpt.clear();
    }
  }

  async function recordVerdict(verdict: Verdict): Promise<void> {
    const runId = activeRunId;
    if (runId === null) return;
    try {
      await invoke('close_run', { runId, verdict });
      chat.note(`Verdict on ${runId}: ${verdict}`);
    } catch (err) {
      chat.note(`⚠️ close_run failed: ${toMessage(err)}`);
    } finally {
      activeRunId = null;
    }
  }
</script>

<div class="shell" data-status={status.value}>
  <header><span class="pill">sidecar: {status.value}</span></header>
  <main class="panes">
    <section class="pane chat"><ChatPane /></section>
    <section class="pane code"><CodeViewPane /></section>
    <section class="pane results"><ResultsPane /></section>
  </main>
</div>

{#if ckpt.pending}
  <PremiseGate
    proposal={ckpt.pending}
    drift={ckpt.drift}
    onapprove={(proposal: ProposeExperiment) => {
      void approve(proposal);
    }}
    ondecline={() => {
      ckpt.clear();
    }}
  />
{/if}

{#if activeRunId}
  <ResultsGate
    runId={activeRunId}
    onpick={(verdict: Verdict) => {
      void recordVerdict(verdict);
    }}
  />
{/if}

<style>
  .shell {
    display: grid;
    grid-template-rows: 32px 1fr;
    height: 100vh;
  }
  .panes {
    display: grid;
    grid-template-columns: 1fr 1.2fr 1.4fr;
    gap: 1px;
    background: #2a2a2a;
    min-height: 0;
  }
  .pane {
    background: #1e1e1e;
    color: #e6e6e6;
    padding: 12px;
    overflow: auto;
  }
  header {
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: #111;
    color: #ccc;
  }
  .pill {
    font-size: 12px;
    padding: 2px 8px;
    border-radius: 999px;
    background: #2a2a2a;
  }
</style>
