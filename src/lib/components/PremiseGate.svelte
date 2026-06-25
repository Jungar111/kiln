<script lang="ts">
  import {
    SLOT_FIELDS,
    slotLabel,
    type DriftEntry,
    type ProposeExperiment,
  } from '$lib/checkpoint-types';
  import SlotRow from './SlotRow.svelte';
  import InspectionRepl from './InspectionRepl.svelte';

  let {
    proposal,
    drift = [],
    onapprove,
    ondecline,
  }: {
    proposal: ProposeExperiment;
    /** Locked in-scope slots this proposal would change (Ticket 72). */
    drift?: readonly DriftEntry[];
    onapprove: (proposal: ProposeExperiment) => void;
    ondecline: () => void;
  } = $props();

  // Which slot keys drifted, so the matching SlotRow can flag itself.
  const driftedKeys = $derived(new Set(drift.map((d) => d.slot)));
</script>

<div class="backdrop">
  <div
    class="gate"
    class:drifted={drift.length > 0}
    role="dialog"
    aria-modal="true"
    aria-label="Premise gate"
  >
    <header>
      {#if drift.length > 0}<span class="drift-flag">Drift</span>{/if}
      <h2>{proposal.title}</h2>
      <p class="premise">{proposal.premise}</p>
    </header>

    {#if drift.length > 0}
      <div class="drift-banner" role="alert">
        <strong>Drift detected</strong>
        <p>A locked decision would change. Re-confirm the frame before proceeding.</p>
        <ul>
          {#each drift as entry (entry.slot)}
            <li>
              <span class="drift-slot">{slotLabel(entry.slot)}</span>
              <span class="drift-old">{entry.old}</span>
              <span class="drift-arrow">→</span>
              <span class="drift-new">{entry.new}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if proposal.look_here.length > 0}
      <div class="look-here">
        <strong>Look here</strong>
        <ul>
          {#each proposal.look_here as item (item)}
            <li>{item}</li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="slots">
      {#each SLOT_FIELDS as [key, label] (key)}
        <SlotRow {label} slot={proposal[key]} drifted={driftedKeys.has(key)} />
      {/each}
    </div>

    <!-- The instrument of the review: poke the idle kernel before deciding. The
         REPL lives on this component, so declining/approving discards its session. -->
    <InspectionRepl />

    <footer>
      <!-- Real CodeView lands in Phase 5; placeholder for now. -->
      <button type="button" class="ghost" disabled>show experiment code</button>
      <div class="spacer"></div>
      <button type="button" class="decline" onclick={ondecline}>decline</button>
      <button
        type="button"
        class="approve"
        onclick={() => {
          onapprove(proposal);
        }}>approve</button
      >
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: grid;
    place-items: center;
    z-index: 10;
  }
  .gate {
    width: min(680px, 92vw);
    max-height: 88vh;
    overflow: auto;
    background: #161616;
    color: #e6e6e6;
    border: 1px solid #333;
    border-radius: 10px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .gate.drifted {
    border-color: #b3372a;
    box-shadow: 0 0 0 1px rgba(224, 86, 63, 0.4);
  }
  h2 {
    margin: 0 0 4px;
  }
  .premise {
    margin: 0;
    color: #bbb;
  }
  .drift-flag {
    display: inline-block;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #ffd9d2;
    background: #b3372a;
    padding: 2px 8px;
    border-radius: 999px;
    margin-bottom: 6px;
  }
  .drift-banner {
    border: 1px solid #b3372a;
    background: rgba(224, 86, 63, 0.12);
    border-radius: 8px;
    padding: 10px 12px;
  }
  .drift-banner > p {
    margin: 4px 0 0;
    color: #e8c4bd;
  }
  .drift-banner ul {
    margin: 8px 0 0;
    padding-left: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .drift-banner li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    font-size: 13px;
  }
  .drift-slot {
    font-weight: 600;
  }
  .drift-old {
    color: #c99;
    text-decoration: line-through;
  }
  .drift-arrow {
    color: #888;
  }
  .drift-new {
    color: #eafff2;
  }
  .look-here {
    border: 1px solid #e0563f;
    background: rgba(224, 86, 63, 0.08);
    border-radius: 8px;
    padding: 8px 12px;
  }
  .look-here ul {
    margin: 6px 0 0;
    padding-left: 18px;
  }
  .slots {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .spacer {
    flex: 1;
  }
  button {
    border: none;
    border-radius: 6px;
    padding: 8px 16px;
    cursor: pointer;
    font: inherit;
  }
  .ghost {
    background: #222;
    color: #888;
    cursor: not-allowed;
  }
  .decline {
    background: #3a2a2a;
    color: #e6b0a4;
  }
  .approve {
    background: #2e6f4e;
    color: #eafff2;
  }
</style>
