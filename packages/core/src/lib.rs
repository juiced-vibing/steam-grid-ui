mod error;
mod sgdb;
mod steam;
mod store;
mod types;
mod vdf;

use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::c_char;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use crate::error::Error;
use crate::error::Result;
use crate::sgdb::Asset;
use crate::sgdb::Filters;
use crate::steam::Steam;
use crate::steam::User;
use crate::types::ApiKey;
use crate::types::AppId;
use crate::types::ArtKind;
use crate::types::AssetUrl;
use crate::types::Slot;
use crate::types::UserId;

const THUMB_CACHE_MAX: u64 = 256 * 1024 * 1024;
const KEY_CHECK_APP: u32 = 440;

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "args", rename_all = "snake_case")]
enum Request {
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
enum SteamState {
    Found { path: PathBuf, users: Vec<User> },
    Missing { reason: String },
}

#[derive(Serialize)]
struct InitReply {
    api_key_set: bool,
    steam: SteamState,
}

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum AssetPage {
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
struct PathReply {
    path: PathBuf,
}

#[derive(Serialize)]
struct ClearReply {
    removed: usize,
}

fn reply<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|_| Error::Internal)
}

fn handle(request: Request) -> Result<String> {
    match request {
        Request::Init {} => {
            let cfg = store::load_config();

            // once per start, off the calling thread: keep the thumbnail cache bounded
            std::thread::spawn(|| store::prune_dir(&store::sub_dir("thumbs"), THUMB_CACHE_MAX));

            reply(&InitReply {
                api_key_set: store::api_key().is_some(),
                steam: match Steam::locate(&cfg) {
                    Ok(steam) => SteamState::Found {
                        path: steam.root().to_path_buf(),
                        users: steam.users(),
                    },
                    Err(e) => SteamState::Missing { reason: e.to_string() },
                },
            })
        }

        Request::SetApiKey { key } => {
            let key = ApiKey::new(&key).ok_or(Error::NoApiKey)?;

            // a cheap authenticated request, so a typo is caught now
            sgdb::game_name(&key, KEY_CHECK_APP.into())?;

            let mut cfg = store::load_config();
            cfg.api_key = key.as_str().to_string();
            store::save_config(&cfg)?;
            sgdb::forget_pages();

            reply(&())
        }

        Request::ListGames { user } => {
            let steam = Steam::locate(&store::load_config())?;
            reply(&steam.games(user, &store::load_names()))
        }

        Request::ListAssets {
            appid,
            kind,
            page,
            filters,
            need_name,
        } => {
            let key = store::api_key().ok_or(Error::NoApiKey)?;

            let game_name = if need_name { sgdb::game_name(&key, appid)? } else { None };
            if let Some(name) = &game_name {
                store::remember_name(appid, name);
            }

            reply(&match sgdb::assets(&key, appid, kind, page, filters)? {
                Some(page) => AssetPage::Known {
                    items: page.items,
                    more: page.more,
                    game_name,
                },
                None => AssetPage::Unknown { game_name },
            })
        }

        Request::Thumb { url } => {
            let name = url
                .file_name()
                .ok_or_else(|| Error::BadRequest("the thumbnail URL has no file name".into()))?;

            let path = store::sub_dir("thumbs").join(name);
            if !path.is_file() {
                sgdb::download(&url, &path)?;
            }

            reply(&PathReply { path })
        }

        Request::Apply { user, appid, slot, url } => {
            let format = url
                .format()
                .ok_or_else(|| Error::UnsupportedFormat(url.ext().unwrap_or_default().to_string()))?;

            let steam = Steam::locate(&store::load_config())?;
            let grid = steam.grid(user, &store::sub_dir("backup"));

            // fetched before anything is touched, so a failed download costs nothing
            let staged = grid.stage(appid, slot)?;
            sgdb::download(&url, staged.path())?;

            reply(&grid.place(staged, appid, slot, format)?)
        }

        Request::Clear { user, appid, slot } => {
            let steam = Steam::locate(&store::load_config())?;
            let removed = steam.grid(user, &store::sub_dir("backup")).clear(appid, slot)?;

            reply(&ClearReply { removed })
        }

        Request::SaveAs { url, dest } => {
            sgdb::download(&url, &dest)?;
            reply(&())
        }

        Request::BackupDir {} => reply(&PathReply {
            path: store::sub_dir("backup"),
        }),
    }
}

fn respond(method: &str, args: &str) -> String {
    let outcome = serde_json::from_str::<Value>(args)
        .and_then(|args| serde_json::from_value::<Request>(json!({ "method": method, "args": args })))
        .map_err(|e| Error::BadRequest(e.to_string()))
        .and_then(|request| std::panic::catch_unwind(|| handle(request)).unwrap_or(Err(Error::Internal)));

    match outcome {
        // the reply is already JSON, written in field order so a tag always comes first
        Ok(data) => format!(r#"{{"ok":true,"data":{data}}}"#),
        Err(e) => json!({ "ok": false, "error": { "code": e.code(), "message": e.to_string() } }).to_string(),
    }
}

/// # Safety
/// `method` and `args` must be NUL-terminated strings that stay valid for the call
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "C ABI entry point for the C# frontend")]
pub unsafe extern "C" fn sgdb_call(method: *const c_char, args: *const c_char) -> *mut c_char {
    let text = |ptr: *const c_char| {
        // SAFETY: the caller guarantees a NUL-terminated string for the duration of the call
        (!ptr.is_null()).then(|| unsafe { CStr::from_ptr(ptr) }.to_string_lossy())
    };

    let response = match (text(method), text(args)) {
        (Some(method), Some(args)) => respond(&method, &args),
        _ => respond("", "null"),
    };

    CString::new(response).unwrap_or_default().into_raw()
}

/// # Safety
/// `ptr` must come from `sgdb_call` and be freed exactly once
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "C ABI entry point for the C# frontend")]
pub unsafe extern "C" fn sgdb_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: the caller guarantees `ptr` came from `sgdb_call`
        drop(unsafe { CString::from_raw(ptr) });
    }
}

#[cfg(test)]
#[path = "../tests/lib.rs"]
mod tests;
