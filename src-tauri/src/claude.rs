use std::process::Stdio;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, Emitter as _};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _};
use tokio::process::Command;

use crate::checkpoint::ProposeExperiment;

/// Holds the Claude Code session id so chat turns continue one conversation.
///
/// The first turn runs a fresh session and the CLI assigns an id, which we keep;
/// every later turn passes `--resume <id>` so Claude retains the prior context.
#[derive(Default)]
pub struct ClaudeSession {
    id: Mutex<Option<String>>,
}

impl ClaudeSession {
    fn current(&self) -> Option<String> {
        self.id
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn remember(&self, id: String) {
        *self.id.lock().unwrap_or_else(PoisonError::into_inner) = Some(id);
    }
}

/// Hard ceiling so a wedged `claude` subprocess can't hang the chat forever.
// ponytail: fixed 5-min timeout; lift it if real agentic turns run longer.
const TIMEOUT: Duration = Duration::from_secs(300);

/// The fence tag Claude uses to hand a checkpoint proposal to the harness.
const PROPOSAL_FENCE: &str = "```kiln-experiment";

/// The fence tag Claude uses to ask the harness to run Python in the live kernel.
const RUN_FENCE: &str = "```kiln-run";

/// Appended to every turn so Claude knows the checkpoint convention. This is the
/// CLI-native stand-in for an API tool schema (Ticket 30): instead of a tool
/// call, Claude emits a fenced JSON block that `extract_proposal` picks up.
// ponytail: prompt-driven contract; a real MCP `propose_experiment` tool is the
// upgrade if/when prose-extraction proves too loose.
const KILN_SYSTEM_PROMPT: &str = "\
You run INSIDE Kiln, a desktop app with a LIVE Python kernel (numpy, pandas, \
matplotlib, … already imported-on-demand) and Code/Result/Plot panes. You do NOT \
have a usable terminal here: shell/Bash commands are sandboxed and fail, and the \
human cannot paste code into a terminal. NEVER tell the human to run something \
themselves, NEVER hand back a snippet to copy, and NEVER suggest `uv run`, python, \
or shell commands. To run Python, emit exactly one fenced code block tagged \
`kiln-run` containing the code — Kiln executes it in the live kernel, plots appear \
in the Plot tab, and the cell's stdout, return value, and any error are returned to \
you so you can report the result or fix and re-run. Use `kiln-run` whenever the \
human asks you to plot, compute, run, load, or show anything.\n\n\
You are running inside Kiln, which gates data-science experiments on a structured \
premise review. ONLY when you are proposing a concrete experiment for the human to \
review before running it, end your reply with exactly one fenced code block tagged \
`kiln-experiment` containing a JSON object with: title (string), premise (string), \
look_here (array of strings), and seven slots — validation_strategy, target_definition, \
feature_provenance, preprocessing_fit_scope, data_scope_and_exclusions, \
missing_data_handling, metric_choice. Each slot is \
{\"in_scope\": bool, \"severity\": \"critical\"|\"notable\"|\"fyi\", \"answer\": string}; \
use \"N/A\" with in_scope=false when a slot does not apply. An in-scope slot's answer \
must be non-empty. Do NOT emit the block for ordinary conversation.";

/// The shape of `claude --output-format json` (we only read what we need).
#[derive(Debug, Deserialize)]
struct CliResult {
    is_error: bool,
    result: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

/// A parsed CLI reply: the text plus the session id to resume next turn.
#[derive(Debug)]
struct Reply {
    text: String,
    session_id: Option<String>,
}

/// One newline-delimited object from `--output-format stream-json`. We only need
/// the type tag and, for `stream_event`, the nested event payload.
#[derive(Debug, Deserialize)]
struct StreamLine {
    #[serde(rename = "type")]
    kind: String,
    event: Option<serde_json::Value>,
}

/// Pull the incremental text out of a `content_block_delta` / `text_delta`
/// stream event; `None` for any other event (tool use, thinking, start/stop).
fn delta_text(event: &serde_json::Value) -> Option<&str> {
    if event.get("type")?.as_str()? != "content_block_delta" {
        return None;
    }
    let delta = event.get("delta")?;
    if delta.get("type")?.as_str()? != "text_delta" {
        return None;
    }
    delta.get("text")?.as_str()
}

/// Run one Claude Code turn, continuing `session` so the conversation persists.
///
/// We shell out to the `claude` CLI (`-p --output-format json`) instead of the
/// raw Anthropic API: auth, tools, MCP, and project context all come for free
/// from the user's existing Claude Code install. The prompt is piped over stdin
/// so a message that looks like a flag (e.g. "--continue") can't be misparsed.
pub async fn send(
    prompt: &str,
    session: &ClaudeSession,
    app: &AppHandle,
) -> Result<String, String> {
    let resume = session.current();
    let reply = tokio::time::timeout(TIMEOUT, run(prompt, resume.as_deref(), app))
        .await
        .map_err(|_| format!("claude timed out after {}s", TIMEOUT.as_secs()))??;
    if let Some(id) = reply.session_id {
        session.remember(id);
    }
    Ok(reply.text)
}

async fn run(prompt: &str, resume: Option<&str>, app: &AppHandle) -> Result<Reply, String> {
    let mut command = Command::new("claude");
    command
        .arg("-p")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--append-system-prompt")
        .arg(KILN_SYSTEM_PROMPT);
    if let Some(id) = resume {
        command.arg("--resume").arg(id);
    }
    let mut child = command
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

    // Stream stdout line by line: forward text deltas to the webview as they
    // arrive, and capture the terminal `result` object (full text + session id).
    let stdout = child.stdout.take().expect("stdout is piped");
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    let mut reply: Option<Reply> = None;
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| format!("reading claude stream: {e}"))?
    {
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<StreamLine>(&line) else {
            continue;
        };
        match event.kind.as_str() {
            "stream_event" => {
                if let Some(text) = event.event.as_ref().and_then(delta_text) {
                    let _ = app.emit("chat:delta", text);
                }
            }
            "result" => reply = Some(parse_output(line.as_bytes())?),
            _ => {}
        }
    }

