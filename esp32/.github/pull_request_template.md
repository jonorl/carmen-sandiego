## What this changes

## How it was tested

- [ ] `rustfmt +stable --edition 2024 --check $(git ls-files '*.rs')`
- [ ] `cargo clippy --release --bins --lib -- -D warnings -A clippy::missing_const_for_thread_local` and `cargo build --release`
- [ ] In `tools/emulator`: `cargo clippy --release --all-targets -- -D warnings` and `cargo test --release`
- [ ] Ran it on a board, if it changes what the device does
- [ ] Rerecorded `docs/demo.gif`, if it changes what's on screen
