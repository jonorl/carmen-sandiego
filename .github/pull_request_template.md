## What this changes

## How it was tested

The game (in `game/`):

- [ ] `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo run -- --check` (after any data edit)
- [ ] Played it, on the desktop or in the browser, if it changes what's on screen

The ESP32 version (in `esp32/`):

- [ ] `rustfmt +stable --edition 2024 --check $(git ls-files '*.rs')`
- [ ] In `tools/emulator`: `cargo clippy --release --all-targets -- -D warnings` and `cargo test --release`
- [ ] Ran it on a board, if it changes what the device does
- [ ] Rerecorded `docs/demo.gif`, if it changes what's on screen
