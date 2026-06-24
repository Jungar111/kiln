# Kiln — MVP Spec

_A studio where a professional data scientist's findings become production-grade and stay findable — instead of scattered across CSVs, screenshots, and chat scrollback._

A cross-platform desktop harness for **Claude-driven data science experimentation**. Claude writes experiments, runs the REPL, and logs to MLflow. The human operates one level up — reading code, reading results, and making architectural and data-science decisions, not implementation decisions.

---

## 1. Product thesis

This is **not** "Jupyter with a chat panel." It is a **review-and-decide harness over a Claude-driven IPython + MLflow backend**.

The human is the **flight director / reviewer**, not the pilot. Claude does all implementation; the human sets the frame, judges results, and decides what to keep. The single idea that makes this different: **the human operates at the premise/decision altitude while Claude handles everything below it** — and the harness makes those decisions _un-skippable_ so review stays real rather than becoming a rubber stamp.

The thing that must be true for the product to have a reason to exist: **you can review and adjudicate Claude's experiments without dropping into implementation, and you can catch the data-science errors that are your job to catch.**

---

## 2. Scope assumptions (MVP)

- **Local data** only.
- **uv** as package + virtual-environment manager (one venv per project; per-worktree later).
- **IPython** as the execution substrate, run as a sidecar subprocess, one kernel per project.
- **MLflow** as the compare/tracking engine (local sqlite backend, no server).
- **Tauri** as the desktop shell (Rust core + webview frontend).
- **Worktrees are explicitly out of scope for the MVP.** Experiment flow is the MVP; worktree-based parallelism comes later.

---

## 3. The interaction model: premise-gated investigation

The unit of work is an **investigation**, organized around a **shared premise**.

1. **Premise gate (pre-compute).** Claude proposes an investigation ("implement and evaluate feature X"). The human reviews the _frame_ — not the steps — and approves. This is a **handshake that establishes shared context**, not a permission toll. It is the human's highest-leverage moment and the heaviest review point.
2. **Free collaborative exploration (no gates).** Once the premise is agreed, Claude explores freely. Subsequent investigations _within the agreed premise_ do not need approval. Human and Claude investigate together; the REPL is live for either party.
3. **Re-gate on premise drift.** If Claude's work would change a **locked frame decision** (target, validation strategy, feature provenance, metric, population), a new premise gate fires. Rule: **gate the frame, not the steps; re-gate when the frame changes.**
4. **Results gate (post-compute).** Lighter than the premise gate, because the real review already happened upstream. Did the agreed investigation answer the question? **Keep / kill / iterate.**

### Why premise-level (not per-fit)

- Per-fit gating causes **checkpoint fatigue** → rubber-stamping with extra clicks.
- Premise-level gating places ~one heavy gate per investigation, where leverage is highest, then drops friction to zero.
- Trade-off accepted: a wrong premise has a **wider blast radius** (every downstream step inherits the flaw). Mitigation: the premise gate must surface frame-level decisions well, and the drift tripwire must re-gate on frame changes.

---

## 4. The checkpoint object

A checkpoint is **filled, not written** — Claude populates a structured `propose_experiment` tool call. The harness will not render the gate until in-scope required slots are populated. This converts "trust Claude to mention the train/test split" into "the split decision is a structural field that cannot be empty." Claude can fill a slot _wrong_ (visible, challengeable) but not _invisibly_ (the failure that matters).

### Required slots (the validity / leakage checklist)

Each experiment marks which slots are **in scope**; in-scope slots are required. The taxonomy is deliberately a leakage-and-validity checklist, because that is the senior-DS reviewer's actual job.

1. **Validation strategy** — split / CV scheme, temporal vs random. (Top leakage vector; temporal-vs-random is _the_ call for time-series work.)
2. **Target definition** — what `y` is, how constructed, any lookahead in the label.
3. **Feature provenance** — is any feature computed from information unavailable at prediction time.
4. **Preprocessing fit scope** — scalers / imputers / encoders fit before or after the split.
5. **Data scope & exclusions** — rows dropped, what population this is actually about.
6. **Missing-data handling** — drop vs impute, and whether imputation leaks.
7. **Metric choice** — and whether it fits the problem (e.g. AUC under imbalance; proper scoring rules for calibration / probabilistic work).

