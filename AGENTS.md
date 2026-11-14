# AGENTS.md

Instructions for coding agents working in this repo. Humans: see [README.md](README.md).

steam-grid-ui is a Windows app that browses SteamGridDB artwork for the games in the local Steam library and applies it. It only uses the official SteamGridDB API with the user's own key: no scraping, no browser automation.

## Layout

| path                         | contents                                                                            |
| ---------------------------- | ----------------------------------------------------------------------------------- |
| `packages/core/`             | Rust `cdylib` (`sgdb_core.dll`): all logic and I/O                                  |
| `packages/core/src/lib.rs`   | the C ABI (`sgdb_call`, `sgdb_free`), the `Request` enum and its handlers           |
| `packages/core/src/types.rs` | values checked once where they enter: ids, `ApiKey`, `Slot`, `AssetUrl`             |
| `packages/core/src/error.rs` | the one `Error` enum (thiserror); each variant has a user message and a stable code |
| `packages/core/src/steam.rs` | `Steam` (install, accounts, library) and `GridDir` (art slots and their backups)    |
| `packages/core/src/sgdb.rs`  | SteamGridDB API v2 client, the page cache and downloads                             |
| `packages/core/src/store.rs` | `%APPDATA%\SteamGridUI`: config, name cache, thumbnails; API key lookup (`.env`)    |
| `packages/core/src/vdf.rs`   | parser for Valve's text KeyValues files (`.vdf`, `.acf`)                            |
| `packages/core/tests/`       | the unit tests of each module                                                       |
| `packages/app/`              | WPF frontend, net10, fluent theme, plain code-behind (no MVVM)                      |
| `packages/app/Protocol.cs`   | the C# mirror of the core's requests, replies and error codes                       |
| `packages/app/Native.cs`     | the P/Invoke side: `Native.Send(request)`                                           |
| `assets/`                    | `app/` the icon (`main.ico`, `main.png`), `installer/` the setup wizard images      |
| `installer/SteamGridUI.iss`  | Inno Setup script; fetches the .NET 10 Desktop Runtime when it is missing           |
| `.github/workflows/`         | `ci.yml` (push and PR to `main`), `release.yml` (on a `v*` tag)                     |

The frontend never touches Steam files or the network. It calls `Native.Send(new SomeRequest(…))`, which runs `sgdb_call` off the UI thread and returns the typed reply that request declares, or throws a `BackendException` with an `ErrorCode`. On the wire it is JSON: `{"ok":true,"data":…}` or `{"ok":false,"error":{"code":"…","message":"…"}}`.

A new feature is a variant of `Request` in `lib.rs` with its arm in `handle`, and the matching record in `Protocol.cs`. A new failure is a variant of `Error` with its code, and the same name in `ErrorCode`.

## Commands

```pwsh
dotnet run --project packages/app -c Release     # builds the rust core first, then runs the app
dotnet publish packages/app -c Release -o dist   # one exe with the core bundled inside

cargo fmt --manifest-path packages/core/Cargo.toml
cargo clippy --manifest-path packages/core/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path packages/core/Cargo.toml
dotnet format packages/app/SteamGridUI.csproj

prek install             # once
prek run --all-files     # everything above, plus taplo, zizmor and shfmt
scripts/pin-actions.sh   # pin workflow actions to commit hashes (needs pinact)
scripts/build-installer.ps1 -Version 0.1.0   # dist/SteamGridUI-0.1.0-setup.exe (needs inno setup)
```

The toolchain is nightly (`rust-toolchain.toml`), because `rustfmt.toml` uses unstable options. Close the running app before rebuilding: it locks `sgdb_core.dll`.

## Conventions

- Rust: edition 2024, the lint table in `packages/core/Cargo.toml` must pass with `-D warnings`. `unsafe` only at the C ABI, each use under `#[expect(unsafe_code, reason = "…")]`.
- Unit tests live in `packages/core/tests/<module>.rs`, pulled into the module with `#[cfg(test)] #[path = "../tests/<module>.rs"] mod tests;`. `autotests = false` keeps cargo from building them as integration tests.
- Tests are for edge cases and invariants (bad input, limits, what must not happen), not for restating the code. The C# side has none: keep logic in the core.
- Comments: no docstrings on anything that is not public API, and few comments at all. A short `//` line only where the reason is not obvious from the code; lower case, no trailing period.
- Make invalid states unrepresentable instead of checking for them. A value is validated once, by its type, when a request is parsed: `AppId` and `UserId` are numbers, an `AssetUrl` is an https steamgriddb.com URL, a `Slot` is never an icon, an `ApiKey` is never empty. Code that receives one does not check again. Use enums with data (`Placed`, `AssetPage`, `SteamState`, `Preview`) rather than flags and nullable fields.
- Errors: return `error::Result`, add context with `Error::io("doing what")`. The message is shown to the user as it is, so word it for a person. The frontend decides what to do from the code, never from the message, and handles every failure in `MainWindow.Report`.
- Replaced or removed artwork is moved to the backup folder, never deleted; the newest three per slot are kept.
- Workflows and scripts: one command per line, no `\` continuations.
- Workflow actions are pinned to a commit hash with the version in a comment. `permissions: {}` at the top, the least needed per job.
- The API key lives in `.env` or the app's config. Never commit it, log it or put it in an error message.

## Commits

Conventional commits, lower case, one short line, no body unless it is needed: `feat(core): …`, `fix(app): …`, `chore: …`, `docs: …`, `ci: …`. No co-author trailers.

## Releasing

1. Add a `## [x.y.z] - YYYY-MM-DD` section to `CHANGELOG.md`; it becomes the release notes.
2. Bump `version` in `packages/core/Cargo.toml` and `<Version>` in `packages/app/SteamGridUI.csproj`.
3. Push a `vx.y.z` tag. `release.yml` builds `SteamGridUI.exe` and the installer, and creates the GitHub release with their checksums.
