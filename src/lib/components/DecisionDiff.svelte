<script lang="ts">
  import { SLOT_FIELDS } from '$lib/checkpoint-types';
  import type { Run } from '$lib/runs-store.svelte';

  let { runs }: { runs: readonly Run[] } = $props();

  function differs(values: (string | undefined)[]): boolean {
    return new Set(values).size > 1;
  }

  const lookHere = $derived([...new Set(runs.flatMap((r) => [...r.look_here]))]);
</script>

<div class="decisions">
  {#if lookHere.length > 0}
    <div class="look-here">
      <strong>Look here</strong>
      <ul>
        {#each lookHere as item (item)}<li>{item}</li>{/each}
      </ul>
    </div>
  {/if}
  <table>
    <tbody>
      {#each SLOT_FIELDS as [key, label] (key)}
        <tr class:diff={differs(runs.map((r) => r.decisions[key]))}>
          <th>{label}</th>
          {#each runs as run (run.run_id)}<td>{run.decisions[key] ?? '—'}</td>{/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .look-here {
    border: 1px solid #e0563f;
    background: rgba(224, 86, 63, 0.08);
    border-radius: 8px;
    padding: 6px 10px;
    margin-bottom: 8px;
  }
  .look-here ul {
    margin: 4px 0 0;
    padding-left: 18px;
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
    vertical-align: top;
    padding: 4px 8px;
    white-space: nowrap;
  }
  td {
    padding: 4px 8px;
    color: #e6e6e6;
    vertical-align: top;
  }
  tr.diff td {
    border-left: 3px solid #d9a441;
    background: rgba(217, 164, 65, 0.08);
  }
</style>
