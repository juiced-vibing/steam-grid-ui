use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::Error;
use crate::error::Result;
use crate::types::ApiKey;
use crate::types::AppId;

const KEY_VAR: &str = "SGDB_API_KEY";

pub const NAMES: &str = "names.json";
pub const MATCHES: &str = "matches.json";

static MAPS_LOCK: Mutex<()> = Mutex::new(());

pub fn app_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map_or_else(std::env::temp_dir, PathBuf::from);
    let dir = base.join("SteamGridUI");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn sub_dir(name: &str) -> PathBuf {
    let dir = app_dir().join(name);
    let _ = fs::create_dir_all(&dir);
    dir
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub steam_path: Option<String>,
}

pub fn load_config() -> Config {
    read_json(&app_dir().join("config.json"))
}

pub fn save_config(cfg: &Config) -> Result<()> {
    let text = serde_json::to_string_pretty(cfg).map_err(|_| Error::Internal)?;
    fs::write(app_dir().join("config.json"), text).map_err(Error::io("saving the settings"))
}

fn read_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn load_map<V: DeserializeOwned>(file: &str) -> HashMap<AppId, V> {
    read_json(&app_dir().join(file))
}

pub fn remember<V: Serialize + DeserializeOwned + PartialEq>(file: &str, app: AppId, value: V) {
    remember_in(&app_dir().join(file), app, value);
}

fn remember_in<V: Serialize + DeserializeOwned + PartialEq>(path: &Path, app: AppId, value: V) {
    let _guard = MAPS_LOCK.lock();

    let mut map: HashMap<AppId, V> = read_json(path);
    if map.get(&app) == Some(&value) {
        return;
    }

    map.insert(app, value);
    if let Ok(text) = serde_json::to_string(&map) {
        let _ = fs::write(path, text);
    }
}

pub fn prune_dir(dir: &Path, max_bytes: u64) -> usize {
    let Ok(entries) = fs::read_dir(dir) else { return 0 };

    let mut files: Vec<(SystemTime, u64, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok().filter(fs::Metadata::is_file)?;
            Some((meta.modified().unwrap_or(UNIX_EPOCH), meta.len(), entry.path()))
        })
        .collect();

    let mut total: u64 = files.iter().map(|(_, len, _)| len).sum();
    if total <= max_bytes {
        return 0;
    }

    files.sort_unstable_by_key(|(modified, ..)| *modified);

    let target = max_bytes / 4 * 3;
    let mut removed = 0;

    for (_, len, path) in files {
        if total <= target {
            break;
        }

        if fs::remove_file(&path).is_ok() {
            total -= len;
            removed += 1;
        }
    }

    removed
}

fn parse_env(text: &str, name: &str) -> Option<String> {
    text.lines()
        .find_map(|line| {
            let line = line.trim();
            if line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            (key.trim() == name).then(|| value.trim().trim_matches(['"', '\'']).to_string())
        })
        .filter(|value| !value.is_empty())
}

fn dotenv(name: &str) -> Option<String> {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
    [exe_dir, std::env::current_dir().ok()]
        .into_iter()
        .flatten()
        .find_map(|start| {
            let text = start
                .ancestors()
                .find_map(|dir| fs::read_to_string(dir.join(".env")).ok())?;
            parse_env(&text, name)
        })
}

pub fn api_key() -> Option<ApiKey> {
    std::env::var(KEY_VAR)
        .ok()
        .or_else(|| dotenv(KEY_VAR))
        .as_deref()
        .and_then(ApiKey::new)
        .or_else(|| ApiKey::new(&load_config().api_key))
}

#[cfg(test)]
#[path = "../tests/store.rs"]
mod tests;
