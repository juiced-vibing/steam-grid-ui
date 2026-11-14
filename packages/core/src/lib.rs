//! `sgdb_core`: backend for Steam Grid UI.
//!
//! The whole surface is two C functions. `sgdb_call(method, json_args)` blocks and
//! returns a JSON string `{"ok":true,"data":…}` or `{"ok":false,"error":"…"}` that the
//! caller must release with `sgdb_free`.

mod sgdb;
mod steam;
mod store;
mod vdf;

use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::c_char;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use serde_json::json;

fn arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing argument \"{key}\""))
}

fn flag(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// Numeric ids only: these end up in file paths and URLs.
fn id_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    let v = arg(args, key)?;
    if steam::is_digits(v) {
        Ok(v)
    } else {
        Err(format!("\"{key}\" must be numeric"))
    }
}

fn api_key() -> Result<String, String> {
    let key = store::load_config().api_key;
    if key.is_empty() {
        Err("No SteamGridDB API key set. Add one under \"API key…\".".into())
    } else {
        Ok(key)
    }
}

fn steam_dir() -> Result<PathBuf, String> {
    steam::steam_path(&store::load_config())
}

fn show(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn list_assets(args: &Value) -> Result<Value, String> {
    let appid = id_arg(args, "appid")?;
    let kind = arg(args, "kind")?;
    let key = api_key()?;
    let filters = sgdb::Filters {
        page: args.get("page").and_then(Value::as_u64).unwrap_or(0) as u32,
        animated: flag(args, "animated"),
        nsfw: flag(args, "nsfw"),
        humor: flag(args, "humor"),
    };

    let mut game_name = None;
    if flag(args, "need_name") {
        game_name = sgdb::game_name(&key, appid).map_err(String::from)?;
        if let Some(name) = &game_name {
            store::remember_name(appid, name);
        }
    }

    match sgdb::assets(&key, appid, kind, &filters) {
        Ok(body) => Ok(json!({
            "items": body.get("data").cloned().unwrap_or_else(|| json!([])),
            "total": body.get("total"),
            "limit": body.get("limit"),
            "game_name": game_name,
            "known": true,
        })),
        Err(sgdb::ApiError::NotFound) => Ok(json!({
            "items": [], "total": 0, "limit": null, "game_name": game_name, "known": false,
        })),
        Err(e) => Err(e.into()),
    }
}

/// Download an asset into the user's grid folder, backing up whatever held the slot.
fn apply(args: &Value) -> Result<Value, String> {
    let user = id_arg(args, "user")?;
    let appid = id_arg(args, "appid")?;
    let url = arg(args, "url")?;
    let base = steam::art_base(appid, arg(args, "kind")?)?;
    let ext = sgdb::url_ext(url);
    if !steam::ART_EXTS.contains(&ext.as_str()) {
        return Err(format!("Steam cannot use \".{ext}\" files as library art"));
    }

    let grid = steam::grid_dir(&steam_dir()?, user);
    std::fs::create_dir_all(&grid).map_err(|e| format!("{}: {e}", grid.display()))?;

    // Fetch first so a failed download never costs the user their current art.
    let staged = grid.join(format!("{base}.incoming"));
    sgdb::download(url, &staged)?;
    let dest = grid.join(format!("{base}.{ext}"));
    let finish = steam::backup_slot(&grid, &base, &store::sub_dir("backup"), user).and_then(|n| {
        std::fs::rename(&staged, &dest).map_err(|e| e.to_string())?;
        Ok(n)
    });
    match finish {
        Ok(replaced) => Ok(json!({ "path": show(&dest), "replaced": replaced })),
        Err(e) => {
            let _ = std::fs::remove_file(&staged);
            Err(e)
        }
    }
}

fn dispatch(method: &str, args: &Value) -> Result<Value, String> {
    match method {
        "init" => {
            let cfg = store::load_config();
            let steam = steam::steam_path(&cfg);
            let users = steam.as_deref().map_or_default(steam::users);
            Ok(json!({
                "api_key_set": !cfg.api_key.is_empty(),
                "steam_path": steam.as_deref().ok().map(show),
                "steam_error": steam.as_ref().err(),
                "users": users,
            }))
        }

        "set_api_key" => {
            let key = arg(args, "key")?.trim().to_string();
            if !key.is_empty() {
                // Cheap authenticated request so a typo is caught now, not later.
                // 440 is Team Fortress 2; a 404 would still prove the key works.
                sgdb::game_name(&key, "440").map_err(String::from)?;
            }
            let mut cfg = store::load_config();
            cfg.api_key = key;
            store::save_config(&cfg)?;
            Ok(Value::Null)
        }

        "list_games" => {
            let user = id_arg(args, "user")?;
            let games = steam::games(&steam_dir()?, user, &store::load_names());
            Ok(json!(games))
        }

        "list_assets" => list_assets(args),

        "thumb" => {
            let url = arg(args, "url")?;
            let name = sgdb::url_file_name(url);
            if name.is_empty() || name.starts_with('.') {
                return Err("thumbnail URL has no file name".into());
            }
            let path = store::sub_dir("thumbs").join(name);
            if !path.is_file() {
                sgdb::download(url, &path)?;
            }
            Ok(json!({ "path": show(&path) }))
        }

        "apply" => apply(args),

        "clear" => {
            let user = id_arg(args, "user")?;
            let appid = id_arg(args, "appid")?;
            let base = steam::art_base(appid, arg(args, "kind")?)?;
            let grid = steam::grid_dir(&steam_dir()?, user);
            let removed = steam::backup_slot(&grid, &base, &store::sub_dir("backup"), user)?;
            Ok(json!({ "removed": removed }))
        }

        "save_as" => {
            let dest = PathBuf::from(arg(args, "dest")?);
            sgdb::download(arg(args, "url")?, &dest)?;
            Ok(Value::Null)
        }

        "backup_dir" => Ok(json!({ "path": show(&store::sub_dir("backup")) })),

        other => Err(format!("unknown method \"{other}\"")),
    }
}

#[expect(unsafe_code, reason = "reads the C strings handed over by the C# frontend")]
fn run(method: *const c_char, args: *const c_char) -> Result<Value, String> {
    if method.is_null() || args.is_null() {
        return Err("null argument".into());
    }
    // SAFETY: the caller passes NUL-terminated buffers that outlive this call.
    let (method, args) = unsafe { (CStr::from_ptr(method), CStr::from_ptr(args)) };
    let method = method.to_str().map_err(|_| "method is not UTF-8")?;
    let args: Value = serde_json::from_slice(args.to_bytes()).map_err(|e| format!("bad arguments: {e}"))?;
    dispatch(method, &args)
}

/// # Safety
/// `method` and `args` must be NUL-terminated UTF-8 strings.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "C ABI entry point for the C# frontend")]
pub unsafe extern "C" fn sgdb_call(method: *const c_char, args: *const c_char) -> *mut c_char {
    let result =
        std::panic::catch_unwind(|| run(method, args)).unwrap_or_else(|_| Err("internal error in sgdb_core".into()));
    let reply = match result {
        Ok(data) => json!({ "ok": true, "data": data }),
        Err(error) => json!({ "ok": false, "error": error }),
    };
    CString::new(reply.to_string()).unwrap_or_default().into_raw()
}

/// # Safety
/// `ptr` must come from `sgdb_call` and be freed exactly once.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "C ABI entry point for the C# frontend")]
pub unsafe extern "C" fn sgdb_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}
