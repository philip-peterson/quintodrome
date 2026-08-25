# Quintodrome desktop app

# Run the desktop app (builds the Navidrome sidecar, then `cargo run`).
# Requires Go >= 1.26 and Node >= 24 (see .nvmrc) for the sidecar build.
run:
    ./desktop/scripts/build.sh --skip-tauri
    cd desktop/src-tauri && cargo run

# Build the packaged .app (desktop/src-tauri/target/release/bundle/).
bundle:
    ./desktop/scripts/build.sh

# Build only the Navidrome sidecar binary.
sidecar:
    ./desktop/scripts/build.sh --skip-tauri
