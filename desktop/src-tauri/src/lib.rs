mod server;

use server::{Navidrome, ServerState};
use tauri::{
    Emitter, LogicalPosition, LogicalSize, Manager, RunEvent, Url, WebviewBuilder, WebviewUrl,
    WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

const TOOLBAR_HEIGHT: f64 = 60.0;
const DEFAULT_PUBLIC_URL: &str = "http://quintodrome";

/// Injected into the content webview to make Cmd+C/X/V/A/Z work. WKWebView
/// doesn't reliably route these through the app menu's key equivalents, so we
/// handle them at the DOM level (and read the clipboard natively for paste).
const EDIT_SHORTCUTS_SCRIPT: &str = r#"
(function () {
  if (window.__quintodromeShortcuts) return;
  window.__quintodromeShortcuts = true;
  document.addEventListener('keydown', function (e) {
    if (!e.metaKey || e.ctrlKey || e.altKey) return;
    var k = (e.key || '').toLowerCase();
    var cmd = null;
    if (k === 'x') cmd = 'cut';
    else if (k === 'c') cmd = 'copy';
    else if (k === 'a') cmd = 'selectAll';
    else if (k === 'z') cmd = e.shiftKey ? 'redo' : 'undo';
    else if (k === 'v') {
      e.preventDefault();
      window.__TAURI_INTERNALS__.invoke('read_clipboard').then(function (text) {
        if (text) document.execCommand('insertText', false, text);
      }).catch(function () {});
      return;
    }
    if (!cmd) return;
    e.preventDefault();
    try { document.execCommand(cmd); } catch (err) {}
  }, true);
})();
"#;

/// Maps the internal Navidrome origin (e.g. `http://127.0.0.1:4533`) to a
/// friendlier, user-facing origin shown in the address bar.
#[derive(Clone)]
struct UrlMask {
    real: String,
    public: String,
}

impl UrlMask {
    fn new(host: &str, port: u16) -> Self {
        let real = format!("http://{host}:{port}");
        let public = std::env::var("QUINTODROME_PUBLIC_URL")
            .unwrap_or_else(|_| DEFAULT_PUBLIC_URL.to_string());
        Self { real, public }
    }

    /// `http://127.0.0.1:4533/...` -> `http://quintodrome/...`
    fn mask(&self, url: &str) -> String {
        url.strip_prefix(&self.real)
            .map(|rest| format!("{}{}", self.public, rest))
            .unwrap_or_else(|| url.to_string())
    }

    /// `http://quintodrome/...` -> `http://127.0.0.1:4533/...`
    fn unmask(&self, url: &str) -> String {
        url.strip_prefix(&self.public)
            .map(|rest| format!("{}{}", self.real, rest))
            .unwrap_or_else(|| url.to_string())
    }
}

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
    let url = app.state::<UrlMask>().unmask(&url);
    if let Some(content) = app.get_webview("content") {
        if let Ok(url) = Url::parse(&url) {
            let _ = content.navigate(url);
        }
    }
}

#[tauri::command]
fn read_clipboard(app: tauri::AppHandle) -> Result<String, String> {
    app.clipboard().read_text().map_err(|e| e.to_string())
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

/// Keeps the address bar in sync with the content webview's real URL.
///
/// `on_navigation` only fires for actual document loads, so history traversal
/// (back/forward) and SPA `pushState` navigation would otherwise leave the
/// address bar stale. Polling `Webview::url()` covers all of those.
fn start_url_poller(app: &tauri::AppHandle, mask: UrlMask) {
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut last: Option<String> = None;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let Some(content) = handle.get_webview("content") else {
                continue;
            };
            let Ok(url) = content.url() else { continue };
            if url.scheme() != "http" && url.scheme() != "https" {
                continue;
            }
            if url.host_str() == Some("tauri.localhost") {
                continue;
            }
            let masked = mask.mask(url.as_str());
            if last.as_deref() != Some(masked.as_str()) {
                if let Some(toolbar) = handle.get_webview("main") {
                    let _ = toolbar.emit("url-changed", masked.clone());
                }
                last = Some(masked);
            }
        }
    });
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        // The default menu wires up the Edit items (undo/cut/copy/paste/select
        // all), so Cmd+X/C/V/A/Z work in the webview's text fields.
        .menu(|handle| tauri::menu::Menu::default(handle))
        .manage(ServerState::default())
        .invoke_handler(tauri::generate_handler![back, forward, reload, navigate, read_clipboard])
        .setup(|app| {
            let handle = app.handle().clone();

            let host = std::env::var("QUINTODROME_HOST")
                .unwrap_or_else(|_| server::DEFAULT_HOST.to_string());
            let port = std::env::var("QUINTODROME_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(server::DEFAULT_PORT);

            let navidrome = Navidrome::new(host.clone(), port);
            let navidrome_url = navidrome.url.clone();

            let mask = UrlMask::new(&host, port);
            app.manage(mask.clone());

            // The primary webview ("main") is the toolbar; the Navidrome content
            // lives in a child webview below it.
            let content = WebviewBuilder::new("content", WebviewUrl::App("index.html".into()))
                .initialization_script(EDIT_SHORTCUTS_SCRIPT)
                .on_new_window({
                    let handle = handle.clone();
                    move |url, _features| {
                        // Open external links (Last.fm, etc.) in the default
                        // browser instead of a new in-app window.
                        if url.scheme() == "http" || url.scheme() == "https" {
                            let _ = handle.opener().open_url(url.to_string(), None::<&str>);
                        }
                        tauri::webview::NewWindowResponse::Deny
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
                let _ = toolbar.emit("url-changed", mask.mask(&navidrome_url));
            }

            start_url_poller(app.handle(), mask);

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

                // Replace (not push) so the splash page isn't left in history —
                // otherwise "back" would return to the loading screen.
                if let Some(content) = handle.get_webview("content") {
                    let js = format!(
                        "location.replace({})",
                        serde_json::to_string(&content_url).unwrap()
                    );
                    let _ = content.eval(&js);
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
