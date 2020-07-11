use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error retrieving page")]
    Http {
        #[from]
        err: reqwest::Error,
    },
    #[error("Error parsing response")]
    Serde {
        #[from]
        err: serde_json::Error,
    },
    #[error("Error parsing bincoded data")]
    Bincode {
        #[from]
        err: bincode::Error,
    },
    #[error("Error performing file IO")]
    Io {
        #[from]
        err: std::io::Error,
    },
    #[error("Error performing sled operation")]
    Sled {
        #[from]
        err: sled::Error,
    },
    #[error("Cache entry not found in sled DB")]
    CacheEntryNotFound,
    #[error("Could not convert UTF8 string")]
    Utf8Error {
        #[from]
        err: std::str::Utf8Error,
    },
}
