<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ChatPane from './ChatPane.svelte';
  import CodeViewPane from './CodeViewPane.svelte';
  import PremiseGate from './PremiseGate.svelte';
  import ResultsGate from './ResultsGate.svelte';
  import CompareView from './CompareView.svelte';
  import DataFrameView from './DataFrameView.svelte';
  import PlotPanel from './PlotPanel.svelte';
  import { createSidecarStatus } from '$lib/sidecar-status.svelte';
  import { createCheckpointStore } from '$lib/checkpoint-store.svelte';
  import { createRunsStore, type Run } from '$lib/runs-store.svelte';
  import { dfStore } from '$lib/df-store.svelte';
  import { plotStore } from '$lib/plot-store.svelte';
  import { chat } from '$lib/chat-store.svelte';
  import { toMessage } from '$lib/errors';
  import type { ProposeExperiment, Verdict } from '$lib/checkpoint-types';

  // ponytail: placeholder workspace label — wire to a project store when one exists.
  const PROJECT = 'workspace';

  const status = createSidecarStatus();
  const ckpt = createCheckpointStore();
  const runsStore = createRunsStore();

  type Tab = 'checkpoint' | 'code' | 'dataframe' | 'plots' | 'compare';
  const TABS: readonly { id: Tab; label: string; context: string }[] = [
    { id: 'checkpoint', label: 'Checkpoint', context: 'propose_experiment' },
    { id: 'code', label: 'Code', context: 'experiment source' },
    { id: 'dataframe', label: 'DataFrame', context: 'Arrow IPC · zero-copy' },
    { id: 'plots', label: 'Plots', context: 'display_data · image/png' },
    { id: 'compare', label: 'Compare', context: 'search_runs() · inline diff' },
  ];
  let tab = $state<Tab>('checkpoint');
  const activeContext = $derived(TABS.find((t) => t.id === tab)?.context ?? '');

  // The run awaiting a keep/kill/iterate verdict. Set on approve; cleared once a
  // verdict is recorded.
  // ponytail: the results gate currently appears right after approval. Firing it
  // on autolog completion waits on the experiment-execution loop (later phase).
  let activeRunId = $state<string | null>(null);

  const selectedRuns = $derived(runsStore.runs.filter((run) => runsStore.selected.has(run.run_id)));

  // Kernel status mirrors the sidecar lifecycle — the heavy review point needs to
  // know the kernel is live before you poke it.
  const kernel = $derived.by(() => {
    switch (status.value) {
      case 'ready':
        return { dot: 'var(--good)', text: 'kernel idle · 1 venv' };
      case 'starting':
        return { dot: 'var(--warn)', text: 'kernel booting' };
      case 'exited':
        return { dot: 'var(--bad)', text: 'kernel exited' };
    }
  });

  function primaryMetric(run: Run): string {
    const order = ['pr_auc', 'roc_auc', 'accuracy'];
    const key = order.find((k) => k in run.metrics) ?? Object.keys(run.metrics)[0];
    const value = key === undefined ? undefined : run.metrics[key];
    if (value === undefined) return '—';
    // ".58" mono style from the design (drop the leading zero on fractions).
    return value.toFixed(2).replace(/^0\./, '.');
  }

  function toggleRun(runId: string): void {
    if (runsStore.selected.has(runId)) runsStore.selected.delete(runId);
    else runsStore.selected.add(runId);
  }

  async function loadRuns(): Promise<void> {
    try {
      await runsStore.refresh();
    } catch (err) {
      chat.note(`⚠️ list_runs failed: ${toMessage(err)}`);
    }
  }

  async function approve(proposal: ProposeExperiment): Promise<void> {
    try {
      const { run_id } = await invoke<{ run_id: string }>('approve_checkpoint', { proposal });
      chat.note(`Run started: ${run_id}`);
      activeRunId = run_id;
      await loadRuns();
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
      await loadRuns();
    } catch (err) {
      chat.note(`⚠️ close_run failed: ${toMessage(err)}`);
    } finally {
      activeRunId = null;
    }
  }

  // Bring the right surface forward the moment a gate fires — the review must be
  // seen. Preserves the original modal's "you must act on this" intent in the tab
  // model without trapping the user there afterwards.
  $effect(() => {
    if (ckpt.pending) tab = 'checkpoint';
  });
  $effect(() => {
    if (activeRunId !== null) tab = 'compare';
  });

  void loadRuns();
</script>

