use tokio::process::Child;

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
        let child = tokio::process::Command::new("uv")
            .arg("run")
            .arg("--directory")
            .arg(repo_root.join("sidecar"))
            .arg("kiln-sidecar")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        Ok(Self { child })
    }

    pub fn take_stdin(&mut self) -> Option<tokio::process::ChildStdin> {
        self.child.stdin.take()
    }

    pub fn take_stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        self.child.stdout.take()
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
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[tokio::test]
    async fn spawn_and_shutdown_round_trip() {
        let sidecar = Sidecar::spawn(&repo_root()).await.expect("spawn");
        sidecar.shutdown().await.expect("shutdown");
    }
}
