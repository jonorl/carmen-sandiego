# Contributing

Thanks for wanting to help. Bug reports, clue corrections, new countries, fixes and ports to other boards or screens are all welcome.

## Before you start

For anything bigger than a small fix, open an issue first so we can agree on the approach.

The repository holds two editions of the game: `game/` (desktops, browsers and the records server) and `esp32/` (the ESP32-S3 version). The ESP32 keeps its own copies of some of the game's code and data, listed under "Copied from Carmen" in [esp32/README.md](esp32/README.md): a change to those in `game/` needs carrying over by hand.

## Setting up

You need [rustup](https://rustup.rs). Each folder's `rust-toolchain.toml` pins its Rust version, so the first `cargo` command there installs it.

The game:

```sh
cd game
cargo run                 # play
cargo run -- --check      # validate the data files
./build-web.sh            # browser build in dist/
```

On Linux, the desktop build needs `pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev`.

The ESP32 firmware needs the `esp` toolchain (`espup install`), `ldproxy` and `espflash`. The first build downloads ESP-IDF. Copy `cfg.toml.example` to `cfg.toml` for WiFi, then, with the board plugged in:

```sh
cd esp32
cargo run --release       # build, flash and open the serial monitor
```

## Checks

CI runs these on every pull request, and all of them must pass. In `game/`:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -- --check
```

The server's tests need a Postgres they can create databases on, for example:

```sh
docker run --rm -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:17-alpine
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test --workspace
```

In `esp32/`, the game's code is tested on the computer, through the emulator in `tools/emulator`:

```sh
rustfmt +stable --edition 2024 --check $(git ls-files '*.rs')
cargo clippy --release --bins --lib -- -D warnings -A clippy::missing_const_for_thread_local
cargo build --release
cd tools/emulator
cargo clippy --release --all-targets -- -D warnings
cargo test --release
```

If an ESP32 change shows on the screen, rerecord the demo (`cargo run --release` in `esp32/tools/emulator`, needs ffmpeg) and try it on a board if you have one.

## Style

- British English everywhere: code, comments, clues and copy.
- Comments say why, not what.
- Code is formatted at 140 columns (`rustfmt.toml`).
- Clues follow the rules under "Content rules for clues" in [game/CLAUDE.md](game/CLAUDE.md): never name the country or city, avoid facts that go stale, and give every clue its Argentine Spanish in the `es` section.
- No soldering: parts suggested for the ESP32 hardware must connect without it.

## Pull requests

Keep each one to one change, say what it does and how you tested it (including whether it ran on a board), and run `--check` after any data edit.

By contributing you agree that your work is released under the [MIT licence](LICENSE), and to follow the [code of conduct](CODE_OF_CONDUCT.md).