    // ponytail: drain stderr after stdout EOF; claude's stderr is tiny, so this
    // can't deadlock the way a mid-stream read could.
    let mut stderr_buf = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut stderr_buf).await;
    }
    let status = child
        .wait()
        .await
        .map_err(|e| format!("claude failed: {e}"))?;
    if !status.success() {
        return Err(format!(
            "claude exited with {}: {}",
            status,
            stderr_buf.trim()
        ));
    }

    reply.ok_or_else(|| "claude produced no result".to_owned())
}

/// Extract the text reply + session id from `claude --output-format json` stdout.
fn parse_output(stdout: &[u8]) -> Result<Reply, String> {
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
    Ok(Reply {
        text,
        session_id: parsed.session_id,
    })
}

/// Pull the body out of the first fenced block tagged `fence` (e.g.
/// ```` ```kiln-experiment ````), if the reply contains one.
fn extract_fenced<'a>(text: &'a str, fence: &str) -> Option<&'a str> {
    let start = text.find(fence)? + fence.len();
    let after_tag = &text[start..];
    let body_start = after_tag.find('\n')? + 1;
    let body = &after_tag[body_start..];
    let end = body.find("```")?;
    Some(body[..end].trim())
}

/// Pull the JSON body out of a ```` ```kiln-experiment ```` block, if present.
fn extract_block(text: &str) -> Option<&str> {
    extract_fenced(text, PROPOSAL_FENCE)
}

/// Pull the Python source out of a ```` ```kiln-run ```` block, if present. The
/// harness runs it in the live kernel and feeds the output back to Claude.
pub fn extract_run_block(text: &str) -> Option<&str> {
    extract_fenced(text, RUN_FENCE)
}

