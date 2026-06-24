use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize)]
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
        let stdin = sidecar.take_stdin().expect("sidecar stdin is always piped");
        let stdout = sidecar
            .take_stdout()
            .expect("sidecar stdout is always piped");

        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
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
        let pending_clone = Arc::clone(&pending);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buf = String::new();
            loop {
                buf.clear();
                match reader.read_line(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        match serde_json::from_str::<RpcResponse>(buf.trim()) {
                            Ok(resp) => {
                                if let Some(tx) = pending_clone.lock().await.remove(&resp.id) {
                                    // Ignore send errors — the waiter may have dropped its receiver.
                                    let _ = tx.send(resp);
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
                    Err(_) => break,
                }
            }
            tracing::info!("sidecar reader task exited (stdout closed)");
        });

        Self {
            next_id: AtomicU64::new(1),
            writer_tx,
            pending,
        }
    }

    async fn call<P: Serialize>(
        &self,
        method: &str,
        params: P,
    ) -> Result<serde_json::Value, RpcError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Insert into the pending map BEFORE sending the request so that a fast
        // reply cannot arrive at the reader before the waiter is registered.
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

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
        })?;

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
}
