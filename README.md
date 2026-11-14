# steam-grid-ui

Windows app that browses [SteamGridDB](https://www.steamgriddb.com/) artwork for the games in your Steam library and applies it: portrait and wide grids, heroes and logos. Icons can be saved to disk.

The backend is a Rust DLL (`packages/core`) that reads the local Steam install and talks to the SteamGridDB API. The frontend is a WPF window (`packages/app`) that calls it over P/Invoke.

## Requirements

- Windows 10 or later, with Steam installed and signed in once
- [.NET 8 Desktop Runtime](https://dotnet.microsoft.com/download/dotnet/8.0)
- A free [SteamGridDB API key](https://www.steamgriddb.com/profile/preferences/api)

## Get it running

Download `SteamGridUI.exe` from the [latest release](https://github.com/juiced-vibing/steam-grid-ui/releases/latest) and run it. The app asks for your API key on first start.

## Build from source

Needs [Rust](https://rustup.rs/) (the nightly toolchain is picked up from `rust-toolchain.toml`) and the [.NET SDK](https://dotnet.microsoft.com/download) 8 or later.

```pwsh
dotnet run --project packages/app -c Release    # builds the Rust core first
dotnet publish packages/app -c Release -o dist  # one exe with the core inside
```

The key can also come from a `.env` at the repo root (see `.env.example`) or the `SGDB_API_KEY` environment variable, both of which win over the saved key.

## How it works

| what                | where                                                                  |
| ------------------- | ---------------------------------------------------------------------- |
| installed games     | `steamapps/appmanifest_*.acf` in every library folder                  |
| other owned games   | `userdata/<id>/config/localconfig.vdf`, named on first lookup          |
| applied artwork     | `userdata/<id>/config/grid/` (`<appid>p`, `<appid>`, `_hero`, `_logo`) |
| replaced artwork    | moved to `%APPDATA%\SteamGridUI\backup\`, never deleted                |
| key, cache, backups | `%APPDATA%\SteamGridUI\`                                               |

Restart Steam if the library does not pick up a change.

> [!NOTE]
> Steam has no custom icon slot for its own games, so icons are download-only. Animated artwork is listed without a preview.

Not affiliated with Valve or SteamGridDB.
