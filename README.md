# steam-grid-ui

Windows app that browses [SteamGridDB](https://www.steamgriddb.com/) artwork for the games in your Steam library and applies it: portrait and wide grids, heroes and logos. Icons can be saved to disk.

The backend is a Rust DLL (`packages/core`) that reads the local Steam install and talks to the SteamGridDB API. The frontend is a WPF window (`packages/app`) that calls it over P/Invoke.

## Requirements

- Windows 10 or later, with Steam installed and signed in once
- A free [SteamGridDB API key](https://www.steamgriddb.com/profile/preferences/api)
- [Rust](https://rustup.rs/) (the nightly toolchain is picked up from `rust-toolchain.toml`)
- [.NET SDK](https://dotnet.microsoft.com/download) 8 or later

## Run

```pwsh
dotnet run --project packages/app -c Release
```

This builds the Rust core first and copies `sgdb_core.dll` next to the exe.

The app asks for your API key on first start. You can also put it in a `.env` at the repo root (see `.env.example`) or in the `SGDB_API_KEY` environment variable, both of which win over the saved key.

## How it works

| what                | where                                                                    |
| ------------------- | ------------------------------------------------------------------------ |
| installed games     | `steamapps/appmanifest_*.acf` in every library folder                    |
| other owned games   | `userdata/<id>/config/localconfig.vdf`, named on first lookup            |
| applied artwork     | `userdata/<id>/config/grid/` (`<appid>p`, `<appid>`, `_hero`, `_logo`)   |
| replaced artwork    | moved to `%APPDATA%\SteamGridUI\backup\`, never deleted                  |
| key, cache, backups | `%APPDATA%\SteamGridUI\`                                                 |

Restart Steam if the library does not pick up a change.

> [!NOTE]
> Steam has no custom icon slot for its own games, so icons are download-only. Animated artwork is listed without a preview.

## Development

```pwsh
prek install
prek run --all-files
cargo test --manifest-path packages/core/Cargo.toml
```

Not affiliated with Valve or SteamGridDB.
