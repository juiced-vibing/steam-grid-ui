//! App-local state under %APPDATA%\SteamGridUI: config, name cache, thumbnails, backups.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Deserialize;
use serde::Serialize;

/// Environment variable (or `.env` entry) that overrides the key saved from the UI.
const KEY_VAR: &str = "SGDB_API_KEY";

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

/// Value of `name` in the nearest `.env`, walking up from the exe and from the working directory.
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

/// The API key to use: process environment, then `.env`, then the key saved from the UI.
pub fn api_key() -> String {
    std::env::var(KEY_VAR)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| dotenv(KEY_VAR))
        .unwrap_or_else(|| load_config().api_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_keys_from_env_text() {
        let text = "# comment\nOTHER=1\nSGDB_API_KEY = \"abc123\"\n";
        assert_eq!(parse_env(text, "SGDB_API_KEY").as_deref(), Some("abc123"));
        assert_eq!(parse_env("SGDB_API_KEY=\n", "SGDB_API_KEY"), None);
        assert_eq!(parse_env("#SGDB_API_KEY=x\n", "SGDB_API_KEY"), None);
    }
}
