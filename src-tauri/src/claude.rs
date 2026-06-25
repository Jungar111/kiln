use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

/// Hard ceiling so a wedged `claude` subprocess can't hang the chat forever.
// ponytail: fixed 5-min timeout; lift it if real agentic turns run longer.
const TIMEOUT: Duration = Duration::from_secs(300);

/// The shape of `claude --output-format json` (we only read what we need).
#[derive(Debug, Deserialize)]
struct CliResult {
    is_error: bool,
    result: Option<String>,
}

/// Run one non-interactive Claude Code turn and return its text result.
///
/// We shell out to the `claude` CLI (`-p --output-format json`) instead of the
/// raw Anthropic API: auth, tools, MCP, and project context all come for free
/// from the user's existing Claude Code install. The prompt is piped over stdin
/// so a message that looks like a flag (e.g. "--continue") can't be misparsed.
///
/// Out of scope (Ticket 22): multi-turn. Each call is a fresh session. Passing
/// `--session-id` / `--resume` is the one-line upgrade when thread support lands.
pub async fn send(prompt: &str) -> Result<String, String> {
    tokio::time::timeout(TIMEOUT, run(prompt))
        .await
        .map_err(|_| format!("claude timed out after {}s", TIMEOUT.as_secs()))?
}

async fn run(prompt: &str) -> Result<String, String> {
    let mut child = Command::new("claude")
        .arg("-p")
        .arg("--output-format")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to run `claude` (is it on PATH?): {e}"))?;

    // Take/write/drop closes the pipe, signalling EOF so the CLI proceeds.
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(prompt.as_bytes())
        .await
        .map_err(|e| format!("failed to send prompt to claude: {e}"))?;

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("claude failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "claude exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    parse_output(&output.stdout)
}

/// Extract the text reply from `claude --output-format json` stdout.
fn parse_output(stdout: &[u8]) -> Result<String, String> {
    let parsed: CliResult = serde_json::from_slice(stdout)
        .map_err(|e| format!("could not parse claude output: {e}"))?;
    let text = parsed.result.unwrap_or_default();
    if parsed.is_error {
        return Err(if text.is_empty() {
            "claude reported an error".to_owned()
        } else {
            text
        });
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::parse_output;

    #[test]
    fn extracts_result_text() {
        let raw = br#"{"type":"result","subtype":"success","is_error":false,"result":"pong","session_id":"x"}"#;
        assert_eq!(parse_output(raw).unwrap(), "pong");
    }

    #[test]
    fn surfaces_error_subtype() {
        let raw = br#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"boom"}"#;
        assert_eq!(parse_output(raw).unwrap_err(), "boom");
    }
}
