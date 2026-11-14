//! `SteamGridDB` API v2 client (<https://www.steamgriddb.com/api/v2>).

use std::fs;
use std::io::Read;
use std::io::{self};
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde_json::Value;

const BASE: &str = "https://www.steamgriddb.com/api/v2";
const AGENT: &str = "SteamGridUI/0.1";
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;

pub enum ApiError {
    NotFound,
    Other(String),
}

impl From<ApiError> for String {
    fn from(e: ApiError) -> String {
        match e {
            ApiError::NotFound => "Not found on SteamGridDB".into(),
            ApiError::Other(s) => s,
        }
    }
}

fn describe(err: ureq::Error) -> ApiError {
    match err {
        ureq::Error::Status(404, _) => ApiError::NotFound,
        ureq::Error::Status(401 | 403, _) => {
            ApiError::Other("SteamGridDB rejected the API key. Check it under \"API key…\".".into())
        }
        ureq::Error::Status(429, _) => {
            ApiError::Other("SteamGridDB rate limit reached. Wait a moment and try again.".into())
        }
        ureq::Error::Status(code, _) => ApiError::Other(format!("SteamGridDB returned HTTP {code}")),
        ureq::Error::Transport(t) => ApiError::Other(format!("Network error: {t}")),
    }
}

pub fn get(key: &str, path: &str, query: &[(&str, &str)]) -> Result<Value, ApiError> {
    let mut req = ureq::get(&format!("{BASE}{path}"))
        .set("Authorization", &format!("Bearer {key}"))
        .set("User-Agent", AGENT)
        .timeout(Duration::from_secs(30));
    for (k, v) in query {
        req = req.query(k, v);
    }
    let body: Value = req
        .call()
        .map_err(describe)?
        .into_json()
        .map_err(|e| ApiError::Other(format!("Unreadable SteamGridDB response: {e}")))?;
    if body.get("success").and_then(Value::as_bool) == Some(false) {
        let msg = body
            .get("errors")
            .and_then(|e| e.get(0))
            .and_then(Value::as_str)
            .unwrap_or("request failed");
        return Err(ApiError::Other(format!("SteamGridDB: {msg}")));
    }
    Ok(body)
}

/// Name of the game `SteamGridDB` associates with a Steam appid.
pub fn game_name(key: &str, appid: &str) -> Result<Option<String>, ApiError> {
    match get(key, &format!("/games/steam/{appid}"), &[]) {
        Ok(body) => Ok(body
            .get("data")
            .and_then(|d| d.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string)),
        Err(ApiError::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct Filters {
    pub page: u32,
    pub animated: bool,
    pub nsfw: bool,
    pub humor: bool,
}

/// One page of assets for a Steam appid. `kind` is grid | wide | hero | logo | icon.
pub fn assets(key: &str, appid: &str, kind: &str, f: &Filters) -> Result<Value, ApiError> {
    let (endpoint, dimensions) = match kind {
        "grid" => ("grids", Some("600x900,342x482,660x930")),
        "wide" => ("grids", Some("460x215,920x430")),
        "hero" => ("heroes", None),
        "logo" => ("logos", None),
        "icon" => ("icons", None),
        other => return Err(ApiError::Other(format!("unknown asset kind \"{other}\""))),
    };
    let page = f.page.to_string();
    let mut query = vec![
        ("types", if f.animated { "static,animated" } else { "static" }),
        ("nsfw", if f.nsfw { "any" } else { "false" }),
        ("humor", if f.humor { "any" } else { "false" }),
        ("page", page.as_str()),
    ];
    if let Some(d) = dimensions {
        query.push(("dimensions", d));
    }
    get(key, &format!("/{endpoint}/steam/{appid}"), &query)
}

/// Only ever download from `SteamGridDB`'s own hosts.
fn check_host(url: &str) -> Result<(), String> {
    let host = url
        .strip_prefix("https://")
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .unwrap_or("");
    if host == "steamgriddb.com" || host.ends_with(".steamgriddb.com") {
        Ok(())
    } else {
        Err("Refusing to download from a host that is not steamgriddb.com".into())
    }
}

/// File extension of the asset behind a URL, lower-cased.
pub fn url_ext(url: &str) -> String {
    let file = url
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("");
    match file.rsplit_once('.') {
        Some((_, ext)) if ext.len() <= 5 && ext.bytes().all(|b| b.is_ascii_alphanumeric()) => ext.to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// Last path segment, reduced to characters that are safe in a file name.
pub fn url_file_name(url: &str) -> String {
    url.split(['?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect()
}

/// Download to a temp file next to `dest`, then move it into place.
pub fn download(url: &str, dest: &Path) -> Result<(), String> {
    check_host(url)?;
    let resp = ureq::get(url)
        .set("User-Agent", AGENT)
        .timeout(Duration::from_secs(180))
        .call()
        .map_err(|e| String::from(describe(e)))?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let tmp = dest.with_extension(format!("part{nanos}"));
    let result = (|| -> io::Result<()> {
        let mut file = fs::File::create(&tmp)?;
        let copied = io::copy(&mut resp.into_reader().take(MAX_DOWNLOAD), &mut file)?;
        if copied == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "empty download"));
        }
        drop(file);
        fs::rename(&tmp, dest)
    })();
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Download failed: {e}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_allowlist() {
        assert!(check_host("https://cdn2.steamgriddb.com/grid/abc.png").is_ok());
        assert!(check_host("https://steamgriddb.com/x").is_ok());
        assert!(check_host("https://evilsteamgriddb.com/x.png").is_err());
        assert!(check_host("https://steamgriddb.com.evil.io/x.png").is_err());
        assert!(check_host("http://cdn2.steamgriddb.com/x.png").is_err());
        assert!(check_host("file:///c:/x.png").is_err());
    }

    #[test]
    fn url_parts() {
        assert_eq!(url_ext("https://cdn2.steamgriddb.com/grid/abc.PNG?x=1"), "png");
        assert_eq!(url_ext("https://cdn2.steamgriddb.com/grid/abc"), "");
        assert_eq!(
            url_file_name("https://cdn2.steamgriddb.com/thumb/a..b/../c d.jpg"),
            "cd.jpg"
        );
    }
}
