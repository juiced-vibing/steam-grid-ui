use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;
use winreg::enums::HKEY_LOCAL_MACHINE;

use crate::error::Error;
use crate::error::Result;
use crate::grid::GridDir;
use crate::store::Config;
use crate::types::AppId;
use crate::types::Slot;
use crate::types::UserId;
use crate::vdf;

const HIDDEN_APPS: &[u32] = &[228_980];

#[derive(Debug, Serialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Game {
    pub appid: AppId,
    pub name: Option<String>,
    pub installed: bool,
    pub custom: Vec<Slot>,
}

#[derive(Debug)]
pub struct Steam {
    root: PathBuf,
}

impl Steam {
    pub fn locate(cfg: &Config) -> Result<Self> {
        if let Some(path) = cfg.steam_path.as_deref().filter(|p| !p.is_empty()) {
            return Self::at(PathBuf::from(path));
        }

        let found = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Valve\Steam")
            .and_then(|k| k.get_value::<String, _>("SteamPath"))
            .or_else(|_| {
                RegKey::predef(HKEY_LOCAL_MACHINE)
                    .open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam")
                    .and_then(|k| k.get_value::<String, _>("InstallPath"))
            })
            .map_err(|_| Error::SteamNotFound("it is not in the registry".into()))?;

        Self::at(PathBuf::from(found.replace('/', "\\")))
    }

    fn at(root: PathBuf) -> Result<Self> {
        if root.is_dir() {
            Ok(Self { root })
        } else {
            Err(Error::SteamNotFound(format!("{} does not exist", root.display())))
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn users(&self) -> Vec<User> {
        // account id (userdata folder name) -> (persona, most recent)
        let mut known: HashMap<u32, (String, bool)> = HashMap::new();

        if let Ok(v) = vdf::parse_file(&self.root.join("config").join("loginusers.vdf"))
            && let Some(list) = v.get("users")
        {
            for (steam64, info) in list.entries() {
                let Ok(id64) = steam64.parse::<u64>() else { continue };
                let account = (id64 & 0xFFFF_FFFF) as u32;
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
        if let Ok(dir) = fs::read_dir(self.root.join("userdata")) {
            for entry in dir.flatten() {
                // "0" is Steam's placeholder for no account
                let Some(id) = entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u32>().ok())
                    .filter(|&id| id != 0 && entry.path().is_dir())
                else {
                    continue;
                };

                let (persona, recent) = known.get(&id).cloned().unwrap_or_default();
                let name = if persona.is_empty() {
                    id.to_string()
                } else {
                    format!("{persona} ({id})")
                };

                out.push((recent, User { id: id.into(), name }));
            }
        }

        out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
        out.into_iter().map(|(_, user)| user).collect()
    }

    fn libraries(&self) -> Vec<PathBuf> {
        let mut libs = vec![self.root.clone()];

        if let Ok(v) = vdf::parse_file(&self.root.join("steamapps").join("libraryfolders.vdf"))
            && let Some(folders) = v.get("libraryfolders")
        {
            for (_, folder) in folders.entries() {
                if let Some(path) = folder.get("path").and_then(|p| p.as_str()) {
                    libs.push(PathBuf::from(path));
                }
            }
        }

        let mut seen = HashSet::new();

        libs.retain(|path| seen.insert(path.to_string_lossy().to_lowercase()));
        libs
    }

    pub fn games(&self, user: UserId, names: &HashMap<AppId, String>) -> Vec<Game> {
        // appid -> (name, installed)
        let mut found: BTreeMap<u32, (Option<String>, bool)> = BTreeMap::new();

        for lib in self.libraries() {
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

                let name = state
                    .get("name")
                    .and_then(|v| v.as_str())
                    .filter(|name| !name.is_empty());
                found.insert(id, (name.map(str::to_string), true));
            }
        }

        let local = self
            .root
            .join("userdata")
            .join(user.to_string())
            .join("config")
            .join("localconfig.vdf");
        if let Ok(v) = vdf::parse_file(&local) {
            let apps = v.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"]);
            for (key, _) in apps.map_or(&[][..], vdf::Vdf::entries) {
                if let Ok(id) = key.parse::<u32>() {
                    found
                        .entry(id)
                        .or_insert_with(|| (names.get(&id.into()).cloned(), false));
                }
            }
        }

        let art = self.grid(user, Path::new("")).art_stems();

        let mut out: Vec<Game> = found
            .into_iter()
            .filter(|(id, _)| !HIDDEN_APPS.contains(id))
            .map(|(id, (name, installed))| {
                let appid = AppId::from(id);
                Game {
                    appid,
                    name,
                    installed,
                    custom: Slot::ALL
                        .into_iter()
                        .filter(|slot| art.contains(&slot.file_stem(appid)))
                        .collect(),
                }
            })
            .collect();

        // named games first, by name; the rest by appid
        out.sort_by_cached_key(|game| (game.name.as_deref().map(str::to_lowercase).ok_or(()), game.appid));
        out
    }

    pub fn grid(&self, user: UserId, backup_root: &Path) -> GridDir {
        let user = user.to_string();
        GridDir::new(
            self.root.join("userdata").join(&user).join("config").join("grid"),
            backup_root.join(user),
        )
    }
}

#[cfg(test)]
#[path = "../tests/steam.rs"]
mod tests;
