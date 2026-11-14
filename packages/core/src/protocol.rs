use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::sgdb::Asset;
use crate::sgdb::Filters;
use crate::steam::User;
use crate::types::AppId;
use crate::types::ArtKind;
use crate::types::AssetUrl;
use crate::types::Slot;
use crate::types::UserId;

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "args", rename_all = "snake_case")]
pub enum Request {
    Init {},
    SetApiKey {
        key: String,
    },
    ListGames {
        user: UserId,
    },
    ListAssets {
        appid: AppId,
        kind: ArtKind,
        #[serde(default)]
        page: u32,
        #[serde(flatten)]
        filters: Filters,
        #[serde(default)]
        need_name: bool,
    },
    Thumb {
        url: AssetUrl,
    },
    Apply {
        user: UserId,
        appid: AppId,
        slot: Slot,
        url: AssetUrl,
    },
    Clear {
        user: UserId,
        appid: AppId,
        slot: Slot,
    },
    SaveAs {
        url: AssetUrl,
        dest: PathBuf,
    },
    BackupDir {},
}

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SteamState {
    Found { path: PathBuf, users: Vec<User> },
    Missing { reason: String },
}

#[derive(Serialize)]
pub struct InitReply {
    pub api_key_set: bool,
    pub steam: SteamState,
}

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AssetPage {
    Known {
        items: Vec<Asset>,
        more: bool,
        game_name: Option<String>,
    },
    Unknown {
        game_name: Option<String>,
    },
}

#[derive(Serialize)]
pub struct PathReply {
    pub path: PathBuf,
}

#[derive(Serialize)]
pub struct ClearReply {
    pub removed: usize,
}

#[cfg(test)]
#[path = "../tests/protocol.rs"]
mod tests;
