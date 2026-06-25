use serde::Serialize;
use tauri::State;

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
#[tauri::command]
pub async fn chat(message: String) -> Result<String, String> {
    crate::claude::send(&message).await
}
