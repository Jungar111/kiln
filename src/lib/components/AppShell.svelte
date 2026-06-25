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
