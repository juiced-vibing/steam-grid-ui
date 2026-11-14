use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::io::{self};
use std::path::Path;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use ureq::Agent;

use crate::error::Error;
use crate::error::Result;
use crate::types::ApiKey;
use crate::types::AppId;
use crate::types::ArtKind;
use crate::types::AssetUrl;

const BASE: &str = "https://www.steamgriddb.com/api/v2";
const USER_AGENT: &str = "SteamGridUI/0.1";
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;

const API_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(180);

const DEFAULT_LIMIT: usize = 50;

const PAGE_TTL: Duration = Duration::from_secs(600);
const PAGE_CACHE_MAX: usize = 128;

static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::config_builder()
        .timeout_global(Some(API_TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into()
});

static PAGES: LazyLock<Mutex<HashMap<String, (Instant, Page)>>> = LazyLock::new(Mutex::default);

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

fn cached_page(cache_key: &str) -> Option<Page> {
    let pages = PAGES.lock().unwrap_or_else(PoisonError::into_inner);
    pages
        .get(cache_key)
        .filter(|(at, _)| at.elapsed() < PAGE_TTL)
        .map(|(_, page)| page.clone())
}

fn remember_page(cache_key: String, page: &Page) {
    let mut pages = PAGES.lock().unwrap_or_else(PoisonError::into_inner);
    if pages.len() >= PAGE_CACHE_MAX {
        pages.retain(|_, (at, _)| at.elapsed() < PAGE_TTL);
    }
    if pages.len() >= PAGE_CACHE_MAX {
        pages.clear();
    }

    pages.insert(cache_key, (Instant::now(), page.clone()));
}

pub fn forget_pages() {
    PAGES.lock().unwrap_or_else(PoisonError::into_inner).clear();
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

pub fn assets(key: &ApiKey, app: AppId, kind: ArtKind, page: u32, filters: Filters) -> Result<Option<Page>> {
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

    let path = format!("/{endpoint}/steam/{app}");
    let cache_key = page_key(&path, &query);
    if let Some(page) = cached_page(&cache_key) {
        return Ok(Some(page));
    }

    let Some(body) = get(key, &path, &query)? else {
        return Ok(None);
    };

    let page = Page::parse(&body);
    remember_page(cache_key, &page);

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
