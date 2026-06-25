# Ticket 22 — Real Claude call behind the `chat` command

**Phase:** 3
**Depends on:** [21](./21-chat-pane.md)
**Blocks:** [30](./30-propose-experiment-schema.md)

> **⚠️ Implemented via the Claude Code CLI, not the Anthropic API.** Kiln shells
> out to `claude -p --output-format json` (prompt piped over stdin) instead of
> POSTing to `api.anthropic.com`. Auth, tools, MCP, and project context come for
> free from the user's Claude Code install — so there is no `reqwest`, no
> `ANTHROPIC_API_KEY` plumbing, and no `.env.example`. The implementation lives
> in `src-tauri/src/claude.rs::send()`. The sections below (reqwest client, API
> key env var, model constant) are **superseded** — kept for historical context.

## Goal

Replace the echo with a real Anthropic API call (model `claude-opus-4-7`). Streaming **not** required for this ticket — wait for the full message. The model gets a single tool: a stub `propose_experiment` that simply returns its args. Ticket 30 will replace the stub with the real schema.

## Why a stub tool now

If the chat surface never sees a tool call land, every later checkpoint ticket has to debug both the model wiring AND the gate UI at once. Get the tool round-trip working with a no-op tool first.

## Files

- Modify: `src-tauri/Cargo.toml` — `reqwest = { version = "0.12", features = ["json", "rustls-tls"] }`.
- Create: `src-tauri/src/claude.rs`.
- Modify: `src-tauri/src/commands.rs` — `chat` now delegates to `claude`.
- Modify: `src-tauri/src/lib.rs` — read `ANTHROPIC_API_KEY` from env, manage `ClaudeClient`.
- Create: `.env.example` documenting the var.
- Modify: `.gitignore` to ensure `.env` is excluded (already covered — verify).

## Steps

- [ ] **1. Failing test.** Add an integration test gated on `ANTHROPIC_API_KEY` (skipped in CI unless the var is present).

- [ ] **2. Minimal client.**

  ```rust
  // src-tauri/src/claude.rs
  use serde::{Deserialize, Serialize};

  pub const MODEL: &str = "claude-opus-4-7";
  pub const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

  #[derive(Debug, Serialize)]
  struct MessagesRequest<'a> {
      model: &'a str,
      max_tokens: u32,
      messages: Vec<UserMessage<'a>>,
      tools: Vec<Tool>,
  }

  #[derive(Debug, Serialize)]
  struct UserMessage<'a> { role: &'a str, content: &'a str }

  #[derive(Debug, Serialize)]
  struct Tool { name: &'static str, description: &'static str, input_schema: serde_json::Value }

  #[derive(Debug, Deserialize)]
  pub struct MessagesResponse { pub content: Vec<ContentBlock> }

  #[derive(Debug, Deserialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  pub enum ContentBlock {
      Text { text: String },
      ToolUse { id: String, name: String, input: serde_json::Value },
  }

  pub struct ClaudeClient { api_key: String, http: reqwest::Client }

  impl ClaudeClient {
      pub fn new(api_key: String) -> Self {
          Self { api_key, http: reqwest::Client::new() }
      }

      pub async fn send(&self, prompt: &str) -> Result<MessagesResponse, reqwest::Error> {
          let body = MessagesRequest {
              model: MODEL,
              max_tokens: 2048,
              messages: vec![UserMessage { role: "user", content: prompt }],
              tools: vec![Tool {
                  name: "propose_experiment",
                  description: "Stub. Replaced in ticket 30.",
                  input_schema: serde_json::json!({"type":"object","additionalProperties":true}),
              }],
          };
          self.http
              .post(ENDPOINT)
              .header("x-api-key", &self.api_key)
              .header("anthropic-version", "2023-06-01")
              .json(&body)
              .send()
              .await?
              .error_for_status()?
              .json::<MessagesResponse>()
              .await
      }
  }
  ```

- [ ] **3. Wire into `commands::chat`.**

  ```rust
  #[tauri::command]
  pub async fn chat(message: String, claude: State<'_, ClaudeClient>) -> Result<String, String> {
      let resp = claude.send(&message).await.map_err(|e| e.to_string())?;
      let mut out = String::new();
      for block in resp.content {
          if let crate::claude::ContentBlock::Text { text } = block {
              out.push_str(&text);
          }
      }
      Ok(out)
  }
  ```

- [ ] **4. Setup in `lib.rs`.**

  ```rust
  let api_key = std::env::var("ANTHROPIC_API_KEY")
      .expect("ANTHROPIC_API_KEY must be set (see .env.example)");
  app.manage(ClaudeClient::new(api_key));
  ```

- [ ] **5. `.env.example`.**

  ```sh
  # Copy to .env (DO NOT COMMIT — see .gitignore). Read by `tauri dev`.
  ANTHROPIC_API_KEY=sk-ant-...
  ```

- [ ] **6. Smoke test.**

  ```sh
  export ANTHROPIC_API_KEY=...   # local dev only
  just dev
  ```

  Send a message in the chat. Confirm a real reply lands.

- [ ] **7. Lint + commit.**

  ```sh
  just lint
  git commit -m "feat(chat): wire chat command to claude-opus-4-7"
  ```

## Acceptance

- Real reply renders in the chat pane.
- `ANTHROPIC_API_KEY` is read from env, never from a file in the repo, and `.env` is gitignored.
- Gitleaks hook would reject a committed key (verify by trying — but `git restore` the test).

## Out of scope

- Streaming — fast-follow.
- Conversation history → API. The MVP sends only the latest user message; a follow-up ticket adds full thread support.
- Tool calls actually doing something — Ticket 30.
