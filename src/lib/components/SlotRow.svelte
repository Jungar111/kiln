<script lang="ts">
  import type { Slot } from '$lib/checkpoint-types';

  let { label, slot, drifted = false }: { label: string; slot: Slot; drifted?: boolean } = $props();

  // Critical/notable expanded by default; fyi collapsed (layered disclosure).
  // A drifted slot is always expanded — it is the reason the gate re-fired.
  const open = $derived(drifted || slot.severity !== 'fyi');
</script>

<details class="slot sev-{slot.severity}" class:drifted {open}>
  <summary>
    <span class="bullet"></span>
    <span class="name">{label}</span>
    <span class="badge">{slot.severity}</span>
    {#if drifted}<span class="badge drift">⚑ drift</span>{/if}
    {#if !slot.in_scope}<span class="oos">out of scope</span>{/if}
  </summary>
  <p class="answer">{slot.answer}</p>
</details>

<style>
  .slot {
    display: block;
    padding: 12px 13px;
    background: var(--bg-panel-2);
    border: 1px solid var(--bd-soft);
    border-radius: 7px;
  }
  summary {
    list-style: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  .bullet {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .name {
    color: var(--tx-bright);
    font-weight: 600;
    font-size: 12.5px;
  }
  .badge {
    font-family: var(--font-mono);
    font-size: 9px;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid;
  }
  .oos {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--tx-mut2);
  }
  .answer {
    margin: 6px 0 0 17px;
    color: var(--tx-dim);
    white-space: pre-wrap;
  }

  /* critical */
  .sev-critical {
    background: #1d1712;
    border-color: #3a2a1a;
  }
  .sev-critical .bullet {
    background: var(--bad);
    box-shadow: 0 0 8px -1px var(--bad);
  }
  .sev-critical .badge {
    color: var(--bad);
    border-color: #5a2a22;
  }

  /* notable */
  .sev-notable .bullet {
    background: var(--warn);
  }
  .sev-notable .badge {
    color: var(--warn);
    border-color: #5a4a22;
  }

  /* fyi */
  .sev-fyi {
    background: transparent;
    border-color: transparent;
    padding: 9px 13px;
  }
  .sev-fyi .bullet {
    background: transparent;
    border: 1px solid var(--info);
  }
  .sev-fyi .name {
    color: var(--tx-mut);
    font-weight: 500;
  }
  .sev-fyi .badge {
    color: var(--info);
    border-color: #36434f;
  }
  .sev-fyi .answer {
    color: var(--tx-dim2);
    font-size: 11.5px;
  }

  /* drift override — the loudest state */
  .slot.drifted {
    background: var(--bg-flag);
    border-color: var(--bd-bad);
  }
  .slot.drifted .bullet {
    background: var(--bad);
    box-shadow: 0 0 8px -1px var(--bad);
  }
  .badge.drift {
    color: var(--bad-bright);
    border-color: var(--bd-bad);
  }
</style>
