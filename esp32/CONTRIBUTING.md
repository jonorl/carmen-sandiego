# Contributing

Thanks for wanting to help. Bug reports, fixes and ports to other boards or screens are all welcome. Changes to the game itself (its rules, clues and countries) belong in [Carmen](https://github.com/jonorl/carmen): this repository copies them from there (see "Copied from Carmen" in the [README](README.md)).

## Before you start

For anything bigger than a small fix, open an issue first so we can agree on the approach. The suspects are the owner's family and friends, so pull requests adding people to `suspects.json` won't be merged; build your own copy for yours.

## Setting up

The firmware needs the `esp` toolchain (`espup install`), `ldproxy` and `espflash`; `rust-toolchain.toml` picks the toolchain. The first build downloads ESP-IDF. Copy `cfg.toml.example` to `cfg.toml` for WiFi, then, with the board plugged in:

```sh
cargo run --release        # build, flash and open the serial monitor
```

Wiring and the checks in `src/bin` are in the [README](README.md).

## Checks

CI runs these on every pull request, and all of them must pass. The game's code is tested on the computer, through the emulator in `tools/emulator`:

```sh
rustfmt +stable --edition 2024 --check $(git ls-files '*.rs')
cargo clippy --release --bins --lib -- -D warnings -A clippy::missing_const_for_thread_local
cargo build --release
cd tools/emulator
cargo clippy --release --all-targets -- -D warnings
cargo test --release
```

If your change shows on the screen, rerecord the demo (`cargo run --release` in `tools/emulator`, needs ffmpeg) and try it on a board if you have one.

## Style

- British English everywhere: code, comments and copy.
- Comments say why, not what.
- Code is formatted at 140 columns (`rustfmt.toml`).
- No soldering: parts suggested for the hardware must connect without it.

## Pull requests

Keep each one to one change, and say what it does and how you tested it, including whether it ran on a board.

By contributing you agree that your work is released under the [MIT licence](LICENSE), and to follow the [code of conduct](CODE_OF_CONDUCT.md).
