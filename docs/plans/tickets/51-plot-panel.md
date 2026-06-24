# Ticket 51 — Plot panel rendering displays

**Phase:** 6
**Depends on:** [50](./50-mime-router.md), [20](./20-app-shell.md)
**Blocks:** none in MVP

## Goal

A `<PlotPanel>` component in the Results pane that walks `displays` in priority order and renders the first one it can: `text/html` → sandboxed iframe; `image/png` → `<img src="data:image/png;base64,...">`; `image/svg+xml` → `<svg>` direct; everything else → collapsed text.

## Files

- Create: `src/lib/components/PlotPanel.svelte`.
- Modify: `src/lib/components/ResultsPane.svelte` — render `<PlotPanel displays={result.displays} />` alongside the DataFrame view.

## Steps

- [ ] **1. Component.**

  ```svelte
  <script lang="ts">
    type Display = {
      readonly mime: string;
      readonly payload: string;
      readonly metadata: Readonly<Record<string, unknown>>;
    };
    let { displays }: { displays: readonly Display[] } = $props();

    const priority = ['text/html', 'image/svg+xml', 'image/png', 'text/plain'] as const;
    type Mime = (typeof priority)[number];

    function pick(): Display | null {
      for (const mime of priority) {
        const match = displays.find((d) => d.mime === mime);
        if (match) return match;
      }
      return null;
    }
    const chosen = $derived(pick());
  </script>

  {#if chosen}
    {#if chosen.mime === 'image/png'}
      <img src={`data:image/png;base64,${chosen.payload}`} alt="plot" />
    {:else if chosen.mime === 'image/svg+xml'}
      {@html chosen.payload}
    {:else if chosen.mime === 'text/html'}
      <iframe sandbox="allow-scripts" srcdoc={chosen.payload} title="plot"></iframe>
    {:else}
      <pre>{chosen.payload}</pre>
    {/if}
  {/if}
  ```

  **Note on `{@html}`:** SVG comes from a kernel we control. It is still untrusted as data; the iframe-sandbox path handles HTML. SVG `{@html}` is a calculated risk in the MVP — call it out in `CLAUDE.md` if a follow-up ticket tightens this.

- [ ] **2. Wire into ResultsPane.**

- [ ] **3. Smoke test.**

  - `import matplotlib.pyplot as plt; plt.plot([1,2,3]); plt.gcf()` → PNG renders.
  - `import plotly.express as px; px.scatter(x=[1,2,3], y=[3,2,1])` → interactive HTML renders in an iframe (no parent-page script execution).

- [ ] **4. Lint + commit.**

  ```sh
  git commit -m "feat(plot): plot panel rendering matplotlib + plotly displays"
  ```

## Acceptance

- Plotly's HTML is sandboxed (verify by trying `window.top` inside — must be blocked).
- The PNG case has no flash of layout shift.
- No `any`. No `eslint-disable`.

## Out of scope

- Multiple displays per cell (collage view) — fast-follow.
- Click-to-export — fast-follow.
- ipywidgets — out of MVP.