/// If the reply proposes an experiment, parse and validate it:
/// - `None` — no proposal block (ordinary chat).
/// - `Some(Ok(_))` — a valid proposal ready to gate.
/// - `Some(Err(_))` — a proposal block that failed to parse or validate.
pub fn extract_proposal(text: &str) -> Option<Result<ProposeExperiment, String>> {
    let json = extract_block(text)?;
    let proposal: ProposeExperiment = match serde_json::from_str(json) {
        Ok(p) => p,
        Err(e) => return Some(Err(format!("invalid proposal JSON: {e}"))),
    };
    let errors = proposal.validate();
    if errors.is_empty() {
        Some(Ok(proposal))
    } else {
        let list = errors
            .iter()
            .map(|e| format!("{}: {}", e.slot, e.reason))
            .collect::<Vec<_>>()
            .join(", ");
        Some(Err(list))
    }
}

/// The prose to show in chat with the machine-readable proposal block removed
/// (it is rendered structurally in the premise gate instead).
pub fn strip_proposal_block(text: &str) -> String {
    match text.find(PROPOSAL_FENCE) {
        Some(start) => text[..start].trim().to_owned(),
        None => text.trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_proposal, extract_run_block, parse_output, strip_proposal_block};
    use crate::checkpoint::SAMPLE_JSON;

    fn reply_with_proposal(json: &str) -> String {
        format!("Here's what I propose.\n\n```kiln-experiment\n{json}\n```")
    }

    #[test]
    fn extracts_result_text() {
        let raw = br#"{"type":"result","subtype":"success","is_error":false,"result":"pong","session_id":"x"}"#;
        let reply = parse_output(raw).unwrap();
        assert_eq!(reply.text, "pong");
        assert_eq!(reply.session_id.as_deref(), Some("x"));
    }

    #[test]
    fn surfaces_error_subtype() {
        let raw = br#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"boom"}"#;
        assert_eq!(parse_output(raw).unwrap_err(), "boom");
    }

    #[test]
    fn no_block_means_no_proposal() {
        assert!(extract_proposal("just chatting, no experiment here").is_none());
    }

    #[test]
    fn valid_block_parses() {
        let reply = reply_with_proposal(SAMPLE_JSON);
        let proposal = extract_proposal(&reply)
            .expect("block present")
            .expect("valid");
        assert_eq!(proposal.title, "Predict churn from the first 30 days");
    }

    #[test]
    fn invalid_block_surfaces_error() {
        let bad = SAMPLE_JSON.replace("PR-AUC; classes are imbalanced.", "");
        let reply = reply_with_proposal(&bad);
        let err = extract_proposal(&reply)
            .expect("block present")
            .expect_err("invalid");
        assert!(err.contains("metric_choice"));
    }

    #[test]
    fn session_remembers_id_for_resume() {
        let session = super::ClaudeSession::default();
        assert!(session.current().is_none());
        session.remember("sess-123".to_owned());
        assert_eq!(session.current().as_deref(), Some("sess-123"));
    }

    #[test]
    fn strip_removes_the_block() {
        let reply = reply_with_proposal(SAMPLE_JSON);
        assert_eq!(strip_proposal_block(&reply), "Here's what I propose.");
    }

    #[test]
    fn extracts_run_code() {
        let reply =
            "Sure, plotting that now.\n\n```kiln-run\nimport numpy as np\nprint(np.pi)\n```";
        assert_eq!(
            extract_run_block(reply),
            Some("import numpy as np\nprint(np.pi)")
        );
    }

    #[test]
    fn no_run_block_means_none() {
        assert!(extract_run_block("just chatting, nothing to run").is_none());
    }

    #[test]
    fn run_block_not_confused_with_experiment() {
        // A kiln-experiment proposal is not a kiln-run block.
        let reply = reply_with_proposal(SAMPLE_JSON);
        assert!(extract_run_block(&reply).is_none());
    }
}
