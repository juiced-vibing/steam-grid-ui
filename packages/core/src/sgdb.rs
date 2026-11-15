use std::fs;
use std::io::Read;
use std::io::{self};
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use ureq::Agent;

use crate::cache;
use crate::error::Error;
use crate::error::Result;
use crate::types::ApiKey;
use crate::types::AppId;
use crate::types::ArtKind;
use crate::types::AssetUrl;
use crate::types::GameId;
use crate::types::GameRef;

const BASE: &str = "https://www.steamgriddb.com/api/v2";
const USER_AGENT: &str = "SteamGridUI/0.1";
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;

const API_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(180);

const DEFAULT_LIMIT: usize = 50;

static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::config_builder()
        .timeout_global(Some(API_TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into()
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Asset {
    pub url: AssetUrl,
    pub preview: Option<AssetUrl>,
    pub width: u32,
    pub height: u32,
    pub style: Option<String>,
    pub author: Option<String>,
}

impl Asset {
    fn parse(raw: &Value) -> Option<Self> {
        let text = |key: &str| raw.get(key).and_then(Value::as_str).filter(|s| !s.is_empty());
        let size = |key: &str| raw.get(key).and_then(Value::as_u64).unwrap_or(0) as u32;
        let url = |key: &str| AssetUrl::try_from(text(key)?.to_string()).ok();

        Some(Self {
            url: url("url")?,
            preview: url("thumb").filter(|thumb| thumb.format().is_some()),
            width: size("width"),
            height: size("height"),
            style: text("style").map(str::to_string),
            author: raw
                .get("author")
                .and_then(|author| author.get("name"))
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_string),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub items: Vec<Asset>,
    pub more: bool,
}

impl Page {
    fn parse(body: &Value) -> Self {
        let raw = body
            .get("data")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        let limit = body
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(DEFAULT_LIMIT, |limit| limit as usize);

        // "total" counts what the filters hide, so only a full page means there is more
        Self {
            items: raw.iter().filter_map(Asset::parse).collect(),
            more: raw.len() >= limit,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Filters {
    #[serde(default)]
    pub animated: bool,
    #[serde(default)]
    pub nsfw: bool,
    #[serde(default)]
    pub humor: bool,
}

fn get(key: &ApiKey, path: &str, query: &[(&str, &str)]) -> Result<Option<Value>> {
    let mut request = AGENT
        .get(format!("{BASE}{path}"))
        .header("Authorization", format!("Bearer {}", key.as_str()));
    for (k, v) in query {
        request = request.query(k, v);
    }

    let mut response = match request.call() {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let body: Value = response
        .body_mut()
        .read_json()
        .map_err(|e| Error::Api(format!("unreadable response ({e})")))?;

    if body.get("success").and_then(Value::as_bool) == Some(false) {
        let message = body
            .get("errors")
            .and_then(|errors| errors.get(0))
            .and_then(Value::as_str)
            .unwrap_or("request failed");

        return Err(Error::Api(message.to_string()));
    }

    Ok(Some(body))
}

pub fn game_name(key: &ApiKey, app: AppId) -> Result<Option<String>> {
    let body = get(key, &format!("/games/steam/{app}"), &[])?;

    Ok(body
        .as_ref()
        .and_then(|body| body.get("data"))
        .and_then(|data| data.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string))
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Match {
    pub id: GameId,
    pub name: String,
    pub year: Option<i64>,
}

impl Match {
    fn parse(raw: &Value) -> Option<Self> {
        Some(Self {
            id: u32::try_from(raw.get("id")?.as_u64()?).ok()?.into(),
            name: raw.get("name")?.as_str().filter(|name| !name.is_empty())?.to_string(),
            year: raw.get("release_date").and_then(Value::as_i64).map(year_of),
        })
    }
}

fn year_of(unix_secs: i64) -> i64 {
    let day = unix_secs.div_euclid(86_400) + 719_468;
    let (era, day_of_era) = (day.div_euclid(146_097), day.rem_euclid(146_097));
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);

    era * 400 + year_of_era + i64::from(day_of_year >= 306)
}

fn path_segment(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => char::from(b).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn search(key: &ApiKey, term: &str) -> Result<Vec<Match>> {
    let term = term.trim();
    if term.is_empty() {
        return Ok(Vec::new());
    }

    let body = get(key, &format!("/search/autocomplete/{}", path_segment(term)), &[])?;

    Ok(body
        .as_ref()
        .and_then(|body| body.get("data"))
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(Match::parse)
        .collect())
}

const fn endpoint(kind: ArtKind) -> (&'static str, Option<&'static str>) {
    match kind {
        ArtKind::Grid => ("grids", Some("600x900,342x482,660x930")),
        ArtKind::Wide => ("grids", Some("460x215,920x430")),
        ArtKind::Hero => ("heroes", None),
        ArtKind::Logo => ("logos", None),
        ArtKind::Icon => ("icons", None),
    }
}

fn page_key(path: &str, query: &[(&str, &str)]) -> String {
    let pairs: Vec<String> = query.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{path}?{}", pairs.join("&"))
}

pub fn assets(key: &ApiKey, game: GameRef, kind: ArtKind, page: u32, filters: Filters) -> Result<Option<Page>> {
    let (endpoint, dimensions) = endpoint(kind);

    let page = page.to_string();
    let mut query = vec![
        ("types", if filters.animated { "static,animated" } else { "static" }),
        ("nsfw", if filters.nsfw { "any" } else { "false" }),
        ("humor", if filters.humor { "any" } else { "false" }),
        ("page", page.as_str()),
    ];
    if let Some(dimensions) = dimensions {
        query.push(("dimensions", dimensions));
    }

    let path = format!("/{endpoint}/{}", game.api_path());
    let cache_key = page_key(&path, &query);
    if let Some(page) = cache::lookup(&cache_key) {
        return Ok(Some(page));
    }

    let Some(body) = get(key, &path, &query)? else {
        return Ok(None);
    };

    let page = Page::parse(&body);
    cache::store(cache_key, &page);

    Ok(Some(page))
}

pub fn download(url: &AssetUrl, dest: &Path) -> Result<()> {
    let response = AGENT
        .get(url.as_str())
        .config()
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build()
        .call()?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let tmp = dest.with_extension(format!("part{nanos}"));

    let written = (|| -> io::Result<()> {
        let mut file = fs::File::create(&tmp)?;
        let copied = io::copy(&mut response.into_body().into_reader().take(MAX_DOWNLOAD), &mut file)?;
        if copied == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "empty download"));
        }

        drop(file);
        fs::rename(&tmp, dest)
    })();

    written.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Error::io("Download failed")(e)
    })
}

#[cfg(test)]
#[path = "../tests/sgdb.rs"]
mod tests;
