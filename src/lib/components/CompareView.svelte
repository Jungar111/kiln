<script lang="ts">
  import type { Run } from '$lib/runs-store.svelte';
  import { flagSuspiciousRuns } from '$lib/heuristics';
  import DecisionDiff from './DecisionDiff.svelte';

  let { runs }: { runs: readonly Run[] } = $props();

  const flags = $derived(flagSuspiciousRuns(runs));

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
</script>

<div class="compare">
  {#if flags.length > 0}
    <div class="flags">
      {#each flags as flag (flag.run + flag.slot)}
        <div class="flag">⚠ {flag.note}</div>
      {/each}
    </div>
  {/if}

  <table class="head">
    <thead>
      <tr>
        <th></th>
        {#each runs as run (run.run_id)}<th>{run.name || run.run_id.slice(0, 8)}</th>{/each}
      </tr>
    </thead>
    <tbody>
      <tr
        ><th>status</th>{#each runs as run (run.run_id)}<td>{run.status}</td>{/each}</tr
      >
      <tr
        ><th>outcome</th>{#each runs as run (run.run_id)}<td>{run.outcome ?? '—'}</td>{/each}</tr
      >
    </tbody>
  </table>

  <h3>Declared decisions</h3>
  <DecisionDiff {runs} />

  {#if paramKeys.length > 0}
    <h3>Params</h3>
    <table>
      <tbody>
        {#each paramKeys as key (key)}
          <tr class:diff={differs(runs.map((run) => run.params[key]))}>
            <th>{key}</th>
            {#each runs as run (run.run_id)}<td>{run.params[key] ?? '—'}</td>{/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  {#if metricKeys.length > 0}
    <h3>Metrics</h3>
    <table>
      <tbody>
        {#each metricKeys as key (key)}
          <tr class:diff={differs(runs.map((run) => fmtMetric(run.metrics[key])))}>
            <th>{key}</th>
            {#each runs as run (run.run_id)}<td>{fmtMetric(run.metrics[key])}</td>{/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .compare {
    margin-top: 12px;
    border-top: 1px solid #2a2a2a;
    padding-top: 12px;
  }
  .flags {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 10px;
  }
  .flag {
    background: rgba(224, 86, 63, 0.12);
    border: 1px solid #e0563f;
    border-radius: 6px;
    padding: 4px 10px;
    color: #ffd9cf;
    font-size: 13px;
  }
  h3 {
    margin: 14px 0 4px;
    font-size: 13px;
    color: #9ad;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th {
    text-align: left;
    color: #bbb;
    font-weight: 600;
    padding: 3px 8px;
    white-space: nowrap;
  }
  td {
    padding: 3px 8px;
    color: #e6e6e6;
  }
  .head thead th {
    color: #fff;
    border-bottom: 1px solid #333;
  }
  tr.diff td {
    background: rgba(217, 164, 65, 0.12);
  }
</style>
