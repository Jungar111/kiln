<script lang="ts">
  import {
    SLOT_FIELDS,
    slotLabel,
    type DriftEntry,
    type ProposeExperiment,
    type Severity,
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

  const driftedKeys = $derived(new Set(drift.map((d) => d.slot)));

  // Severity descending — the heavy review point puts the riskiest decisions first.
  const RANK: Record<Severity, number> = { critical: 0, notable: 1, fyi: 2 };
  const rows = $derived(
    [...SLOT_FIELDS]
      .map(([key, label]) => ({ key, label, slot: proposal[key] }))
      .sort((a, b) => RANK[a.slot.severity] - RANK[b.slot.severity]),
  );

  const inScope = $derived(rows.filter((r) => r.slot.in_scope));
  const filled = $derived(inScope.filter((r) => r.slot.answer.trim() !== '').length);
</script>

<div class="gate" class:drifted={drift.length > 0}>
  <div class="scroll">
    <!-- header -->
    <div class="head">
      <div class="head-main">
        <div class="eyebrow">Premise gate · pre-compute</div>
        <h1 class="title">{proposal.title}</h1>
        <p class="premise">
          {proposal.premise} You certify the <span class="hi">design decisions</span>, not every
          line. Approval writes these as tags on the MLflow run.
        </p>
      </div>
      <div class="counter">{filled} / {inScope.length} in-scope slots filled</div>
    </div>

    {#if drift.length > 0}
      <div class="drift" role="alert">
        <div class="drift-title">
          ⚑ Drift — a locked decision would change. Re-confirm the frame.
        </div>
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
      <div class="look">
        <div class="look-eye">👁</div>
        <div>
          <div class="look-title">Look here — where I'm least sure</div>
          {#each proposal.look_here as item (item)}
            <div class="look-item">{item}</div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- decisions -->
    <div class="sec">
      <span class="sec-label">Frame decisions · leakage &amp; validity checklist</span>
      <span class="rule"></span>
      <span class="sec-hint">severity ↓</span>
    </div>

    <div class="slots">
      {#each rows as row (row.key)}
        <SlotRow label={row.label} slot={row.slot} drifted={driftedKeys.has(row.key)} />
      {/each}
    </div>

    <!-- The instrument of the review: poke the idle kernel before deciding. The
         REPL lives on this component, so declining/approving discards its session. -->
    <InspectionRepl />
  </div>

  <!-- decision bar -->
  <footer>
    <div class="hints">
      <span>▸ expand experiment code</span>
      <span class="dot-sep">·</span>
      <span>⌘K poke the kernel</span>
    </div>
    <div class="actions">
      <button type="button" class="ghost" onclick={ondecline}>Request changes</button>
      <button
        type="button"
        class="approve"
        onclick={() => {
          onapprove(proposal);
        }}>Approve premise →</button
      >
    </div>
  </footer>
</div>

<style>
  .gate {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .scroll {
    flex: 1;
    overflow: auto;
    padding: 18px 22px 0;
  }

  .head {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }
  .head-main {
    flex: 1;
  }
  .eyebrow {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 1.4px;
    text-transform: uppercase;
    color: var(--tx-mut);
  }
  .title {
    font-family: var(--font-serif);
    font-size: 25px;
    font-weight: 500;
    color: var(--tx-title);
    margin: 5px 0 0;
    line-height: 1.2;
  }
  .premise {
    margin: 6px 0 0;
    color: var(--tx-dim);
    max-width: 560px;
  }
  .hi {
    color: var(--tx-3);
  }
  .counter {
    flex-shrink: 0;
    background: var(--bg-sel);
    border: 1px solid var(--bd-ember);
    color: var(--ember-soft);
    font-family: var(--font-mono);
    font-size: 10.5px;
    padding: 5px 9px;
    border-radius: 5px;
  }

  .drift {
    margin-top: 14px;
    background: var(--bg-flag);
    border: 1px solid var(--bd-bad);
    border-radius: 8px;
    padding: 11px 14px;
  }
  .drift-title {
    color: var(--bad-bright);
    font-weight: 600;
    font-size: 12px;
  }
  .drift ul {
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .drift li {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 11.5px;
  }
  .drift-slot {
    color: var(--tx-2);
  }
  .drift-old {
    color: var(--tx-mut);
    text-decoration: line-through;
  }
  .drift-arrow {
    color: var(--tx-mut2);
  }
  .drift-new {
    color: var(--good);
  }

  .look {
    margin-top: 16px;
    display: flex;
    gap: 11px;
    padding: 12px 14px;
    background: var(--bg-look);
    border: 1px solid var(--bd-ember);
    border-radius: 8px;
  }
  .look-eye {
    font-size: 15px;
    line-height: 1;
  }
  .look-title {
    color: var(--ember-soft);
    font-weight: 600;
    font-size: 12px;
  }
  .look-item {
    color: var(--tx-2);
    margin-top: 3px;
    max-width: 600px;
  }

  .sec {
    margin-top: 18px;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .sec-label {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 1.4px;
    text-transform: uppercase;
    color: var(--tx-mut);
  }
  .rule {
    flex: 1;
    height: 1px;
    background: var(--bd);
  }
  .sec-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--tx-mut2);
  }
  .slots {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-bottom: 14px;
  }

  footer {
    flex-shrink: 0;
    border-top: 1px solid var(--bd);
    background: var(--bg-panel);
    padding: 12px 22px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .hints {
    display: flex;
    gap: 8px;
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--tx-mut);
  }
  .dot-sep {
    color: var(--tx-mut3);
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: 10px;
  }
  button {
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
  }
  .ghost {
    font-size: 12px;
    color: var(--tx-2);
    background: transparent;
    border: 1px solid #3a3630;
    padding: 8px 16px;
  }
  .ghost:hover {
    border-color: var(--tx-mut);
  }
  .approve {
    font-size: 12.5px;
    font-weight: 600;
    color: #1a1208;
    background: var(--ember-grad);
    padding: 9px 20px;
    box-shadow: 0 6px 18px -6px var(--ember);
  }
  .approve:hover {
    filter: brightness(1.05);
  }
</style>
