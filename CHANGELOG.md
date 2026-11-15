# Changelog

All notable changes to steam-grid-ui.

## [0.1.5] - 2026-11-15

### Changed

- Even spacing throughout the main window and the API key dialog
- Captions, placeholders and error text follow the light or dark theme instead of fixed colours

## [0.1.4] - 2026-11-15

### Changed

- Maintenance release: nothing changes in how the app looks or behaves

## [0.1.3] - 2026-11-15

### Added

- Non-Steam games: the shortcuts of the account are listed next to its Steam games, and take grids, heroes and logos the same way
- A non-Steam game is matched to SteamGridDB by name; when the first match is the wrong game, pick another from the list above the artwork and it is remembered

## [0.1.2] - 2026-11-15

### Added

- "Show IDs instead of names" swaps the two lines of every game in the list

## [0.1.1] - 2026-11-15

### Added

- Sort the game list by name or by ID

## [0.1.0] - 2026-11-14

### Added

- Browse SteamGridDB portrait grids, wide grids, heroes, logos and icons for the games in your Steam library
- Apply a grid, hero or logo to Steam in one click; the artwork it replaces is moved to a backup folder, which keeps the last three per game and slot
- Save any artwork to disk
- Installer that also installs the .NET 10 Desktop Runtime when it is missing
- Filters for animated, humor and NSFW artwork
- API key from the in-app dialog, a `.env` file or the `SGDB_API_KEY` environment variable
