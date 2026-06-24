# Ticket 10 — Spawn the sidecar from the Rust core

**Phase:** 2 (Rust core ↔ sidecar)
**Depends on:** [04](./04-execute-roundtrip.md)
**Blocks:** [11](./11-control-client.md)

## Goal

Implement a `Sidecar` struct in `src-tauri/` that spawns `kiln-sidecar` as a child process via `uv run --directory sidecar`, captures stdio, and shuts it down on `Drop`. No JSON-RPC client yet — Ticket 11 layers that on top.

## Files

- Modify: `src-tauri/Cargo.toml` (add `tokio` with `process`, `io-util`, `macros`, `rt-multi-thread`).
- Create: `src-tauri/src/sidecar.rs`.
- Modify: `src-tauri/src/lib.rs` (mod the new module + a `cargo test` smoke).

## Steps

- [ ] **1. Add dependencies.**

  In `src-tauri/Cargo.toml`:

  ```toml
  [dependencies]
  tauri = { version = "2", features = [] }
  tauri-plugin-opener = "2"
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  tokio = { version = "1", features = ["process", "io-util", "macros", "rt-multi-thread", "sync", "time"] }
  thiserror = "2"
  tracing = "0.1"
  ```

- [ ] **2. Failing test.**

  ```rust
  // src-tauri/src/sidecar.rs
  use tokio::process::Child;
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

  #[derive(Debug, thiserror::Error)]
  pub enum SidecarError {
      #[error("failed to spawn sidecar: {0}")]
      Spawn(#[from] std::io::Error),
      #[error("sidecar exited unexpectedly")]
      Exited,
  }

  pub struct Sidecar {
      child: Child,
  }

  impl Sidecar {
      pub async fn spawn(repo_root: &std::path::Path) -> Result<Self, SidecarError> {
          let mut command = tokio::process::Command::new("uv");
          command
              .arg("run")
              .arg("--directory")
              .arg(repo_root.join("sidecar"))
              .arg("kiln-sidecar")
              .stdin(std::process::Stdio::piped())
              .stdout(std::process::Stdio::piped())
              .stderr(std::process::Stdio::piped());
          let child = command.spawn()?;
          Ok(Self { child })
      }

      pub async fn shutdown(mut self) -> Result<(), SidecarError> {
          self.child.kill().await?;
          Ok(())
      }
  }

  #[cfg(test)]
  mod tests {
      use super::*;
      use std::path::PathBuf;

      // Walks up from CARGO_MANIFEST_DIR (src-tauri/) to the repo root.
      fn repo_root() -> PathBuf {
          PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
      }

      #[tokio::test]
      async fn spawn_and_shutdown_round_trip() {
          let sidecar = Sidecar::spawn(&repo_root()).await.expect("spawn");
          sidecar.shutdown().await.expect("shutdown");
      }
  }
  ```

- [ ] **3. Wire into `lib.rs`.**

  ```rust
  // src-tauri/src/lib.rs — add at the top
  pub mod sidecar;
  ```

- [ ] **4. Run cargo test.**

  ```sh
  just test-rs
  ```

- [ ] **5. Lint.**

  ```sh
  just lint-rs
  ```

- [ ] **6. Commit.**

  ```sh
  git commit -m "feat(core): spawn the kiln-sidecar child process via tokio"
  ```

## Acceptance

- `just test-rs` green (the spawn-and-shutdown test passes).
- `just lint-rs` green (clippy clean, no `#[allow(...)]` introduced).
- No clippy `allow` comments without justification.

## Out of scope

- JSON-RPC framing — Ticket 11.
- Crash detection / restart — Ticket 13.
- Bundling `uv` itself — out of MVP; we trust the dev environment.
