# Ticket 11 — JSON-RPC client to the sidecar

**Phase:** 2
**Depends on:** [03](./03-jsonrpc-control.md), [10](./10-sidecar-process-supervisor.md)
**Blocks:** [12](./12-tauri-execute-command.md)

## Goal

Add a `Client` to `src-tauri/src/sidecar.rs` (or a sibling module) that owns the sidecar's stdin/stdout, multiplexes requests over the single pipe with monotonic IDs, and exposes `ping() -> "pong"` and `execute(code: &str, ephemeral: bool) -> ExecuteResponse`.

## Files

- Modify: `src-tauri/src/sidecar.rs`.
- Modify: `src-tauri/Cargo.toml` (add `tokio` features already covered in Ticket 10 should suffice — add `serde_json` is present).
- Create: `src-tauri/src/sidecar_client.rs`.

## Steps

- [ ] **1. Failing test.**

  ```rust
  // src-tauri/src/sidecar_client.rs (bottom of file)
  #[cfg(test)]
  mod tests {
      use super::*;
      use std::path::PathBuf;

      fn repo_root() -> PathBuf {
          PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
      }

      #[tokio::test]
      async fn ping_returns_pong() {
          let mut sidecar = crate::sidecar::Sidecar::spawn(&repo_root()).await.unwrap();
          let client = SidecarClient::attach(&mut sidecar);
          let reply = client.ping().await.expect("ping");
          assert_eq!(reply, "pong");
          sidecar.shutdown().await.unwrap();
      }

      #[tokio::test]
      async fn execute_returns_value() {
          let mut sidecar = crate::sidecar::Sidecar::spawn(&repo_root()).await.unwrap();
          let client = SidecarClient::attach(&mut sidecar);
          let reply = client.execute("1+1", false).await.expect("execute");
          assert_eq!(reply.value.as_deref(), Some("2"));
          sidecar.shutdown().await.unwrap();
      }
  }
  ```

- [ ] **2. Run — fails (module doesn't exist).**

- [ ] **3. Implement the client.**

  Skeleton (fill in following the multiplexer pattern below):

  ```rust
  use serde::{Deserialize, Serialize};
  use std::sync::atomic::{AtomicU64, Ordering};
  use std::sync::Arc;
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
  use tokio::sync::{oneshot, Mutex};
  use std::collections::HashMap;

  #[derive(Debug, Serialize)]
  struct RpcRequest<'a, P: Serialize> {
      jsonrpc: &'static str,
      id: u64,
      method: &'a str,
      params: P,
  }

  #[derive(Debug, Deserialize)]
  struct RpcResponse {
      id: u64,
      result: Option<serde_json::Value>,
      error: Option<RpcError>,
  }

  #[derive(Debug, Deserialize)]
  pub struct RpcError {
      pub code: i64,
      pub message: String,
  }

  #[derive(Debug, Deserialize)]
  pub struct ExecuteResponse {
      pub status: String,
      pub stdout: String,
      pub value: Option<String>,
      pub traceback: Option<String>,
      pub ephemeral: bool,
  }

  type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResponse>>>>;

  pub struct SidecarClient {
      next_id: AtomicU64,
      writer_tx: tokio::sync::mpsc::Sender<String>,
      pending: Pending,
  }

  impl SidecarClient {
      pub fn attach(sidecar: &mut crate::sidecar::Sidecar) -> Self {
          let stdin = sidecar.take_stdin().expect("sidecar stdin");
          let stdout = sidecar.take_stdout().expect("sidecar stdout");

          let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
          let (writer_tx, mut writer_rx) = tokio::sync::mpsc::channel::<String>(64);

          // Writer task — owns stdin.
          let mut stdin = stdin;
          tokio::spawn(async move {
              while let Some(line) = writer_rx.recv().await {
                  if stdin.write_all(line.as_bytes()).await.is_err() { break; }
                  if stdin.write_all(b"\n").await.is_err() { break; }
                  let _ = stdin.flush().await;
              }
          });

          // Reader task — owns stdout; routes by id into the pending map.
          let pending_clone = Arc::clone(&pending);
          tokio::spawn(async move {
              let mut reader = BufReader::new(stdout);
              let mut buf = String::new();
              loop {
                  buf.clear();
                  match reader.read_line(&mut buf).await {
                      Ok(0) => break,
                      Ok(_) => {
                          if let Ok(resp) = serde_json::from_str::<RpcResponse>(buf.trim()) {
                              if let Some(tx) = pending_clone.lock().await.remove(&resp.id) {
                                  let _ = tx.send(resp);
                              }
                          }
                      }
                      Err(_) => break,
                  }
              }
          });

          Self { next_id: AtomicU64::new(1), writer_tx, pending }
      }

      async fn call<P: Serialize>(&self, method: &str, params: P) -> Result<serde_json::Value, RpcError> {
          let id = self.next_id.fetch_add(1, Ordering::Relaxed);
          let (tx, rx) = oneshot::channel();
          self.pending.lock().await.insert(id, tx);
          let request = RpcRequest { jsonrpc: "2.0", id, method, params };
          let body = serde_json::to_string(&request).expect("serialise");
          self.writer_tx.send(body).await.map_err(|_| RpcError { code: -32099, message: "writer closed".into() })?;
          let resp = rx.await.map_err(|_| RpcError { code: -32099, message: "reader closed".into() })?;
          if let Some(err) = resp.error { return Err(err); }
          Ok(resp.result.unwrap_or(serde_json::Value::Null))
      }

      pub async fn ping(&self) -> Result<String, RpcError> {
          let value = self.call("ping", serde_json::json!({})).await?;
          Ok(value.as_str().unwrap_or_default().to_string())
      }

      pub async fn execute(&self, code: &str, ephemeral: bool) -> Result<ExecuteResponse, RpcError> {
          let value = self.call("execute", serde_json::json!({ "code": code, "ephemeral": ephemeral })).await?;
          serde_json::from_value::<ExecuteResponse>(value).map_err(|e| RpcError { code: -32700, message: e.to_string() })
      }
  }
  ```

- [ ] **4. Add the `take_stdin`/`take_stdout` accessors to `Sidecar`.**

  ```rust
  // sidecar.rs — methods on Sidecar
  pub fn take_stdin(&mut self) -> Option<tokio::process::ChildStdin> { self.child.stdin.take() }
  pub fn take_stdout(&mut self) -> Option<tokio::process::ChildStdout> { self.child.stdout.take() }
  ```

- [ ] **5. Wire the module.**

  ```rust
  // lib.rs
  pub mod sidecar;
  pub mod sidecar_client;
  ```

- [ ] **6. Run tests, lint, commit.**

  ```sh
  just test-rs && just lint-rs
  git commit -m "feat(core): multiplexing JSON-RPC client over sidecar stdio"
  ```

## Acceptance

- Both `tokio::test` cases green.
- No `unwrap()` in non-test code (use `?`).
- Clippy clean.

## Out of scope

- Backpressure on the writer mpsc beyond bounded(64) — the dev workload is bounded by user clicks.
- Reconnect on sidecar crash — Ticket 13 handles state events.
