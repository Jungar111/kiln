<!--
  PlotPanel — renders the highest-fidelity rich display the kernel emitted.

  HOW TO MOUNT (integrator): the Results pane is being restructured into tabs on
  `main`. Wire the Plot tab to this component and feed it the plot store:

      import PlotPanel from '$lib/components/PlotPanel.svelte';
      import { createPlotStore } from '$lib/plot-store.svelte';
      const plots = createPlotStore();
      // after each execute: plots.setDisplays(response.displays);
      <PlotPanel displays={plots.displays} />

  Or pass an ExecuteResponse's `displays` array directly:
      <PlotPanel displays={result.displays} />

  Priority order: text/html (interactive plotly) → image/svg+xml → image/png →
  text/plain fallback. HTML renders in a sandboxed iframe (scripts allowed, but
  same-origin denied) so plotly's JS runs without reaching the parent page.
-->
<script lang="ts">
  import type { Display } from '$lib/plot-store.svelte';

  let { displays }: { displays: readonly Display[] } = $props();

  // Highest fidelity first. Anything not in this list falls through to the
  // collapsed text branch below (it is shown verbatim).
  const priority = ['text/html', 'image/svg+xml', 'image/png', 'text/plain'] as const;

  function pick(candidates: readonly Display[]): Display | null {
    for (const mime of priority) {
      const match = candidates.find((d) => d.mime === mime);
      if (match) return match;
    }
    // No known-good MIME: render the first display we have, if any, as text.
    return candidates[0] ?? null;
  }

  const chosen = $derived(pick(displays));

  // SVG is rendered as an image via a data URI rather than `{@html}`: it avoids
  // the no-at-html-tags lint rule AND is safer — an <img>-sourced SVG cannot run
  // inline scripts against the parent document. `encodeURIComponent` keeps the
  // payload URL-safe without a base64 round-trip.
  const svgSrc = $derived(
    chosen?.mime === 'image/svg+xml'
      ? `data:image/svg+xml,${encodeURIComponent(chosen.payload)}`
      : null,
  );
</script>

<div class="plot-panel">
  {#if chosen === null}
    <p class="empty">No plot for the last cell.</p>
  {:else if chosen.mime === 'image/png'}
    <img class="raster" src={`data:image/png;base64,${chosen.payload}`} alt="plot" />
  {:else if svgSrc !== null}
    <img class="raster" src={svgSrc} alt="plot" />
  {:else if chosen.mime === 'text/html'}
    <!-- sandbox without allow-same-origin: plotly's scripts run isolated and
         cannot reach window.top / the parent document. -->
    <iframe class="html" sandbox="allow-scripts" srcdoc={chosen.payload} title="plot"></iframe>
  {:else}
    <pre class="text">{chosen.payload}</pre>
  {/if}
</div>

<style>
  .plot-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: #0f0f0f;
    border-radius: 8px;
    overflow: hidden;
  }
  .raster {
    /* Reserve the box so the PNG/SVG does not shift layout as it decodes. */
    display: block;
    max-width: 100%;
    max-height: 100%;
    margin: auto;
    object-fit: contain;
  }
  .html {
    flex: 1;
    width: 100%;
    border: none;
    background: #fff;
  }
  .text {
    margin: 0;
    padding: 12px;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 12px;
    color: #ddd;
    overflow: auto;
  }
  .empty {
    margin: auto;
    color: #777;
    font-size: 13px;
  }
</style>
