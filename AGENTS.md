# AGENTS.md

Instructions for coding agents working in this repo. Humans: see [README.md](README.md).

steam-grid-ui is a Windows app that browses SteamGridDB artwork for the games in the local Steam library and applies it. It only uses the official SteamGridDB API with the user's own key: no scraping, no browser automation.

## Layout

| path                         | contents                                                                          |
| ---------------------------- | --------------------------------------------------------------------------------- |
| `packages/core/`             | Rust `cdylib` (`sgdb_core.dll`): all logic and I/O                                |
| `packages/core/src/lib.rs`   | the C ABI (`sgdb_call`, `sgdb_free`) and the method dispatch                      |
| `packages/core/src/steam.rs` | Steam path, accounts, library, the `config/grid` art slots and their backups      |
| `packages/core/src/sgdb.rs`  | SteamGridDB API v2 client and downloads (steamgriddb.com hosts only)              |
| `packages/core/src/store.rs` | `%APPDATA%\SteamGridUI`: config, name cache, thumbnails; API key lookup (`.env`)  |
| `packages/core/src/vdf.rs`   | parser for Valve's text KeyValues files (`.vdf`, `.acf`)                          |
| `packages/app/`              | WPF frontend, net8, plain code-behind (no MVVM); `Native.cs` is the P/Invoke side |
| `.github/workflows/`         | `ci.yml` (push and PR to `main`), `release.yml` (on a `v*` tag)                   |

The frontend never touches Steam files or the network. It calls `Native.Call("method", args)`, which runs `sgdb_call` off the UI thread with JSON in and `{"ok":true,"data":…}` or `{"ok":false,"error":"…"}` out. A new feature is a new method in `dispatch` plus its caller in the window. Error strings are shown to the user as they are, so word them for a person.

## Commands

```pwsh
dotnet run --project packages/app -c Release     # builds the rust core first, then runs the app
dotnet publish packages/app -c Release -o dist   # one exe with the core bundled inside

cargo fmt --manifest-path packages/core/Cargo.toml
cargo clippy --manifest-path packages/core/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path packages/core/Cargo.toml
dotnet format packages/app/SteamGridUI.csproj

prek install            # once
prek run --all-files    # everything above, plus taplo, zizmor and shfmt
scripts/pin-actions.sh  # pin workflow actions to commit hashes (needs pinact)
```

The toolchain is nightly (`rust-toolchain.toml`), because `rustfmt.toml` uses unstable options. Close the running app before rebuilding: it locks `sgdb_core.dll`.

## Conventions

- Rust: edition 2024, the lint table in `packages/core/Cargo.toml` must pass with `-D warnings`. `unsafe` only at the C ABI, each use under `#[expect(unsafe_code, reason = "…")]`.
- Ids that reach a path or URL (`user`, `appid`) go through `id_arg`, which only accepts digits.
- Replaced or removed artwork is moved to the backup folder, never deleted.
- Workflow actions are pinned to a commit hash with the version in a comment. `permissions: {}` at the top, the least needed per job.
- The API key lives in `.env` or the app's config. Never commit it, log it or put it in an error message.

## Commits

Conventional commits, lower case, one short line, no body unless it is needed: `feat(core): …`, `fix(app): …`, `chore: …`, `docs: …`, `ci: …`. No co-author trailers.

## Releasing

1. Add a `## [x.y.z] - YYYY-MM-DD` section to `CHANGELOG.md`; it becomes the release notes.
2. Bump `version` in `packages/core/Cargo.toml` and `<Version>` in `packages/app/SteamGridUI.csproj`.
3. Push a `vx.y.z` tag. `release.yml` builds `SteamGridUI.exe` and creates the GitHub release with its checksum.
