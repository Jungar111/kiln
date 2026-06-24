pub mod commands;
pub mod sidecar;
pub mod sidecar_client;

use tauri::Manager as _;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let repo_root = std::env::current_dir().expect("cwd");
                let mut sidecar = sidecar::Sidecar::spawn(&repo_root)
                    .await
                    .expect("spawn sidecar");
                let client = sidecar_client::SidecarClient::attach(&mut sidecar);
                handle.manage(client);
                handle.manage(sidecar);
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::ping, commands::execute])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
