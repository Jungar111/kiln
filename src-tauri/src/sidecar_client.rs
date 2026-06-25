use serde::{Deserialize, Serialize};

use crate::checkpoint::ProposeExperiment;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{oneshot, Mutex};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

/// One MIME rendering of a kernel `display_data` / `execute_result` bundle
/// (e.g. `image/png`, `text/html`). Small enough to ride the control plane;
/// DataFrames never do — they stay on the direct Arrow path. The webview's
/// plot panel (Ticket 51) picks the highest-fidelity rendering it can show.
#[derive(Debug, Deserialize, Serialize)]
pub struct Display {
    pub mime: String,
    pub payload: String,
    /// Opaque per-MIME metadata from the kernel; passed straight through.
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DfHandle {
    pub handle: String,
    pub rows: u64,
    pub cols: u64,
    pub schema: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecuteResponse {
    pub status: String,
    pub stdout: String,
    pub value: Option<String>,
    pub traceback: Option<String>,
    pub ephemeral: bool,
    // Present when the cell evaluated to a DataFrame. Bytes are paged directly
    // webview↔sidecar over Arrow IPC; only this handle crosses the Rust IPC.
    #[serde(default)]
    pub df: Option<DfHandle>,
    /// Rich MIME bundles the kernel emitted, minus the DataFrame handle MIME.
    #[serde(default)]
    pub displays: Vec<Display>,
}

/// Shared state guarded by a single Mutex to prevent races between
/// `call()` inserting a new waiter and the reader draining on EOF.
/// Invariant: once `closed` is `true`, no new waiters are ever inserted.
struct PendingState {
    waiters: HashMap<u64, oneshot::Sender<Result<RpcResponse, RpcError>>>,
    closed: bool,
}

type Pending = Arc<Mutex<PendingState>>;

pub struct SidecarClient {
    next_id: AtomicU64,
    writer_tx: tokio::sync::mpsc::Sender<String>,
    pending: Pending,
}

impl SidecarClient {
    /// Attach a client to a sidecar. The optional `exit_tx` is notified (with
    /// `()`) when the reader task detects EOF — callers can use this to emit
    /// lifecycle events without importing Tauri into this module.
    pub fn attach_with_exit_notifier(
        sidecar: &mut crate::sidecar::Sidecar,
        exit_tx: Option<oneshot::Sender<()>>,
    ) -> Self {
        let stdin = sidecar.take_stdin().expect("sidecar stdin is always piped");
        let stdout = sidecar
            .take_stdout()
            .expect("sidecar stdout is always piped");
        // Drain stderr to prevent the pipe buffer from filling and blocking the
        // sidecar process. Log any non-empty lines at WARN level.
        if let Some(stderr) = sidecar.take_stderr() {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut buf = String::new();
                loop {
                    buf.clear();
                    match reader.read_line(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            let line = buf.trim();
                            if !line.is_empty() {
                                tracing::warn!(target: "kiln_sidecar_stderr", "{}", line);
                            }
                        }
                    }
                }
                tracing::debug!("sidecar stderr drain task exited");
            });
        }

        let pending: Pending = Arc::new(Mutex::new(PendingState {
            waiters: HashMap::new(),
            closed: false,
        }));
        let (writer_tx, mut writer_rx) = tokio::sync::mpsc::channel::<String>(64);

