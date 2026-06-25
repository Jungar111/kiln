use serde::Serialize;
use tauri::{AppHandle, Emitter as _, State};

use crate::sidecar_client::{ExecuteResponse, SidecarClient};

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
#[tauri::command]
pub async fn chat(message: String, app: AppHandle) -> Result<String, String> {
    let raw = crate::claude::send(&message).await?;
    let prose = crate::claude::strip_proposal_block(&raw);
    match crate::claude::extract_proposal(&raw) {
        None => Ok(raw),
        Some(Ok(proposal)) => {
            let _ = app.emit("checkpoint:proposed", &proposal);
            Ok(if prose.is_empty() {
                "Proposed an experiment — see the premise gate.".to_owned()
            } else {
                prose
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
#[tauri::command]
pub async fn approve_checkpoint(
    proposal: crate::checkpoint::ProposeExperiment,
    client: State<'_, SidecarClient>,
) -> Result<ApproveResponse, ExecuteCommandError> {
    let run_id = client
        .approve_checkpoint(&proposal)
        .await
        .map_err(|e| ExecuteCommandError {
            code: e.code,
            message: e.message,
        })?;
    Ok(ApproveResponse { run_id })
}

/// Results gate: record the keep/kill/iterate verdict and finish the MLflow run.
#[tauri::command]
pub async fn close_run(
    run_id: String,
    verdict: String,
    client: State<'_, SidecarClient>,
) -> Result<(), ExecuteCommandError> {
    client
        .close_run(&run_id, &verdict)
        .await
        .map_err(|e| ExecuteCommandError {
            code: e.code,
            message: e.message,
        })
}
