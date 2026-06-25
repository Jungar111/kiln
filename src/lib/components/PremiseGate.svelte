<script lang="ts">
  import { SLOT_FIELDS, type ProposeExperiment } from '$lib/checkpoint-types';
  import SlotRow from './SlotRow.svelte';
  import InspectionRepl from './InspectionRepl.svelte';

  let {
    proposal,
    onapprove,
    ondecline,
  }: {
    proposal: ProposeExperiment;
    onapprove: (proposal: ProposeExperiment) => void;
    ondecline: () => void;
  } = $props();
</script>

<div class="backdrop">
  <div class="gate" role="dialog" aria-modal="true" aria-label="Premise gate">
    <header>
      <h2>{proposal.title}</h2>
      <p class="premise">{proposal.premise}</p>
    </header>

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
        <SlotRow {label} slot={proposal[key]} />
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
  h2 {
    margin: 0 0 4px;
  }
  .premise {
    margin: 0;
    color: #bbb;
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