<div class="window">
  <!-- ===== titlebar ===== -->
  <div class="titlebar">
    <div class="lights">
      <span style="background:#ff5f57"></span>
      <span style="background:#febc2e"></span>
      <span style="background:#28c840"></span>
    </div>
    <div class="title">{PROJECT} — <span class="brand">kiln</span></div>
    <div class="env">py 3.12 · uv · mlflow</div>
  </div>

  <div class="body">
    <!-- ===== sidebar ===== -->
    <aside class="sidebar">
      <div class="proj">
        <div class="proj-head">
          <span class="logo"></span>
          <span class="proj-name">{PROJECT}</span>
        </div>
        <div class="kernel">
          <span class="dot" style="background:{kernel.dot};box-shadow:0 0 6px {kernel.dot}"></span>
          {kernel.text}
        </div>
      </div>

      <div class="sec-label">Investigations</div>
      <div class="invs">
        {#if ckpt.pending}
          <div class="inv active">
            <div class="inv-name">{ckpt.pending.title}</div>
            <div class="inv-state warn">● awaiting premise review</div>
          </div>
        {:else if activeRunId !== null}
          <div class="inv active good-edge">
            <div class="inv-name">Active investigation</div>
            <div class="inv-state good">● results gate</div>
          </div>
        {:else}
          <div class="inv idle">No active investigation</div>
        {/if}
      </div>

      <div class="sec-label runs-label">
        Runs{#if selectedRuns.length > 0}<span class="sel">
            · {selectedRuns.length} selected</span
          >{/if}
      </div>
      <div class="runs">
        {#each runsStore.runs.slice(0, 12) as run (run.run_id)}
          <button
            type="button"
            class="run"
            class:on={runsStore.selected.has(run.run_id)}
            onclick={() => {
              toggleRun(run.run_id);
            }}
          >
            <span class="run-name">{run.name || run.run_id.slice(0, 8)}</span>
            <span class="run-val">{primaryMetric(run)}</span>
          </button>
        {:else}
          <div class="run-empty">no runs yet</div>
        {/each}
        {#if runsStore.runs.length > 0}
          <div class="run-foot">{runsStore.runs.length} runs · sqlite</div>
        {/if}
      </div>

      <div class="side-foot">
        <span>mlruns · sqlite</span>
        <span class="ok">●</span>
      </div>
    </aside>

    <!-- ===== center ===== -->
    <main class="center">
      <nav class="tabbar">
        {#each TABS as t (t.id)}
          <button
            type="button"
            class="tab"
            class:active={tab === t.id}
            onclick={() => {
              tab = t.id;
            }}
          >
            {#if tab === t.id}<span class="tab-dot"></span>{/if}{t.label}
          </button>
        {/each}
        <span class="tab-context">{activeContext}</span>
      </nav>

      <div class="tab-body">
        {#if tab === 'checkpoint'}
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
          {:else}
            <div class="empty">
              <div class="empty-eyebrow">Premise gate · pre-compute</div>
              <p>No premise awaiting review. Claude proposes the next investigation in chat.</p>
            </div>
          {/if}
        {:else if tab === 'code'}
          <CodeViewPane />
        {:else if tab === 'dataframe'}
          {#if dfStore.current}
            <DataFrameView handle={dfStore.current.handle} rows={dfStore.current.rows} />
          {:else}
            <div class="empty">
              <div class="empty-eyebrow">DataFrame · Arrow IPC</div>
              <p>Run a cell that returns a DataFrame to explore it here — zero-copy.</p>
            </div>
          {/if}
        {:else if tab === 'plots'}
          {#if plotStore.displays.length > 0}
            <PlotPanel displays={plotStore.displays} />
          {:else}
            <div class="empty">
              <div class="empty-eyebrow">Plots · display_data</div>
              <p>Run a cell that draws a plot to see it intercepted here.</p>
            </div>
          {/if}
        {:else}
          <!-- compare -->
          {#if selectedRuns.length > 0}
            <div class="compare-scroll">
              <div class="gate-head">
                <div class="eyebrow">Results gate · post-compute</div>
                <h1 class="serif">Did the agreed investigation answer the question?</h1>
                <p class="sub">
                  The premise decisions you certified are diffed as first-class rows — MLflow stores
                  the ingredients; the comparison is ours.
                </p>
              </div>
              <CompareView runs={selectedRuns} />
            </div>
            {#if activeRunId !== null}
              <ResultsGate
                runId={activeRunId}
                onpick={(verdict: Verdict) => {
                  void recordVerdict(verdict);
                }}
              />
            {/if}
          {:else}
            <div class="empty">
              <div class="empty-eyebrow">Results gate · post-compute</div>
              <p>Select runs in the sidebar to diff their premise decisions and metrics.</p>
            </div>
          {/if}
        {/if}
      </div>

      <!-- repl strip -->
      <div class="repl">
        <span class="dot" style="background:{kernel.dot}"></span>
        <span class="repl-state">{status.value === 'ready' ? 'idle' : status.value}</span>
        <span class="repl-sep">ns:</span>
        {#if dfStore.current}<span class="repl-var">df</span>{/if}
        <span class="repl-prompt">In [·]:</span><span class="kcursor"></span>
        <span class="repl-hint">⌘↵ run · pokes are ephemeral</span>
      </div>
    </main>

    <!-- ===== chat ===== -->
    <ChatPane status={status.value} />
  </div>
</div>

<style>
  .window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    color: var(--tx);
    font-family: var(--font-sans);
    overflow: hidden;
  }

  /* titlebar */
  .titlebar {
    height: 38px;
    flex-shrink: 0;
    background: var(--bg-bar);
    border-bottom: 1px solid var(--bd-2);
    display: flex;
    align-items: center;
    padding: 0 14px;
    position: relative;
  }
  .lights {
    display: flex;
    gap: 8px;
  }
  .lights span {
    width: 12px;
    height: 12px;
    border-radius: 50%;
  }
  .title {
    position: absolute;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 12px;
    color: var(--tx-dim2);
    font-weight: 500;
    pointer-events: none;
  }
  .brand {
    font-family: var(--font-serif);
    font-style: italic;
    color: var(--ember-serif);
  }
  .env {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--tx-mut2);
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  /* sidebar */
  .sidebar {
    width: 212px;
    flex-shrink: 0;
    background: var(--bg-side);
    border-right: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
  }
  .proj {
    padding: 14px 14px 10px;
  }
  .proj-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .logo {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    background: var(--logo-grad);
    box-shadow: 0 0 10px -2px var(--ember);
  }
  .proj-name {
    font-weight: 600;
    color: var(--tx-bright);
    font-size: 13px;
  }
  .kernel {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--tx-mut);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .sec-label {
    padding: 6px 14px 4px;
    font-family: var(--font-mono);
    font-size: 9.5px;
    letter-spacing: 1.4px;
    text-transform: uppercase;
    color: var(--tx-mut3);
  }
  .runs-label {
    padding-top: 14px;
  }
  .sel {
    color: var(--ember-soft);
  }
  .invs {
    padding: 0 8px;
  }
  .inv {
    padding: 8px;
    border-radius: 6px;
  }
  .inv.active {
    background: var(--bg-sel);
    border-left: 2px solid var(--warn);
  }
  .inv.good-edge {
    border-left-color: var(--good);
  }
  .inv-name {
    color: var(--tx-bright);
    font-weight: 500;
    font-size: 12px;
  }
  .inv-state {
    font-family: var(--font-mono);
    font-size: 10px;
    margin-top: 2px;
  }
  .inv-state.warn {
    color: var(--warn);
  }
  .inv-state.good {
    color: var(--good);
  }
  .inv.idle {
    color: var(--tx-mut);
    font-size: 12px;
  }
  .runs {
    padding: 0 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    display: flex;
    flex-direction: column;
  }
  .run {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 5px 6px;
    border-radius: 5px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--tx);
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .run:hover {
    background: var(--bg-row-alt);
  }
  .run.on {
    background: var(--bg-sel);
    border-color: var(--bd-ember);
    color: var(--tx-bright);
  }
  .run-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .run-val {
    color: var(--tx-bright);
    flex-shrink: 0;
    padding-left: 8px;
  }
  .run-empty,
  .run-foot {
    padding: 4px 6px;
    font-size: 10px;
    color: var(--tx-mut3);
  }
  .side-foot {
    margin-top: auto;
    padding: 12px 14px;
    border-top: 1px solid var(--bd);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--tx-mut2);
    display: flex;
    justify-content: space-between;
  }
  .ok {
    color: var(--good);
  }

  /* center */
  .center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .tabbar {
    height: 40px;
    flex-shrink: 0;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: stretch;
    padding: 0 4px;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 14px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    color: var(--tx-dim2);
    font: inherit;
    cursor: pointer;
  }
  .tab:hover {
    color: var(--tx-2);
  }
  .tab.active {
    color: var(--tx-bright);
    font-weight: 500;
    border-bottom-color: var(--ember);
  }
  .tab-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ember);
    box-shadow: 0 0 7px var(--ember);
  }
  .tab-context {
    margin-left: auto;
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--tx-mut2);
  }
  .tab-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .compare-scroll {
    flex: 1;
    overflow: auto;
    padding: 18px 22px;
  }
  .gate-head .eyebrow {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 1.4px;
    text-transform: uppercase;
    color: var(--tx-mut);
  }
  .serif {
    font-family: var(--font-serif);
    font-size: 23px;
    font-weight: 500;
    color: var(--tx-title);
    margin: 5px 0 0;
    line-height: 1.2;
  }
  .sub {
    margin: 5px 0 0;
    color: var(--tx-dim);
    max-width: 540px;
  }

  .empty {
    margin: auto;
    text-align: center;
    color: var(--tx-mut);
    max-width: 420px;
    padding: 40px;
  }
  .empty-eyebrow {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 1.4px;
    text-transform: uppercase;
    color: var(--tx-mut3);
    margin-bottom: 8px;
  }

  /* repl strip */
  .repl {
    height: 34px;
    flex-shrink: 0;
    border-top: 1px solid var(--bd);
    background: var(--bg-repl);
    display: flex;
    align-items: center;
    padding: 0 14px;
    gap: 10px;
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .repl .dot {
    box-shadow: 0 0 6px currentColor;
  }
  .repl-state {
    color: var(--good);
  }
  .repl-sep {
    color: var(--tx-mut3);
  }
  .repl-var {
    color: var(--tx-dim);
  }
  .repl-prompt {
    color: var(--tx-mut3);
    margin-left: 4px;
  }
  .kcursor {
    display: inline-block;
    width: 7px;
    height: 15px;
    background: var(--ember);
    animation: kblink 1.1s step-end infinite;
  }
  .repl-hint {
    margin-left: auto;
    color: var(--tx-mut3);
  }
</style>
