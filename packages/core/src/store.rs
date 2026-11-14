//! App-local state under %APPDATA%\SteamGridUI: config, name cache, thumbnails, backups.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Deserialize;
use serde::Serialize;

static NAMES_LOCK: Mutex<()> = Mutex::new(());

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
    /// Optional override when Steam isn't in the registry.
    #[serde(default)]
    pub steam_path: Option<String>,
}

pub fn load_config() -> Config {
    fs::read_to_string(app_dir().join("config.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &Config) -> Result<(), String> {
    let text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(app_dir().join("config.json"), text).map_err(|e| e.to_string())
}

/// appid -> game name, for owned games that aren't installed (no manifest to read).
pub fn load_names() -> HashMap<String, String> {
    fs::read_to_string(app_dir().join("names.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn remember_name(appid: &str, name: &str) {
    let _guard = NAMES_LOCK.lock();
    let mut names = load_names();
    if names.get(appid).map(String::as_str) == Some(name) {
        return;
    }
    names.insert(appid.to_string(), name.to_string());
    if let Ok(text) = serde_json::to_string(&names) {
        let _ = fs::write(app_dir().join("names.json"), text);
    }
}