        // Writer task — owns stdin, drains the mpsc channel.
        let mut stdin = stdin;
        tokio::spawn(async move {
            while let Some(line) = writer_rx.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.write_all(b"\n").await.is_err() {
                    break;
                }
                let _ = stdin.flush().await;
            }
            tracing::debug!("sidecar writer task exited");
        });

        // Reader task — owns stdout; routes responses by id into the pending map.
        // On EOF or read error: set closed = true and drain every pending waiter
        // with a -32099 error so they resolve immediately instead of hanging.
        let pending_clone = Arc::clone(&pending);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buf = String::new();
            loop {
                buf.clear();
                match reader.read_line(&mut buf).await {
                    Ok(0) => {
                        // EOF — process has exited.
                        tracing::info!("sidecar reader task: EOF (stdout closed)");
                        break;
                    }
                    Ok(_) => {
                        match serde_json::from_str::<RpcResponse>(buf.trim()) {
                            Ok(resp) => {
                                if let Some(tx) =
                                    pending_clone.lock().await.waiters.remove(&resp.id)
                                {
                                    // Ignore send errors — the waiter may have dropped its receiver.
                                    let _ = tx.send(Ok(resp));
                                } else {
                                    tracing::debug!(
                                        id = resp.id,
                                        "sidecar reader: unroutable reply (no pending waiter)"
                                    );
                                }
                            }
                            Err(_) => {
                                // Skip unparseable lines (e.g. startup noise).
                                tracing::debug!(
                                    line = buf.trim(),
                                    "sidecar reader: skipping non-JSON-RPC line"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "sidecar reader: read error; draining waiters");
                        break;
                    }
                }
            }

            // EOF or read error: drain all pending waiters under a single lock
            // acquisition so there is no window between closing and a new insert.
            {
                let mut state = pending_clone.lock().await;
                state.closed = true;
                let error = RpcError {
                    code: -32099,
                    message: "sidecar exited".into(),
                };
                for (_, tx) in state.waiters.drain() {
                    let _ = tx.send(Err(error.clone()));
                }
            }

            // Notify the lifecycle event emitter (lib.rs), if one was provided.
            if let Some(tx) = exit_tx {
                let _ = tx.send(());
            }

            tracing::info!("sidecar reader task exited");
        });

        Self {
            next_id: AtomicU64::new(1),
            writer_tx,
            pending,
        }
    }

    /// Convenience constructor for tests — no exit notifier.
    pub fn attach(sidecar: &mut crate::sidecar::Sidecar) -> Self {
        Self::attach_with_exit_notifier(sidecar, None)
    }

    async fn call<P: Serialize>(
        &self,
        method: &str,
        params: P,
    ) -> Result<serde_json::Value, RpcError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Check `closed` and insert atomically under the same lock.
        // This prevents the race where the reader drains and then a new
        // call inserts a waiter that will never be fulfilled.
        let (tx, rx) = oneshot::channel();
        {
            let mut state = self.pending.lock().await;
            if state.closed {
                return Err(RpcError {
                    code: -32099,
                    message: "sidecar exited".into(),
                });
            }
            state.waiters.insert(id, tx);
        }

        let request = RpcRequest {
            jsonrpc: "2.0",
            id,
            method,
            params,
        };
        let body = serde_json::to_string(&request)
            .expect("serialise: RpcRequest<&str, P: Serialize> is always valid JSON");

        self.writer_tx.send(body).await.map_err(|_| RpcError {
            code: -32099,
            message: "writer channel closed".into(),
        })?;

        let resp = rx.await.map_err(|_| RpcError {
            code: -32099,
            message: "reader channel closed before reply arrived".into(),
        })??;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }

    pub async fn ping(&self) -> Result<String, RpcError> {
        let value = self.call("ping", serde_json::json!({})).await?;
        Ok(value.as_str().unwrap_or_default().to_string())
    }

    pub async fn execute(&self, code: &str, ephemeral: bool) -> Result<ExecuteResponse, RpcError> {
        let value = self
            .call(
                "execute",
                serde_json::json!({ "code": code, "ephemeral": ephemeral }),
            )
            .await?;
        serde_json::from_value::<ExecuteResponse>(value).map_err(|e| RpcError {
            code: -32700,
            message: e.to_string(),
        })
    }

    /// Persist the approved checkpoint's declared decisions as MLflow run tags
    /// and return the new run id.
    pub async fn approve_checkpoint(
        &self,
        proposal: &ProposeExperiment,
    ) -> Result<String, RpcError> {
        let value = self
            .call(
                "approve_checkpoint",
                serde_json::json!({ "proposal": proposal }),
            )
            .await?;
        value
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| RpcError {
                code: -32700,
                message: "approve_checkpoint reply missing run_id".into(),
            })
    }

    /// Record the results-gate verdict (keep | kill | iterate) and finish the run.
    pub async fn close_run(&self, run_id: &str, verdict: &str) -> Result<(), RpcError> {
        self.call(
            "close_run",
            serde_json::json!({ "run_id": run_id, "verdict": verdict }),
        )
        .await?;
        Ok(())
    }

    /// Fetch recent MLflow runs (params, metrics, and `kiln.slot.*` decisions).
    /// The payload is owned by the sidecar/webview contract, so we pass the JSON
    /// array straight through rather than re-modelling it in Rust.
    pub async fn list_runs(&self, limit: u32) -> Result<serde_json::Value, RpcError> {
        self.call("list_runs", serde_json::json!({ "limit": limit }))
            .await
    }

    /// Port of the in-kernel Arrow HTTP server. The webview fetches DataFrame
    /// pages directly from it — bytes never cross this control plane.
    pub async fn arrow_port(&self) -> Result<u32, RpcError> {
        let value = self.call("arrow_port", serde_json::json!({})).await?;
        value
            .get("port")
            .and_then(serde_json::Value::as_u64)
            .map(|port| port as u32)
            .ok_or_else(|| RpcError {
                code: -32700,
                message: "arrow_port reply missing port".into(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
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

    /// After the sidecar process is killed, any subsequent `execute` call must
    /// resolve to `Err(RpcError { code: -32099, .. })` within 1 second and
    /// must NOT hang. This validates the race-free EOF drain: the reader sets
    /// `closed = true` and drains pending waiters under one lock, and `call()`
    /// checks `closed` before inserting — so there is no window for a new
    /// waiter to be stranded.
    #[tokio::test]
    async fn post_kill_execute_resolves_with_error() {
        let (exit_tx, exit_rx) = oneshot::channel::<()>();

        let mut sidecar = crate::sidecar::Sidecar::spawn(&repo_root()).await.unwrap();
        let client = SidecarClient::attach_with_exit_notifier(&mut sidecar, Some(exit_tx));

        // Confirm the sidecar is live before killing.
        let pong = client.ping().await.expect("ping before kill");
        assert_eq!(pong, "pong");

        // Kill the process.
        sidecar.shutdown().await.unwrap();

        // Wait for the reader to detect EOF (reliable: waits for the notifier
        // rather than sleeping a fixed amount of time).
        tokio::time::timeout(tokio::time::Duration::from_secs(5), exit_rx)
            .await
            .expect("reader did not detect EOF within 5s")
            .expect("exit notifier dropped without sending");

        // execute() must fail fast with -32099, NOT hang.
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(1),
            client.execute("1+1", false),
        )
        .await;

        match result {
            Ok(Err(rpc_err)) => assert_eq!(
                rpc_err.code, -32099,
                "expected -32099 but got {}",
                rpc_err.code
            ),
            Ok(Ok(_)) => panic!("expected an error after kill, got success"),
            Err(_) => panic!("execute() hung for > 1s after kill"),
        }
    }
}
