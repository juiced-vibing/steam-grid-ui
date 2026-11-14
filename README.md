# <img src="assets/app/main.png" alt="steam-grid-ui icon" height="28" width="28"> steam-grid-ui

Windows app that browses [SteamGridDB](https://www.steamgriddb.com/) artwork for the games in your Steam library and applies it: portrait and wide grids, heroes and logos. Icons can be saved to disk.

The backend is a Rust DLL (`packages/core`) that reads the local Steam install and talks to the SteamGridDB API. The frontend is a WPF window (`packages/app`) that calls it over P/Invoke.

## Requirements

- Windows 10 or later, 64-bit
- [Steam](https://store.steampowered.com/about/), installed and signed in once
- A free [SteamGridDB API key](https://www.steamgriddb.com/profile/preferences/api); the app asks for it on first start

## Get it running

### Installer (recommended)

Download the setup `.exe` from the [latest release](https://github.com/juiced-vibing/steam-grid-ui/releases/latest). It installs for your user only, and downloads the .NET 10 Desktop Runtime if you do not have it.

### Portable

Download `SteamGridUI.exe` from the [latest release](https://github.com/juiced-vibing/steam-grid-ui/releases/latest) and run it. It needs the [.NET 10 Desktop Runtime](https://dotnet.microsoft.com/download/dotnet/10.0):

```pwsh
winget install Microsoft.DotNet.DesktopRuntime.10
```

## Build from source

```pwsh
winget install Rustlang.Rustup Microsoft.DotNet.SDK.10
winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

The second line is the MSVC linker Rust needs; skip it if you already build Rust on this machine. The nightly toolchain is picked up from `rust-toolchain.toml`.

```pwsh
git clone https://github.com/juiced-vibing/steam-grid-ui.git
cd steam-grid-ui
dotnet run --project packages/app -c Release     # builds the Rust core first
dotnet publish packages/app -c Release -o dist   # one exe with the core inside
```

The installer needs [Inno Setup](https://jrsoftware.org/isinfo.php):

```pwsh
winget install JRSoftware.InnoSetup
scripts/build-installer.ps1 -Version 0.1.0   # dist/SteamGridUI-0.1.0-setup.exe
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