A slot may legitimately be answered "N/A" (e.g. EDA → validation N/A) but it must be answered.

### Scannability devices

- **Severity tiers** per surfaced decision: _critical_ (validity-determining), _notable_ (worth knowing), _fyi_ (logged for completeness). Critical foregrounded; rest collapsed.
- **"Look here" flag** — Claude marks where it is least sure / most wants the human's eye. Inverts the rubber-stamp dynamic by routing attention to the soft spots.
- **Layered disclosure** — (1) tiered decisions always visible → (2) the code Claude wrote, on demand → (3) live state via the REPL. Read decisions, expand code if suspicious, poke state if still suspicious, then decide.

### What approval binds to

Approval binds to the **declared decisions, not every line of code**. The human certifies the design choices, not a full code read. Declared decisions are written as **metadata (tags) on the MLflow run** — so the thing approved becomes the thing compared.

---

## 5. The surfaces (priority order)

The review surface _is_ the product. Priority reflects MVP-criticality.

1. **Experiment code view.** A readable view of the experiment Claude wrote / is about to run — first-class and legible, expandable from the checkpoint. (Not full worktree machinery; just "here is the script.")
2. **Results legible enough to make DS calls.**
   - **DataFrame explorer** — not just a scrollable grid: summary stats, distributions, null patterns, shapes. Bar is "lets the human smell when a result is wrong" (leakage, degenerate distribution, suspicious AUC). Zero-copy transport (Arrow IPC, server-side sort/filter/paginate); **never marshal frames through Rust IPC**.
   - **Plot viewer** — intercepts kernel `display_data` / `execute_result` MIME bundles (`image/png`, `text/html`) and routes to a plot panel.
3. **Run comparison (the keep/kill decision surface).** Built on MLflow as **engine, not surface** — query via `mlflow.search_runs()` / `MlflowClient`, render an **inline** comparison view inside the app. Shows params + metrics **and** declared premise decisions as first-class diffs ("A: temporal split / B: random split"). The inline result-diff is the differentiator and is ours to build; MLflow only stores the ingredients.
4. **Chat with Claude.** Where the human directs and Claude reports / proposes checkpoints.

### Secondary (glass-box aids, optional for MVP)

- **REPL access for manual inspection.** Not the main flow. Lives in the **idle-kernel moment at a checkpoint** — the instrument of the review, the antidote to rubber-stamping (verify, don't just read Claude's self-summary).
- **Variable / state inspector** — name, type, shape, memory footprint. Demoted from load-bearing (Claude manages state now) to a debugging aid.
- **Staleness indicator** — kernel-vs-disk drift. Optional in MVP.

---

## 6. Architecture (process topology)

- **Rust core (Tauri):** orchestration + control plane — process lifecycle, the diff/editor IPC, checkpoint routing. Owns durable state.
- **Python sidecar (one per project):** the IPython kernel (driven via `jupyter_client` over ZMQ) **plus** a small Arrow IPC server in the same process for DataFrame paging.
- **Webview frontend:** editor / code view, df explorer, plot viewer, run comparison, chat. Talks to **both** — control-plane via Rust, bulk data **directly** to the Python Arrow server.

**Hard rules:**

- Big bytes (DataFrames) take the direct Python ↔ webview path. Rust IPC is control-plane only.
- DataFrames are never rendered via the kernel's default HTML repr — a custom display hook registers the frame in a side store and emits a lightweight handle; the viewer pages it over the local Arrow socket.
- Heavy data artifacts (e.g. multi-GB parquet) are logged to MLflow as **path + hash**, not bytes. Only light artifacts (plots, metrics, small model pickles) go into `mlruns/`.

---

## 7. Shared kernel, two drivers

Both Claude and the human can act on the **same** kernel namespace. Resolution principle: **symmetric legibility**, not enforced read-only (unenforceable in Python).

