//! Local Steam discovery: install path, users, library, and the custom-art folder.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Serialize;
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;
use winreg::enums::HKEY_LOCAL_MACHINE;

use crate::store::Config;
use crate::vdf;

/// Extensions Steam will pick up from userdata/<id>/config/grid.
pub const ART_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

/// Apps that are never games (Steamworks Common Redistributables).
const HIDDEN_APPS: &[u32] = &[228_980];

pub fn steam_path(cfg: &Config) -> Result<PathBuf, String> {
    if let Some(p) = cfg.steam_path.as_deref().filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    let found = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .and_then(|k| k.get_value::<String, _>("SteamPath"))
        .or_else(|_| {
            RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam")
                .and_then(|k| k.get_value::<String, _>("InstallPath"))
        })
        .map_err(|_| "Steam installation not found in the registry".to_string())?;
    let path = PathBuf::from(found.replace('/', "\\"));
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!("Steam path {} does not exist", path.display()))
    }
}

#[derive(Serialize)]
pub struct User {
    pub id: String,
    pub name: String,
}

pub fn users(steam: &Path) -> Vec<User> {
    // accountid (userdata folder name) -> (persona, most recent)
    let mut known: HashMap<String, (String, bool)> = HashMap::new();
    if let Ok(v) = vdf::parse_file(&steam.join("config").join("loginusers.vdf"))
        && let Some(list) = v.get("users")
    {
        for (steam64, info) in list.entries() {
            let Ok(id64) = steam64.parse::<u64>() else { continue };
            let account = (id64 & 0xFFFF_FFFF).to_string();
            let persona = info
                .get("PersonaName")
                .or_else(|| info.get("AccountName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let recent = info.get("MostRecent").and_then(|v| v.as_str()) == Some("1");
            known.insert(account, (persona, recent));
        }
    }

    let mut out: Vec<(bool, User)> = Vec::new();
    if let Ok(dir) = fs::read_dir(steam.join("userdata")) {
        for entry in dir.flatten() {
            let id = entry.file_name().to_string_lossy().into_owned();
            if id == "0" || !is_digits(&id) || !entry.path().is_dir() {
                continue;
            }
            let (persona, recent) = known.get(&id).cloned().unwrap_or_default();
            let name = if persona.is_empty() {
                id.clone()
            } else {
                format!("{persona} ({id})")
            };
            out.push((recent, User { id, name }));
        }
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    out.into_iter().map(|(_, u)| u).collect()
}

fn libraries(steam: &Path) -> Vec<PathBuf> {
    let mut libs = vec![steam.to_path_buf()];
    if let Ok(v) = vdf::parse_file(&steam.join("steamapps").join("libraryfolders.vdf"))
        && let Some(folders) = v.get("libraryfolders")
    {
        for (_, folder) in folders.entries() {
            if let Some(p) = folder.get("path").and_then(|p| p.as_str()) {
                libs.push(PathBuf::from(p));
            }
        }
    }
    let mut seen = HashSet::new();
    libs.retain(|p| seen.insert(p.to_string_lossy().to_lowercase()));
    libs
}

#[derive(Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors the JSON the frontend reads, one flag per art slot"
)]
pub struct Game {
    pub appid: String,
    pub name: String,
    pub installed: bool,
    pub has_grid: bool,
    pub has_wide: bool,
    pub has_hero: bool,
    pub has_logo: bool,
}

pub fn games(steam: &Path, user: &str, names: &HashMap<String, String>) -> Vec<Game> {
    let mut found: BTreeMap<u32, (String, bool)> = BTreeMap::new();

    // Installed games: one appmanifest per game in every library folder.
    for lib in libraries(steam) {
        let Ok(dir) = fs::read_dir(lib.join("steamapps")) else {
            continue;
        };
        for entry in dir.flatten() {
            let path = entry.path();
            let is_manifest = entry.file_name().to_string_lossy().starts_with("appmanifest_")
                && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("acf"));
            if !is_manifest {
                continue;
            }
            let Ok(v) = vdf::parse_file(&path) else {
                continue;
            };
            let Some(state) = v.get("AppState") else { continue };
            let Some(id) = state
                .get("appid")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            let name = state.get("name").and_then(|v| v.as_str()).unwrap_or("");
            found.insert(id, (name.to_string(), true));
        }
    }

    // Everything else the account has touched, from the user's local config.
    let local = steam.join("userdata").join(user).join("config").join("localconfig.vdf");
    if let Ok(v) = vdf::parse_file(&local) {
        let apps = v.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"]);
        for (key, _) in apps.map_or(&[][..], vdf::Vdf::entries) {
            if let Ok(id) = key.parse::<u32>() {
                found
                    .entry(id)
                    .or_insert_with(|| (names.get(key).cloned().unwrap_or_default(), false));
            }
        }
    }

    let art = existing_art(&grid_dir(steam, user));
    let mut out: Vec<Game> = found
        .into_iter()
        .filter(|(id, _)| !HIDDEN_APPS.contains(id))
        .map(|(id, (name, installed))| {
            let has = |kind: &str| art.contains(&art_base(&id.to_string(), kind).unwrap_or_default());
            Game {
                appid: id.to_string(),
                name,
                installed,
                has_grid: has("grid"),
                has_wide: has("wide"),
                has_hero: has("hero"),
                has_logo: has("logo"),
            }
        })
        .collect();
    out.sort_by_cached_key(|g| (g.name.is_empty(), g.name.to_lowercase(), g.appid.len(), g.appid.clone()));
    out
}

pub fn grid_dir(steam: &Path, user: &str) -> PathBuf {
    steam.join("userdata").join(user).join("config").join("grid")
}

/// File stem Steam expects for a given art slot.
pub fn art_base(appid: &str, kind: &str) -> Result<String, String> {
    Ok(match kind {
        "grid" => format!("{appid}p"),
        "wide" => appid.to_string(),
        "hero" => format!("{appid}_hero"),
        "logo" => format!("{appid}_logo"),
        other => return Err(format!("\"{other}\" art cannot be applied to Steam")),
    })
}

fn is_art(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ART_EXTS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Lower-cased stems of every custom image in the grid folder.
fn existing_art(grid: &Path) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Ok(dir) = fs::read_dir(grid) {
        for entry in dir.flatten() {
            let path = entry.path();
            if is_art(&path)
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                set.insert(stem.to_lowercase());
            }
        }
    }
    set
}

/// Custom images currently occupying one slot (any extension).
pub fn slot_files(grid: &Path, base: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(dir) = fs::read_dir(grid) {
        for entry in dir.flatten() {
            let path = entry.path();
            let stem_matches = path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case(base));
            if stem_matches && is_art(&path) {
                out.push(path);
            }
        }
    }
    out
}

/// Move whatever is in a slot into the backup folder instead of deleting it.
pub fn backup_slot(grid: &Path, base: &str, backup_root: &Path, user: &str) -> Result<usize, String> {
    let files = slot_files(grid, base);
    if files.is_empty() {
        return Ok(0);
    }
    let dir = backup_root.join(user);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    for file in &files {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("art");
        let dest = dir.join(format!("{stamp}_{name}"));
        if fs::rename(file, &dest).is_err() {
            // Different volume: copy then remove.
            fs::copy(file, &dest).map_err(|e| format!("backup of {name} failed: {e}"))?;
            fs::remove_file(file).map_err(|e| format!("could not replace {name}: {e}"))?;
        }
    }
    Ok(files.len())
}

pub fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit())
}
