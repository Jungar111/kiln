pub mod commands;
pub mod sidecar;
pub mod sidecar_client;

use tauri::{Emitter as _, Manager as _};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Notify the webview that we are about to spawn the sidecar.
                let _ = handle.emit("sidecar:starting", ());

                let repo_root = std::env::current_dir().expect("cwd");
                let mut sidecar = sidecar::Sidecar::spawn(&repo_root)
                    .await
                    .expect("spawn sidecar");

                // Wire up an exit notifier so we can emit `sidecar:exited` when
                // the reader task detects EOF. We use option (b) from the ticket:
                // piggyback on the reader's EOF detection via a oneshot channel.
                // This keeps `sidecar_client.rs` entirely Tauri-free and testable.
                let (exit_tx, exit_rx) = tokio::sync::oneshot::channel::<()>();
                let client = sidecar_client::SidecarClient::attach_with_exit_notifier(
                    &mut sidecar,
                    Some(exit_tx),
                );

                handle.manage(client);
                handle.manage(sidecar);

                // Notify the webview that the sidecar is ready. Emitting AFTER
                // `manage(client)` means the webview can immediately call commands
                // without racing the startup window.
                let _ = handle.emit("sidecar:ready", ());

                // Await the exit notifier in a separate task so the setup task
                // can return (and the main event loop can start). When the reader
                // sees EOF it fires `exit_tx`; we then emit `sidecar:exited`.
                let exit_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    // Ignore channel errors: if the sidecar never exits during the
                    // app's lifetime, `exit_tx` is dropped when the client is freed,
                    // which sends an Err to exit_rx — we just ignore it.
                    if exit_rx.await.is_ok() {
                        let _ = exit_handle.emit("sidecar:exited", ());
                    }
                });
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::ping, commands::execute])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
