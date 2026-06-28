# Kiln Design System — "Terminal Lab"

The visual language for the Kiln desktop app. Source of truth for the look is
[`src/lib/theme.css`](../src/lib/theme.css); this doc explains the intent behind
the tokens and how the shell is composed. The screens were implemented from the
Claude Design doc `Kiln.dc.html` (frames A1 Premise Gate, A2 Free Exploration,
A3 Results Gate).

## Principles

- **Dense, mono-forward.** Data and machine state are rendered in mono; prose and
  headings in sans/serif. The review surface _is_ the product.
- **Ember accent, warm dark.** A near-black warm palette with a single ember
  accent (`--ember`) reserved for "the live / active / your-attention-here" thing.
- **Severity is colour.** Critical = red, notable = amber, FYI = blue, good = green.
  These are consistent everywhere (checklist slots, run flags, diffs, kernel state).
- **No hardcoded hex in components.** Every component references the CSS variables
  in `theme.css`. Add a token before reaching for a literal colour.

## Typography

Loaded via Google Fonts in [`src/app.html`](../src/app.html).

| Token          | Family        | Used for                                  |
| -------------- | ------------- | ----------------------------------------- |
| `--font-sans`  | IBM Plex Sans | Body, labels, buttons                     |
| `--font-mono`  | IBM Plex Mono | Values, code, run names, status, eyebrows |
| `--font-serif` | Newsreader    | Gate titles (italic for the "kiln" brand) |

Eyebrow labels are `--font-mono`, ~10px, uppercase, `letter-spacing: 1.4px`,
in `--tx-mut`.

## Colour tokens

### Surfaces (back to front)

| Token          | Hex       | Used for                         |
| -------------- | --------- | -------------------------------- |
| `--bg`         | `#161513` | App / center background          |
| `--bg-side`    | `#19120e` | Left sidebar                     |
| `--bg-chat`    | `#181612` | Right chat pane                  |
| `--bg-bar`     | `#211f1c` | Titlebar                         |
| `--bg-repl`    | `#120f0c` | REPL strip, inputs               |
| `--bg-panel`   | `#1a1916` | Cards, footers                   |
| `--bg-panel-2` | `#1c1b18` | Table headers, slot rows         |
| `--bg-row-alt` | `#181613` | Zebra rows, hovers               |
| `--bg-sel`     | `#241a13` | Selected / active (ember-tinted) |

Gradients: `--bg-look` (ember "look here" callouts), `--bg-flag` (red drift /
auto-flag callouts), `--logo-grad` / `--ember-grad` (the ember logo + CTA).

### Borders

`--bd` `#2a2825` (default) · `--bd-2` `#34322e` (stronger) · `--bd-soft` `#2f2a23`
(panels) · `--bd-row` `#211d18` (table rows) · `--bd-ember` `#4a3115` (active edge)
· `--bd-bad` / `--bd-good` (status edges).

### Accent & status

| Token           | Hex       | Meaning                           |
| --------------- | --------- | --------------------------------- |
| `--ember`       | `#e8843b` | Live / active / primary CTA       |
| `--ember-soft`  | `#e8a35f` | Ember text on dark                |
| `--ember-serif` | `#c9a06a` | The serif "kiln" brand mark       |
| `--good`        | `#6bbf73` | Idle/ready, kept, clean run       |
| `--warn`        | `#d8a23c` | Notable, booting, awaiting review |
| `--bad`         | `#e5634d` | Critical, kill, flagged / leakage |
| `--info`        | `#6f8aa6` | FYI, ephemeral inspection         |

### Text (bright → muted)

`--tx-title` → `--tx-bright` → `--tx-2`/`--tx-3` → `--tx` (body) → `--tx-dim` →
`--tx-dim2` → `--tx-mut` → `--tx-mut2` → `--tx-mut3`. Use the dimmest shade that
is still legible for the role; reserve the bright end for the one thing that matters
in a row.

## Shell layout

One window, composed in [`AppShell.svelte`](../src/lib/components/AppShell.svelte):

```
┌─ titlebar: ⦿⦿⦿   workspace — kiln              py 3.12 · uv · mlflow ─┐
├──────────┬──────────────────────────────────────────────┬───────────┤
│ sidebar  │  tabs: Checkpoint Code DataFrame Plots Compare │   chat    │
│  project │  ┌────────────────────────────────────────┐   │  (Claude) │
│  kernel  │  │ center body — swaps per active tab       │   │           │
│  invests │  │                                          │   │           │
│  runs    │  └────────────────────────────────────────┘   │           │
│  mlruns  │  repl strip: ● idle  ns: df  In[·]: ▮          │           │
└──────────┴──────────────────────────────────────────────┴───────────┘
```

- **Titlebar** — traffic lights, `project — kiln` (serif italic brand), env line.
- **Sidebar** — project + kernel status (mirrors the sidecar lifecycle), the active
  Investigation, and a compact Runs list (click to select for Compare).
- **Center** — a tab bar with an ember underline on the active tab and a mono
  context hint on the right; the body swaps per tab; a persistent REPL strip shows
  kernel state.
- **Chat** — the Claude pane: assistant turns are bylined and bubble-free, user
  turns are right-aligned bubbles, `⌘↵` sends.

### Tab → component map

| Tab        | Component                        | State source                        |
| ---------- | -------------------------------- | ----------------------------------- |
| Checkpoint | `PremiseGate`                    | `checkpoint-store` pending proposal |
| Code       | `CodeViewPane` (stub)            | —                                   |
| DataFrame  | `DataFrameView` + `SummaryStrip` | `df-store` (Arrow IPC, zero-copy)   |
| Plots      | `PlotPanel`                      | `plot-store` displays               |
| Compare    | `CompareView` + `ResultsGate`    | `runs-store` selection + active run |

When a premise gate fires, the shell auto-focuses Checkpoint; when a run awaits a
verdict, it auto-focuses Compare — the original modal's "you must act" intent,
without trapping the user there.

## Component patterns

- **Severity slot** (`SlotRow`) — coloured bullet + bold name + mono severity
  badge; critical/notable expanded, FYI collapsed (layered disclosure).
- **Look-here callout** — ember-gradient box (`--bg-look`), `👁` glyph. For
  "where I'm least sure".
- **Auto-flag / drift callout** — red-gradient box (`--bg-flag`), `⚑` glyph. The
  harness flags independently of Claude (spec §10).
- **Diff table** (`CompareView`) — banded sections (decisions / params / metrics);
  differing rows tint ember; in a flagged comparison the clean run's value is green,
  the flagged run's red.
- **Decision bars** — primary CTA uses `--ember-grad` with an ember glow; keep =
  `--good`, kill = `--bad`, secondary actions are ghost buttons.

## Conventions

- Reference tokens, never literals. New colour need → add a token here + in
  `theme.css` first.
- Mono for any machine-produced value (metrics, ids, params, kernel state).
- One ember thing per view. If everything is ember, nothing is.
- Status colour is load-bearing — don't reuse `--bad` for decoration.
