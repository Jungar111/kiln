use tokio::process::Child;

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("failed to spawn sidecar: {0}")]
    Spawn(#[from] std::io::Error),
}

pub struct Sidecar {
    child: Child,
    /// Process group id of the child, captured at spawn. With `process_group(0)`
    /// the child is its own group leader, so `pgid == child.id()`. We store it so
    /// `Drop` can group-kill even after the child's stdio handles are taken (and
    /// after `child.id()` would return `None` once the child has been reaped).
    #[cfg(unix)]
    pgid: Option<i32>,
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
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        // Put the child in its own process group (pgid = child pid) so killing
        // the group propagates to grandchildren (e.g. the Python process spawned
        // by `uv run`). `process_group` is a Unix-only API.
        // ponytail: Phase 9 (ticket 80) — Windows needs a job-object equivalent
        // to kill the `uv`-spawned grandchild on quit; MVP targets macOS.
        #[cfg(unix)]
        command.process_group(0);

        let child = command.spawn()?;

        #[cfg(unix)]
        let pgid = child.id().map(|pid| pid as i32);

        Ok(Self {
            child,
            #[cfg(unix)]
            pgid,
        })
    }

    pub fn take_stdin(&mut self) -> Option<tokio::process::ChildStdin> {
        self.child.stdin.take()
    }

    pub fn take_stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        self.child.stdout.take()
    }

    pub fn take_stderr(&mut self) -> Option<tokio::process::ChildStderr> {
        self.child.stderr.take()
    }

    /// The process group id of the sidecar (Unix only). Exposed for tests that
    /// assert the group is killed on drop.
    #[cfg(unix)]
    pub fn pgid(&self) -> Option<i32> {
        self.pgid
    }

    /// Kill the sidecar and the entire process group it leads.
    ///
    /// `uv run` spawns a child Python process before exec-ing; killing only the
    /// `uv` pid leaves the Python grandchild alive with the stdout pipe open,
    /// which would cause the reader task to hang. Sending SIGKILL to the whole
    /// process group (`pgid = child.id()`) kills every process in the group.
    pub async fn shutdown(mut self) -> Result<(), SidecarError> {
        // Group-kill first (synchronous syscall), then reap the `uv` child.
        // `Drop` will also run when `self` is dropped at the end of this method;
        // the second group-kill there just returns ESRCH, which is ignored.
        #[cfg(unix)]
        kill_process_group(self.pgid);
        // Reap the direct `uv` child to avoid a zombie. On non-Unix this is the
        // only kill path (kill_on_drop covers the drop case).
        self.child.kill().await?;
        Ok(())
    }
}

/// SIGKILL an entire process group by pgid. Synchronous (no `.await`), so it is
/// safe to call from `Drop`. ESRCH (group already gone) is treated as success.
#[cfg(unix)]
fn kill_process_group(pgid: Option<i32>) {
    use nix::sys::signal::{killpg, Signal};
    use nix::unistd::Pid;
    if let Some(pgid) = pgid {
        // Ignore the error: if the group is already dead, killpg returns ESRCH,
        // which we treat as success.
        let _ = killpg(Pid::from_raw(pgid), Signal::SIGKILL);
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        // In the real app the `Sidecar` lives in Tauri managed state and is
        // never `shutdown()` — on quit it is simply dropped. `kill_on_drop(true)`
        // only SIGKILLs the direct `uv` child, which would leak the Python
        // grandchild that `uv run` forks. Group-kill here closes that leak.
        // `killpg` is a synchronous syscall, so Drop can do it without an async
        // runtime. If `shutdown()` already ran, the group is gone and killpg
        // returns ESRCH, which we ignore.
        #[cfg(unix)]
        kill_process_group(self.pgid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

    /// Regression: in the real app the `Sidecar` is held in Tauri managed state
    /// and dropped (never `shutdown()`) on quit. `Drop` must group-kill so the
    /// Python grandchild that `uv run` forks does not leak. We assert that after
    /// dropping (without calling `shutdown`), the process group is gone — i.e.
    /// signalling it returns ESRCH (no such process group).
    #[cfg(unix)]
    #[tokio::test]
    async fn drop_kills_process_group() {
        use nix::sys::signal::killpg;
        use nix::unistd::Pid;

        let sidecar = Sidecar::spawn(&repo_root()).await.expect("spawn");
        let pgid = sidecar.pgid().expect("pgid captured at spawn");

        // Signal 0 probes liveness without killing. The group must be alive now.
        assert!(
            killpg(Pid::from_raw(pgid), None).is_ok(),
            "process group should be alive before drop"
        );

        // Drop WITHOUT shutdown — exactly what happens on normal app quit.
        drop(sidecar);

        // Give the kernel a brief moment to tear the group down.
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // The group must now be gone: killpg returns ESRCH.
        let err = killpg(Pid::from_raw(pgid), None).expect_err("group should be dead after drop");
        assert_eq!(
            err,
            nix::errno::Errno::ESRCH,
            "expected ESRCH (no such process group), got {err:?}"
        );
    }

    /// Verify that killing the process group closes the stdout pipe fast.
    /// This is the low-level counterpart of the higher-level EOF-drain test in
    /// `sidecar_client::tests::post_kill_execute_resolves_with_error`.
    #[tokio::test]
    async fn kill_closes_stdout() {
        let mut sidecar = Sidecar::spawn(&repo_root()).await.expect("spawn");
        let mut stdin = sidecar.take_stdin().expect("stdin");
        let stdout = sidecar.take_stdout().expect("stdout");
        let stderr = sidecar.take_stderr().expect("stderr");

        // Drain stderr so it never fills the pipe buffer and stalls the process.
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut buf = String::new();
            loop {
                buf.clear();
                if matches!(reader.read_line(&mut buf).await, Ok(0) | Err(_)) {
                    break;
                }
            }
        });

        // Confirm the sidecar is alive.
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\",\"params\":{}}\n")
            .await
            .expect("write ping");
        stdin.flush().await.expect("flush");

        let mut reader = BufReader::new(stdout);
        let mut buf = String::new();
        reader.read_line(&mut buf).await.expect("read pong");
        assert!(buf.contains("pong"), "expected pong, got: {buf}");

        // Kill the process group; stdout pipe must close promptly.
        sidecar.shutdown().await.expect("shutdown");

        buf.clear();
        let n = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            reader.read_line(&mut buf),
        )
        .await
        .expect("EOF within 5s")
        .expect("no read error");
        assert_eq!(n, 0, "expected EOF (0 bytes) after kill, got {n} bytes");
    }
}
