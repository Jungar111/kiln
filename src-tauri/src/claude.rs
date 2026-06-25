use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

use crate::checkpoint::ProposeExperiment;

/// Hard ceiling so a wedged `claude` subprocess can't hang the chat forever.
// ponytail: fixed 5-min timeout; lift it if real agentic turns run longer.
const TIMEOUT: Duration = Duration::from_secs(300);

/// The fence tag Claude uses to hand a checkpoint proposal to the harness.
const PROPOSAL_FENCE: &str = "```kiln-experiment";

/// Appended to every turn so Claude knows the checkpoint convention. This is the
/// CLI-native stand-in for an API tool schema (Ticket 30): instead of a tool
/// call, Claude emits a fenced JSON block that `extract_proposal` picks up.
// ponytail: prompt-driven contract; a real MCP `propose_experiment` tool is the
// upgrade if/when prose-extraction proves too loose.
const KILN_SYSTEM_PROMPT: &str = "\
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
        .arg("--append-system-prompt")
        .arg(KILN_SYSTEM_PROMPT)
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

/// Pull the JSON body out of a ```` ```kiln-experiment ```` fenced block, if the
/// reply contains one.
fn extract_block(text: &str) -> Option<&str> {
    let start = text.find(PROPOSAL_FENCE)? + PROPOSAL_FENCE.len();
    let after_tag = &text[start..];
    let body_start = after_tag.find('\n')? + 1;
    let body = &after_tag[body_start..];
    let end = body.find("```")?;
    Some(body[..end].trim())
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
    use super::{extract_proposal, parse_output, strip_proposal_block};
    use crate::checkpoint::SAMPLE_JSON;

    fn reply_with_proposal(json: &str) -> String {
        format!("Here's what I propose.\n\n```kiln-experiment\n{json}\n```")
    }

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
    fn strip_removes_the_block() {
        let reply = reply_with_proposal(SAMPLE_JSON);
        assert_eq!(strip_proposal_block(&reply), "Here's what I propose.");
    }
}
