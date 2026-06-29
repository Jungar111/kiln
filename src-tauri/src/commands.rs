use serde::Serialize;
use tauri::{AppHandle, Emitter as _, State};

use crate::checkpoint::{DriftEntry, DriftState, ProposeExperiment};
use crate::sidecar_client::{ExecuteResponse, SidecarClient};

/// Max kiln-run execute → feed-back rounds per chat turn. Bounds runaway loops
/// and the number of `claude` subprocess calls a single message can trigger.
const MAX_RUN_ROUNDS: usize = 3;

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
    session: State<'_, crate::claude::ClaudeSession>,
    client: State<'_, SidecarClient>,
) -> Result<String, String> {
    let mut raw = crate::claude::send(&message, &session, &app).await?;

    // Agentic run loop: if Claude emits a `kiln-run` block, execute it in the live
    // kernel, push any plots to the Plot tab, and feed the output back so it can
    // report or fix-and-retry. Capped so a misbehaving turn can't loop forever.
    // ponytail: cap = 3 rounds; raise it if real multi-step runs need more.
    for _ in 0..MAX_RUN_ROUNDS {
        let Some(code) = crate::claude::extract_run_block(&raw) else {
            break;
        };
        let code = code.to_owned();
        // ephemeral = true: chat-driven runs are exploration, excluded from autolog
        // (matches the human-poke semantics). Approved experiment runs go elsewhere.
        let feedback = match client.execute(&code, true).await {
            Ok(result) => {
                if !result.displays.is_empty() {
                    let _ = app.emit("plot:displays", &result.displays);
                }
                format_kernel_output(&result)
            }
            Err(e) => format!(
                "[Kiln kernel — the kiln-run code could not be executed ({}). Tell the human the \
kernel is unavailable; do not give them terminal commands.]",
                e.message
            ),
        };
        raw = crate::claude::send(&feedback, &session, &app).await?;
    }

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

/// Render a kernel result as a plain-text observation to feed back to Claude for
/// the next turn. Framed so Claude treats it as tool output, not a human message,
/// and can either report it or emit a corrected `kiln-run` block.
fn format_kernel_output(r: &ExecuteResponse) -> String {
    let mut out = String::from(
        "[Kiln kernel — output of the kiln-run code you just executed. This is NOT a message \
from the human. Report the result to the human, or if it errored emit a corrected `kiln-run` \
block.]\n",
    );
    out.push_str(&format!("status: {}\n", r.status));
    let stdout = r.stdout.trim_end();
    if !stdout.is_empty() {
        out.push_str(&format!("stdout:\n{stdout}\n"));
    }
    if let Some(value) = r.value.as_deref().filter(|v| !v.is_empty()) {
        out.push_str(&format!("value: {value}\n"));
    }
    if let Some(traceback) = r.traceback.as_deref().filter(|t| !t.is_empty()) {
        out.push_str(&format!("traceback:\n{traceback}\n"));
    }
    if !r.displays.is_empty() {
        out.push_str("(a plot was rendered to the Plot tab)\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::format_kernel_output;
    use crate::sidecar_client::{Display, ExecuteResponse};

    fn ok_response() -> ExecuteResponse {
        ExecuteResponse {
            status: "ok".to_owned(),
            stdout: String::new(),
            value: None,
            traceback: None,
            ephemeral: true,
            df: None,
            displays: Vec::new(),
        }
    }

    #[test]
    fn reports_status_and_stdout_on_ok() {
        let mut r = ok_response();
        r.stdout = "3.14159\n".to_owned();
        let out = format_kernel_output(&r);
        assert!(out.contains("status: ok"), "got: {out}");
        assert!(out.contains("3.14159"), "got: {out}");
    }

    #[test]
    fn reports_traceback_on_error() {
        let mut r = ok_response();
        r.status = "error".to_owned();
        r.traceback = Some("ZeroDivisionError: division by zero".to_owned());
        let out = format_kernel_output(&r);
        assert!(out.contains("status: error"), "got: {out}");
        assert!(out.contains("ZeroDivisionError"), "got: {out}");
    }

    #[test]
    fn notes_a_rendered_plot() {
        let mut r = ok_response();
        r.displays = vec![Display {
            mime: "image/png".to_owned(),
            payload: "AAAA".to_owned(),
            metadata: serde_json::Value::Null,
        }];
        let out = format_kernel_output(&r);
        assert!(out.to_lowercase().contains("plot"), "got: {out}");
    }
}
