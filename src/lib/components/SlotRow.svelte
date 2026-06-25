<script lang="ts">
  import type { Slot } from '$lib/checkpoint-types';

  let { label, slot, drifted = false }: { label: string; slot: Slot; drifted?: boolean } = $props();

  // Critical/notable expanded by default; fyi collapsed (layered disclosure).
  // A drifted slot is always expanded — it is the reason the gate re-fired.
  const open = $derived(drifted || slot.severity !== 'fyi');
</script>

<details class="slot severity-{slot.severity}" class:drifted {open}>
  <summary>
    <span class="name">{label}</span>
    <span class="badge">{slot.severity}</span>
    {#if drifted}<span class="drift-badge">drift</span>{/if}
    {#if !slot.in_scope}<span class="oos">out of scope</span>{/if}
  </summary>
  <p class="answer">{slot.answer}</p>
</details>

<style>
  .slot {
    border-left: 3px solid #444;
    padding: 6px 10px;
    background: #1a1a1a;
    border-radius: 4px;
  }
  .severity-critical {
    border-left-color: #e0563f;
  }
  .severity-notable {
    border-left-color: #d9a441;
  }
  .severity-fyi {
    border-left-color: #555;
    opacity: 0.85;
  }
  .slot.drifted {
    border-left-color: #b3372a;
    background: rgba(224, 86, 63, 0.1);
    opacity: 1;
  }
  .drift-badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 1px 6px;
    border-radius: 999px;
    background: #b3372a;
    color: #ffd9d2;
    font-weight: 700;
  }
  summary {
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-weight: 600;
  }
  .badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 1px 6px;
    border-radius: 999px;
    background: #2a2a2a;
    color: #bbb;
  }
  .oos {
    font-size: 11px;
    color: #888;
  }
  .answer {
    margin: 6px 0 0;
    color: #ddd;
    white-space: pre-wrap;
  }
</style>
