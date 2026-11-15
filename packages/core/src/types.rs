use std::fmt;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

use crate::error::Error;

macro_rules! numeric_id {
    ($(#[$doc:meta])* $name:ident, $label:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(u32);

        impl TryFrom<String> for $name {
            type Error = Error;

            fn try_from(text: String) -> Result<Self, Error> {
                text.parse()
                    .map(Self)
                    .map_err(|_| Error::BadRequest(format!("{} must be a number, got \"{text}\"", $label)))
            }
        }

        impl $name {
            pub const fn new(id: u32) -> Self {
                Self(id)
            }
        }

        impl From<u32> for $name {
            fn from(id: u32) -> Self {
                Self(id)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0.to_string()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

numeric_id!(AppId, "appid");

numeric_id!(UserId, "user");

numeric_id!(GameId, "game id");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum GameRef {
    Steam { appid: AppId },
    Sgdb { id: GameId },
}

impl GameRef {
    pub fn api_path(self) -> String {
        match self {
            Self::Steam { appid } => format!("steam/{appid}"),
            Self::Sgdb { id } => format!("game/{id}"),
        }
    }
}

// not Debug, so it cannot end up in a log
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(text: &str) -> Option<Self> {
        let text = text.trim();
        (!text.is_empty()).then(|| Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtKind {
    Grid,
    Wide,
    Hero,
    Logo,
    Icon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Grid,
    Wide,
    Hero,
    Logo,
}

impl Slot {
    pub const ALL: [Self; 4] = [Self::Grid, Self::Wide, Self::Hero, Self::Logo];

    pub fn file_stem(self, app: AppId) -> String {
        match self {
            Self::Grid => format!("{app}p"),
            Self::Wide => app.to_string(),
            Self::Hero => format!("{app}_hero"),
            Self::Logo => format!("{app}_logo"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpg,
    Jpeg,
    Webp,
    Gif,
}

impl ImageFormat {
    pub fn from_ext(ext: &str) -> Option<Self> {
        Some(match ext.to_ascii_lowercase().as_str() {
            "png" => Self::Png,
            "jpg" => Self::Jpg,
            "jpeg" => Self::Jpeg,
            "webp" => Self::Webp,
            "gif" => Self::Gif,
            _ => return None,
        })
    }

    pub fn of(path: &Path) -> Option<Self> {
        path.extension()?.to_str().and_then(Self::from_ext)
    }

    pub const fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Jpeg => "jpeg",
            Self::Webp => "webp",
            Self::Gif => "gif",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssetUrl(String);

impl TryFrom<String> for AssetUrl {
    type Error = Error;

    fn try_from(url: String) -> Result<Self, Error> {
        let host = url
            .strip_prefix("https://")
            .and_then(|rest| rest.split(['/', '?', '#']).next())
            .unwrap_or("");

        if host == "steamgriddb.com" || host.ends_with(".steamgriddb.com") {
            Ok(Self(url))
        } else {
            Err(Error::UntrustedUrl)
        }
    }
}

impl From<AssetUrl> for String {
    fn from(url: AssetUrl) -> Self {
        url.0
    }
}

impl AssetUrl {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn last_segment(&self) -> &str {
        let path = self.0.split(['?', '#']).next().unwrap_or("");
        path.rsplit('/').next().unwrap_or("")
    }

    pub fn ext(&self) -> Option<&str> {
        let (_, ext) = self.last_segment().rsplit_once('.')?;
        (!ext.is_empty() && ext.len() <= 5 && ext.bytes().all(|b| b.is_ascii_alphanumeric())).then_some(ext)
    }

    pub fn format(&self) -> Option<ImageFormat> {
        self.ext().and_then(ImageFormat::from_ext)
    }

    pub fn file_name(&self) -> Option<String> {
        let name: String = self
            .last_segment()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
            .collect();

        (!name.is_empty() && !name.starts_with('.')).then_some(name)
    }
}

#[cfg(test)]
#[path = "../tests/types.rs"]
mod tests;
