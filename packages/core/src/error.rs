use std::io;

use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong; the message is shown to the user as it is
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("No SteamGridDB API key set. Add one under \"API key…\".")]
    NoApiKey,

    #[error("SteamGridDB rejected the API key. Check it under \"API key…\".")]
    KeyRejected,

    #[error("SteamGridDB rate limit reached. Wait a moment and try again.")]
    RateLimited,

    #[error("SteamGridDB returned HTTP {0}")]
    Status(u16),

    #[error("SteamGridDB: {0}")]
    Api(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Steam was not found: {0}")]
    SteamNotFound(String),

    #[error("Steam cannot use \".{0}\" files as library art")]
    UnsupportedFormat(String),

    #[error("Refusing to download from a host that is not steamgriddb.com")]
    UntrustedUrl,

    #[error("{context}: {source}")]
    Io { context: String, source: io::Error },

    #[error("internal error in sgdb_core")]
    Internal,
}

impl Error {
    /// For `map_err` on an I/O result: `fs::write(..).map_err(Error::io("saving the config"))`
    pub fn io(context: impl Into<String>) -> impl FnOnce(io::Error) -> Self {
        let context = context.into();
        move |source| Self::Io { context, source }
    }

    /// Stable identifier the frontend switches on, so it never has to read the message
    pub const fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::NoApiKey => "no_api_key",
            Self::KeyRejected => "key_rejected",
            Self::RateLimited => "rate_limited",
            Self::Status(_) | Self::Api(_) => "api",
            Self::Network(_) => "network",
            Self::SteamNotFound(_) => "steam_not_found",
            Self::UnsupportedFormat(_) => "unsupported_format",
            Self::UntrustedUrl => "untrusted_url",
            Self::Io { .. } => "io",
            Self::Internal => "internal",
        }
    }
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::StatusCode(401 | 403) => Self::KeyRejected,
            ureq::Error::StatusCode(429) => Self::RateLimited,
            ureq::Error::StatusCode(code) => Self::Status(code),
            other => Self::Network(other.to_string()),
        }
    }
}

#[cfg(test)]
#[path = "../tests/error.rs"]
mod tests;
