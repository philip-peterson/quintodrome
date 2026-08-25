mod server;

use server::{Navidrome, ServerState};
use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(ServerState::default())
        .setup(|app| {
            let handle = app.handle().clone();

            let host = std::env::var("QUINTODROME_HOST")
                .unwrap_or_else(|_| server::DEFAULT_HOST.to_string());
            let port = std::env::var("QUINTODROME_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(server::DEFAULT_PORT);

            let navidrome = Navidrome::new(host, port);
            let url = navidrome.url.clone();

            // Detect/start Navidrome off the main thread, then point the
            // window at it once it is ready.
            std::thread::spawn(move || {
                if let Err(err) = server::ensure_running(&handle, &navidrome) {
                    eprintln!("quintodrome: {err}");
                    let message = serde_json::to_string(&err.to_string()).unwrap();
                    if let Some(window) = handle.get_webview_window("main") {
                        let _ = window.eval(&format!(
                            "document.getElementById('status').hidden = true;\
                             var e = document.getElementById('error');\
                             e.hidden = false;\
                             e.textContent = 'Failed to start Navidrome: ' + {message};"
                        ));
                    }
                    return;
                }

                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.eval(&format!("window.location.replace('{url}')"));
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Quintodrome")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                server::shutdown(&app.state::<ServerState>());
            }
        });
}
