<script lang="ts">
  import type { Run } from '$lib/runs-store.svelte';
  import { flagSuspiciousRuns } from '$lib/heuristics';
  import { SLOT_FIELDS } from '$lib/checkpoint-types';

  let { runs }: { runs: readonly Run[] } = $props();

  const flags = $derived(flagSuspiciousRuns(runs));
  const flaggedRuns = $derived(new Set(flags.map((f) => f.run)));
  // Only paint clean values green when something else in the set is flagged —
  // a plain difference with nothing suspicious shouldn't read as "the good one".
  const anyFlagged = $derived(flaggedRuns.size > 0);
  const lookHere = $derived([...new Set(runs.flatMap((r) => [...r.look_here]))]);

  // A, B, C… chips, green when the harness left a run un-flagged, red when not.
  function letter(i: number): string {
    return String.fromCharCode(65 + i);
  }

  function unionKeys(pick: (run: Run) => Readonly<Record<string, unknown>>): string[] {
    return [...new Set(runs.flatMap((run) => Object.keys(pick(run))))].sort();
  }
  const paramKeys = $derived(unionKeys((run) => run.params));
  const metricKeys = $derived(unionKeys((run) => run.metrics));

  function differs(values: (string | undefined)[]): boolean {
    return new Set(values).size > 1;
  }
  function fmtMetric(value: number | undefined): string {
    return value === undefined ? '—' : value.toPrecision(4);
  }

  const cols = $derived(`200px repeat(${String(runs.length)}, 1fr)`);
</script>

<div class="cmp">
  {#each flags as flag (flag.run + flag.slot)}
    <div class="flag">
      <span class="flag-ico">⚑</span>
      <div>
        <span class="flag-strong">Harness auto-flag (independent of Claude)</span> — {flag.note}
      </div>
    </div>
  {/each}

  {#if lookHere.length > 0}
    <div class="look">
      <div class="look-title">👁 Look here</div>
      {#each lookHere as item (item)}<div class="look-item">{item}</div>{/each}
    </div>
  {/if}

  <div class="table">
    <!-- header -->
    <div class="grid header" style="grid-template-columns:{cols}">
      <div class="h-label">Decision / metric</div>
      {#each runs as run, i (run.run_id)}
        <div class="h-run">
          <span class="chip" class:bad={flaggedRuns.has(run.run_id)}>{letter(i)}</span>
          <span class="run-name">{run.name || run.run_id.slice(0, 8)}</span>
        </div>
      {/each}
    </div>

    <!-- premise decisions -->
    <div class="band">Premise decisions · certified tags</div>
    {#each SLOT_FIELDS as [key, label] (key)}
      {@const vals = runs.map((r) => r.decisions[key])}
      <div class="grid row" class:diff={differs(vals)} style="grid-template-columns:{cols}">
        <div class="c-label">{label}</div>
        {#each runs as run (run.run_id)}
          <div
            class="c-val mono"
            class:flag-val={differs(vals) && flaggedRuns.has(run.run_id)}
            class:good-val={differs(vals) && anyFlagged && !flaggedRuns.has(run.run_id)}
          >
            {run.decisions[key] ?? '—'}
          </div>
        {/each}
      </div>
    {/each}

    {#if paramKeys.length > 0}
      <div class="band">Params</div>
      {#each paramKeys as key (key)}
        {@const vals = runs.map((r) => r.params[key])}
        <div class="grid row" class:diff={differs(vals)} style="grid-template-columns:{cols}">
          <div class="c-label">{key}</div>
          {#each runs as run (run.run_id)}<div class="c-val mono">
              {run.params[key] ?? '—'}
            </div>{/each}
        </div>
      {/each}
    {/if}

    {#if metricKeys.length > 0}
      <div class="band">Metrics · held-out</div>
      {#each metricKeys as key (key)}
        {@const vals = runs.map((r) => fmtMetric(r.metrics[key]))}
        <div class="grid row" class:diff={differs(vals)} style="grid-template-columns:{cols}">
          <div class="c-label">{key}</div>
          {#each runs as run (run.run_id)}
            <div
              class="c-val mono"
              class:flag-val={differs(vals) && flaggedRuns.has(run.run_id)}
              class:good-val={differs(vals) && anyFlagged && !flaggedRuns.has(run.run_id)}
            >
              {fmtMetric(run.metrics[key])}
            </div>
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .cmp {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 14px;
  }
  .flag {
    display: flex;
    gap: 11px;
    padding: 11px 14px;
    background: var(--bg-flag);
    border: 1px solid var(--bd-bad);
    border-radius: 8px;
    font-size: 12px;
    color: var(--tx-2);
  }
  .flag-ico {
    color: var(--bad);
    font-size: 14px;
  }
  .flag-strong {
    color: var(--bad-bright);
    font-weight: 600;
  }
  .look {
    background: var(--bg-look);
    border: 1px solid var(--bd-ember);
    border-radius: 8px;
    padding: 10px 12px;
  }
  .look-title {
    color: var(--ember-soft);
    font-weight: 600;
    font-size: 12px;
  }
  .look-item {
    color: var(--tx-2);
    margin-top: 3px;
  }

  .table {
    border: 1px solid var(--bd-soft);
    border-radius: 9px;
    overflow: hidden;
    font-size: 12px;
  }
  .grid {
    display: grid;
  }
  .header {
    background: var(--bg-panel-2);
    border-bottom: 1px solid var(--bd-2);
  }
  .h-label {
    padding: 9px 14px;
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--tx-mut);
  }
  .h-run {
    padding: 9px 14px;
    border-left: 1px solid var(--bd);
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .chip {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    background: var(--bg-good-soft);
    border: 1px solid var(--bd-good);
    color: var(--good);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }
  .chip.bad {
    background: #261a16;
    border-color: #4a3022;
    color: var(--bad);
  }
  .run-name {
    font-family: var(--font-mono);
    color: var(--tx-bright);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .band {
    padding: 5px 14px;
    background: #141311;
    border-top: 1px solid var(--bd);
    border-bottom: 1px solid var(--bd);
    font-family: var(--font-mono);
    font-size: 9.5px;
    letter-spacing: 1.2px;
    text-transform: uppercase;
    color: var(--tx-mut3);
  }
  .row {
    border-bottom: 1px solid var(--bd-row);
  }
  .row:last-child {
    border-bottom: none;
  }
  .row.diff {
    background: #1d1712;
  }
  .c-label {
    padding: 8px 14px;
    color: var(--tx-dim);
  }
  .row.diff .c-label {
    color: var(--tx-2);
  }
  .c-val {
    padding: 8px 14px;
    border-left: 1px solid var(--bd);
    color: var(--tx-dim2);
  }
  .mono {
    font-family: var(--font-mono);
  }
  .flag-val {
    color: var(--bad);
  }
  .good-val {
    color: var(--good);
  }
</style>
