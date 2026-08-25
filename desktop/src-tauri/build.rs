const APP_COMMANDS: &[&str] = &[
    "back",
    "forward",
    "reload",
    "navigate",
    "read_clipboard",
    "write_clipboard",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(APP_COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
