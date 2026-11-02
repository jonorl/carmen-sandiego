# Where in the World?

A fan remake of the 1990 *Where in the World is Carmen Sandiego?*, in two editions that share this repository. Each folder is its own Cargo workspace with its own toolchain, so run commands from inside it, and read its own `CLAUDE.md` before working there:

- `game/`: the game for desktops and browsers (macroquad, WebAssembly on the web), `server/` (the records server, axum over Postgres) and `shared/` (what the two share). See `game/CLAUDE.md`.
- `esp32/`: the ESP32-S3 edition, with `tools/emulator/` running its code on the computer for tests and the demo. See `esp32/CLAUDE.md`.

## How the two relate

`esp32/` copies some of the game's code and data rather than sharing it (the device needs it freed of macroquad): `countries.json`, `suspects.json`, `assets/suspects/`, `assets/fonts/`, `assets/sprites/`, and modules under `src/art/`, `src/rules/` and `src/lang.rs`. The copies and what was changed in them are listed under "Copied from Carmen" in `esp32/README.md`. A change to any of them in `game/` doesn't reach `esp32/` by itself: carry it over in the same change, and update that list.

Both editions keep arrests on the same records server (`carmen-api.jonathan-orlowski.dev`), shared with the owner's private family edition of the game.

## Repository-wide

- `.github/workflows/`: `ci.yml` (jobs Game, ESP32 tests and ESP32 firmware, on every push and pull request, unfiltered so required checks always report), `codeql.yml`, `deploy-web.yml` (changes under `game/`: tests, `--check`, `build-web.sh`, then Cloudflare Pages project `carmen-sandiego`, served at `jonathan-orlowski.dev/carmen-sandiego/` through the owner's `renamer` Worker), `deploy-server.yml` (changes to `game/server`, `game/shared` or `game/Cargo.lock`: image `ghcr.io/jonorl/carmen-api`, pulled on the VPS), `release-game.yml` (tag `game-v*` matching `game/Cargo.toml`) and `release-esp32.yml` (tag `esp32-v*` matching `esp32/Cargo.toml`). Both editions release from this repository, so each workflow writes its notes from its own previous tag.
- `.github/dependabot.yml`: one entry per folder and ecosystem.
- `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1, kept verbatim) and `LICENSE` cover both editions. Each folder keeps a README with its own details.
- Dossier photos and their sources are in `game/assets/suspects/CREDITS.md`, copied to `esp32/assets/suspects/`. Several aren't freely licensed.
- British English throughout. Comments explain why, not what. 140-column formatting in both editions.
