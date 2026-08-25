mod server;

use server::{Navidrome, ServerState};
use tauri::{
    Emitter, LogicalPosition, LogicalSize, Manager, RunEvent, Url, WebviewBuilder, WebviewUrl,
    WindowEvent,
};

const TOOLBAR_HEIGHT: f64 = 60.0;

#[tauri::command]
fn back(app: tauri::AppHandle) {
    if let Some(content) = app.get_webview("content") {
        let _ = content.eval("history.back()");
    }
}

#[tauri::command]
fn forward(app: tauri::AppHandle) {
    if let Some(content) = app.get_webview("content") {
        let _ = content.eval("history.forward()");
    }
}

#[tauri::command]
fn reload(app: tauri::AppHandle) {
    if let Some(content) = app.get_webview("content") {
        let _ = content.reload();
    }
}

#[tauri::command]
fn navigate(app: tauri::AppHandle, url: String) {
    if let Some(content) = app.get_webview("content") {
        if let Ok(url) = Url::parse(&url) {
            let _ = content.navigate(url);
        }
    }
}

/// Enables two-finger swipe-to-navigate (back/forward) in the content webview.
#[cfg(target_os = "macos")]
fn enable_swipe_navigation(app: &tauri::AppHandle) {
    if let Some(content) = app.get_webview("content") {
        let _ = content.with_webview(|webview| unsafe {
            let view: &objc2_web_kit::WKWebView = &*webview.inner().cast();
            view.setAllowsBackForwardNavigationGestures(true);
        });
    }
}

/// Positions the toolbar strip at the top and the content webview below it.
fn layout(app: &tauri::AppHandle) {
    let Some(window) = app.get_window("main") else {
        return;
    };
    let Ok(size) = window.inner_size() else {
        return;
    };
    let Ok(scale) = window.scale_factor() else {
        return;
    };
    let width = size.width as f64 / scale;
    let height = size.height as f64 / scale;

    if let Some(toolbar) = app.get_webview("main") {
        let _ = toolbar.set_auto_resize(false);
        let _ = toolbar.set_position(LogicalPosition::new(0.0, 0.0));
        let _ = toolbar.set_size(LogicalSize::new(width, TOOLBAR_HEIGHT));
    }
    if let Some(content) = app.get_webview("content") {
        let _ = content.set_auto_resize(false);
        let _ = content.set_position(LogicalPosition::new(0.0, TOOLBAR_HEIGHT));
        let _ = content.set_size(LogicalSize::new(width, (height - TOOLBAR_HEIGHT).max(0.0)));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(ServerState::default())
        .invoke_handler(tauri::generate_handler![back, forward, reload, navigate])
        .setup(|app| {
            let handle = app.handle().clone();

            let host = std::env::var("QUINTODROME_HOST")
                .unwrap_or_else(|_| server::DEFAULT_HOST.to_string());
            let port = std::env::var("QUINTODROME_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(server::DEFAULT_PORT);

            let navidrome = Navidrome::new(host, port);
            let navidrome_url = navidrome.url.clone();

            // The primary webview ("main") is the toolbar; the Navidrome content
            // lives in a child webview below it.
            let content = WebviewBuilder::new("content", WebviewUrl::App("index.html".into()))
                .on_navigation({
                    let handle = handle.clone();
                    move |url| {
                        if url.scheme() == "http" || url.scheme() == "https" {
                            if let Some(toolbar) = handle.get_webview("main") {
                                let _ = toolbar.emit("url-changed", url.to_string());
                            }
                        }
                        true
                    }
                });

            if let Some(window) = app.get_window("main") {
                let _ = window.add_child(
                    content,
                    LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
                    LogicalSize::new(800.0, 600.0),
                );
                let handle = handle.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Resized(_) = event {
                        layout(&handle);
                    }
                });
            }

            layout(app.handle());

            #[cfg(target_os = "macos")]
            enable_swipe_navigation(app.handle());

            if let Some(toolbar) = app.get_webview("main") {
                let _ = toolbar.emit("url-changed", &navidrome_url);
            }

            // Detect/start Navidrome off the main thread, then load it.
            let content_url = navidrome_url.clone();
            std::thread::spawn(move || {
                if let Err(err) = server::ensure_running(&handle, &navidrome) {
                    eprintln!("quintodrome: {err}");
                    if let Some(content) = handle.get_webview("content") {
                        let message = serde_json::to_string(&err.to_string()).unwrap();
                        let _ = content.eval(&format!(
                            "document.getElementById('spinner').hidden = true;\
                             document.getElementById('status').hidden = true;\
                             var e = document.getElementById('error');\
                             e.hidden = false;\
                             e.textContent = 'Failed to start Navidrome: ' + {message};"
                        ));
                    }
                    return;
                }

                if let Ok(url) = Url::parse(&content_url) {
                    if let Some(content) = handle.get_webview("content") {
                        let _ = content.navigate(url);
                    }
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
