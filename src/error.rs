use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error retrieving page")]
    Http {
        err: reqwest::Error,
        url: String,
        params: Vec<(String, String)>,
    },
    #[error("Error parsing HTTP response")]
    HttpParse {
        err: serde_json::Error,
        url: String,
        params: Vec<(String, String)>,
    },
    #[error("API error")]
    ApiError {
        err: u32,
        url: String,
        params: Vec<(String, String)>,
    },
    #[error("Empty detailed stats")]
    DetailedStats {
        url: String,
        params: Vec<(String, String)>,
        status: String,
        meta: Option<crate::wows_data::GenericReplyMeta>,
        error: Option<crate::wows_data::GenericReplyError>,
    },
    #[error("Error parsing response")]
    Serde {
        #[from]
        err: serde_json::Error,
    },
    #[error("Error performing file IO")]
    Io {
        #[from]
        err: std::io::Error,
    },
    #[error("Cache entry not found in sled DB")]
    CacheEntryNotFound,
    #[error("Could not convert UTF8 string")]
    Utf8Error {
        #[from]
        err: std::str::Utf8Error,
    },
}
