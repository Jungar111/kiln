# Ticket 34 — Results gate: keep / kill / iterate

**Phase:** 4
**Depends on:** [33](./33-mlflow-tag-write.md)
**Blocks:** [60](./60-mlflow-query.md)

## Goal

After the experiment finishes (autolog records params + metrics), surface the **results gate** described in spec §3.4. It is lighter than the premise gate — the design review already happened upstream. The human picks one of three outcomes:

| Outcome  | What happens                                                                      |
| -------- | --------------------------------------------------------------------------------- |
| Keep     | Mark the run `kiln.outcome=keep`, end it, fire `sidecar:results_kept`             |
| Kill     | Mark the run `kiln.outcome=kill`, end it, fire `sidecar:results_killed`           |
| Iterate  | Mark the run `kiln.outcome=iterate`, end it, fire `sidecar:results_iterate`       |

The chat continues with a synthesized message reflecting the choice so Claude knows the verdict.

## Files

- Create: `src/lib/components/ResultsGate.svelte`.
- Modify: `src/lib/components/AppShell.svelte`.
- Modify: `sidecar/src/kiln_sidecar/__main__.py` — new RPC `close_run(verdict)`.
- Modify: `src-tauri/src/commands.rs` — new Tauri command `close_run`.

## Steps

- [ ] **1. Failing test (Python).**

  ```python
  def test_close_run_writes_outcome_tag(tmp_path: Path) -> None:
      mlflow.set_tracking_uri(f"sqlite:///{tmp_path / 'mlflow.db'}")
      run_id = start_run_with_decisions(_sample_proposal())
      close_run(run_id, verdict="keep")
      tags = mlflow.get_run(run_id).data.tags
      assert tags["kiln.outcome"] == "keep"
      assert mlflow.get_run(run_id).info.status == "FINISHED"
  ```

- [ ] **2. Implement `close_run`.**

  ```python
  from typing import Literal
  Verdict = Literal["keep", "kill", "iterate"]

  def close_run(run_id: str, verdict: Verdict) -> None:
      with mlflow.start_run(run_id=run_id):
          mlflow.set_tag("kiln.outcome", verdict)
      mlflow.end_run("FINISHED")
  ```

- [ ] **3. RPC + Tauri command.** Pattern matches Ticket 33.

- [ ] **4. UI.**

  ```svelte
  <!-- ResultsGate.svelte -->
  <script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    type Verdict = 'keep' | 'kill' | 'iterate';
    let { runId, onclose }: { runId: string; onclose: () => void } = $props();

    async function pick(verdict: Verdict): Promise<void> {
      await invoke('close_run', { runId, verdict });
      onclose();
    }
  </script>
  <div class="results-gate">
    <button onclick={() => pick('keep')}>keep</button>
    <button onclick={() => pick('kill')}>kill</button>
    <button onclick={() => pick('iterate')}>iterate</button>
  </div>
  ```

- [ ] **5. Wire fire moment.** Sidecar fires `sidecar:run_finished` when `autolog` writes its final metric (use `mlflow.callbacks`, or wrap `mlflow.end_run` in a notify helper). AppShell shows `ResultsGate` while that event is unhandled.

- [ ] **6. Lint + test + commit.**

  ```sh
  git commit -m "feat(checkpoint): results gate writes keep/kill/iterate verdict"
  ```

## Acceptance

- All three verdicts write the tag and end the run.
- The chat picks up the verdict and posts it back to Claude.
- A run that the human ignores stays RUNNING in MLflow — that's fine, it shows up in the comparison view (Ticket 61).

## Out of scope

- Multi-run "promote" / "compare to baseline" — fast-follow. Stops at the single-run keep/kill/iterate choice.
- Editing the verdict after the fact — out of MVP.