- Claude's consequential choices surface to the human as checkpoints.
- The human's state-changing manual actions surface to Claude before it resumes ("user ran X; namespace now reflects Y").
- Most inspection is non-mutating; this only matters when it isn't — which is exactly when silence would burn either party.

**Inspection is ephemeral; the experiment is the record.** Manual human pokes must not land in the logged MLflow run or the promoted script. The harness draws a clean line between **experiment commands** (Claude's logged, reproducible, promotable actions) and **inspection commands** (human pokes, ephemeral). Draw this from day one — cheap now, miserable to retrofit.

---

## 8. MLflow integration notes

- **Engine, not surface.** Use the tracking backend + logging API + autolog; build our own inline compare view on its data.
- **Interactive run boundaries via autolog.** `mlflow.<flavor>.autolog()` makes `.fit()` the natural run boundary — with no active run it opens/closes one automatically, capturing params/metrics/model. The fifty subsequent probes are reads against the live object + the logged run.
- **Thin run-lifecycle abstraction** over MLflow for non-`.fit()` work (calibration / PIT / probabilistic forecasts): the human/Claude marks a run via a harness affordance, not by typing tracking calls. This thin layer is the real integration work and is small.
- **Local config:** tracking URI → local sqlite (snappier `search_runs` than the bare file store). `uv add mlflow` per venv.

---

## 9. MVP cut

**In:**

- Premise-gated investigation flow (premise gate → free exploration → results gate; re-gate on drift).
- The checkpoint object: `propose_experiment` tool-schema with **required in-scope validity slots** (tool-boundary enforcement — reject/re-prompt on empty consequential slots), severity tiers, "look here" flag, layered disclosure.
- Experiment code view.
- DataFrame explorer (with stats/distributions/nulls) over Arrow IPC.
- Plot viewer.
- Inline run comparison over MLflow, showing declared premise decisions as diffs.
- Chat with Claude.
- REPL for manual inspection at checkpoints (secondary, but present).
- IPython sidecar + MLflow (sqlite) + uv + Tauri plumbing.

**Out (deferred):**

- Worktrees / parallel-experiment and parallel-agent workflows.
- Promotion (session → clean script). Fast-follow.
- DS **linter** that auto-raises slots from static code analysis (the strong form of un-skippable). Design slots to accept it later.
- Declaration-vs-implementation contract checking ("Claude said temporal split — does the code do `shuffle=False`?").
- Variable inspector + staleness indicator as polished surfaces (ship minimal or skip).
- Remote/cloud data, data connectors, environment-management UI.
- Drift-detection sophistication beyond "a locked slot value would change → re-gate."

---

## 10. Known risks

- **Disempowered-reviewer trap.** Agent does all implementation; review degrades to rubber-stamping. _Antidote:_ un-skippable decision slots + "look here" flags + REPL verification. This is the central product risk — the surfaces exist to keep review real.
- **Checkpoint fatigue.** Too many gates → rubber-stamping. _Antidote:_ premise-level granularity.
- **Premise drift.** Free exploration silently erodes the agreed frame. _Antidote:_ re-gate when a locked slot value would change.
- **Self-grading weakness.** Claude is unreliable at critiquing its own results. _Antidote:_ cheap deterministic auto-flags raised by the harness independent of Claude (e.g. AUC above threshold → "check leakage"; large train/test metric gap → "check overfit"), plus human REPL inspection. Never let Claude be the only thing checking Claude.
- **Long-running runs block the kernel.** A 20-min fit on a single kernel prevents the "probe fifty times" flow. _Decide early:_ background/async execution, a queue, or accept the block. Painful to retrofit; interacts with one-kernel-per-project.

---

## 11. Open questions for next pass

- **Drift detection:** what trips a re-gate vs. what counts as a step within the premise. (Mechanism: compare current actions against locked slot values; the judgment is change vs. step-within.)
- **Long-running execution model:** async/queue/block — see risks.
- **Checkpoint UX detail:** exactly what "approve" commits to visually, and how the results gate reads keep/kill/iterate.
- **Promotion design** (deferred but worth pre-shaping): what the session capture needs to contain for clean reconstruction.
