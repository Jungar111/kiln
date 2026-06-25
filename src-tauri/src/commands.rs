use serde::Serialize;
use tauri::{AppHandle, Emitter as _, State};

use crate::checkpoint::{DriftEntry, DriftState, ProposeExperiment};
use crate::sidecar_client::{ExecuteResponse, SidecarClient};

/// Payload of the `checkpoint:proposed` event. `drift` lists the locked in-scope
/// slots this proposal would change; empty for the first proposal of a run (no
/// lock yet) or when nothing in the locked frame moved.
#[derive(Debug, Clone, Serialize)]
pub struct CheckpointProposed<'a> {
    pub proposal: &'a ProposeExperiment,
    pub drift: Vec<DriftEntry>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteCommandError {
    pub code: i64,
    pub message: String,
}

#[tauri::command]
pub async fn ping(client: State<'_, SidecarClient>) -> Result<String, ExecuteCommandError> {
    client.ping().await.map_err(|e| ExecuteCommandError {
        code: e.code,
        message: e.message,
    })
}

#[tauri::command]
pub async fn execute(
    code: String,
    ephemeral: bool,
    client: State<'_, SidecarClient>,
) -> Result<ExecuteResponse, ExecuteCommandError> {
    client
        .execute(&code, ephemeral)
        .await
        .map_err(|e| ExecuteCommandError {
            code: e.code,
            message: e.message,
        })
}

/// Send the user's message to Claude via the Claude Code CLI and return the
/// text reply. Auth, tools, MCP, and project context are handled by the CLI.
///
/// If Claude's reply carries a `kiln-experiment` proposal block, fire the
/// premise gate: a valid proposal is emitted as `checkpoint:proposed`; an
/// invalid one is surfaced in the chat so Claude can self-correct next turn.
///
/// Drift detection (Ticket 72): when a run is open, the proposal is diffed
/// against the locked frame. Any changed in-scope slot rides along on the event
/// so the gate can re-fire with a "drift detected" banner.
#[tauri::command]
pub async fn chat(
    message: String,
    app: AppHandle,
    drift: State<'_, DriftState>,
) -> Result<String, String> {
    let raw = crate::claude::send(&message).await?;
    let prose = crate::claude::strip_proposal_block(&raw);
    match crate::claude::extract_proposal(&raw) {
        None => Ok(raw),
        Some(Ok(proposal)) => {
            let changed = drift.check_drift(&proposal);
            let drifted = !changed.is_empty();
            let _ = app.emit(
                "checkpoint:proposed",
                CheckpointProposed {
                    proposal: &proposal,
                    drift: changed,
                },
            );
            Ok(if !prose.is_empty() {
                prose
            } else if drifted {
                "Re-firing the premise gate — a locked decision would change.".to_owned()
            } else {
                "Proposed an experiment — see the premise gate.".to_owned()
            })
        }
        Some(Err(errors)) => Ok(format!(
            "{prose}\n\n⚠️ Proposal not gated — fix these slots: {errors}"
        )),
    }
}

#[derive(Debug, Serialize)]
pub struct ApproveResponse {
    pub run_id: String,
}

/// Approve a premise gate: forward the proposal to the sidecar, which opens an
/// MLflow run tagged with the declared decisions, and return the run id.
///
/// On success, snapshot the approved proposal as the locked frame so subsequent
/// proposals can be diffed for premise drift (Ticket 72).
#[tauri::command]
pub async fn approve_checkpoint(
    proposal: ProposeExperiment,
    client: State<'_, SidecarClient>,
    drift: State<'_, DriftState>,
) -> Result<ApproveResponse, ExecuteCommandError> {
    let run_id = client
        .approve_checkpoint(&proposal)
        .await
        .map_err(|e| ExecuteCommandError {
            code: e.code,
            message: e.message,
        })?;
    drift.lock(&proposal);
    Ok(ApproveResponse { run_id })
}

/// Results gate: record the keep/kill/iterate verdict and finish the MLflow run.
///
/// Closing the run releases the locked frame (Ticket 72): the investigation is
/// over, so the next proposal starts a fresh frame and cannot report drift.
#[tauri::command]
pub async fn close_run(
    run_id: String,
    verdict: String,
    client: State<'_, SidecarClient>,
    drift: State<'_, DriftState>,
) -> Result<(), ExecuteCommandError> {
    client
        .close_run(&run_id, &verdict)
        .await
        .map_err(|e| ExecuteCommandError {
            code: e.code,
            message: e.message,
        })?;
    drift.release();
    Ok(())
}

/// Return recent MLflow runs for the comparison view (Ticket 61).
#[tauri::command]
pub async fn list_runs(
    limit: u32,
    client: State<'_, SidecarClient>,
) -> Result<serde_json::Value, ExecuteCommandError> {
    client
        .list_runs(limit)
        .await
        .map_err(|e| ExecuteCommandError {
            code: e.code,
            message: e.message,
        })
}

/// Report the in-kernel Arrow HTTP server port so the webview can page frames.
#[tauri::command]
pub async fn arrow_port(client: State<'_, SidecarClient>) -> Result<u32, ExecuteCommandError> {
    client.arrow_port().await.map_err(|e| ExecuteCommandError {
        code: e.code,
        message: e.message,
    })
}
