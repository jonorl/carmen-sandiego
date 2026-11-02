# Contributing

Thanks for wanting to help. Bug reports, clue corrections and new countries are all welcome.

## Before you start

For anything bigger than a small fix, open an issue first so we can agree on the approach. The suspects are the owner's family and friends, so pull requests adding people to `suspects.json` won't be merged; build your own copy for yours.

## Setting up

You need [rustup](https://rustup.rs). `rust-toolchain.toml` pins the Rust version, so the first `cargo` command installs it.

```sh
cargo run                 # play
cargo run -- --check      # validate the data files
./build-web.sh            # browser build in dist/
```

On Linux, the desktop build needs `pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev`.

## Checks

CI runs these on every pull request, and all of them must pass:

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

## Style

- British English everywhere: code, comments, clues and copy.
- Comments say why, not what.
- Code is formatted at 140 columns (`rustfmt.toml`); run `cargo fmt --all`.
- Clues follow the rules under "Content rules for clues" in [CLAUDE.md](CLAUDE.md): never name the country or city, avoid facts that go stale, and give every clue its Argentine Spanish in the `es` section.

## Pull requests

Keep each one to one change, say what it does and how you tested it, and run `--check` after any data edit.

By contributing you agree that your work is released under the [MIT licence](LICENSE), and to follow the [code of conduct](CODE_OF_CONDUCT.md).
