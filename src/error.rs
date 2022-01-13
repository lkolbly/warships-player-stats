use thiserror::Error;
use tracing::*;

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
    #[error("Could not convert UTF8 string")]
    Utf8Error {
        #[from]
        err: std::str::Utf8Error,
    },
}

pub trait DroppableError {
    type OkValue;
    type ErrValue;

    fn log_and_drop_error<F: FnOnce(Self::ErrValue)>(self, cb: F) -> Option<Self::OkValue>;
}

impl<T, E> DroppableError for core::result::Result<T, E> {
    type OkValue = T;
    type ErrValue = E;

    fn log_and_drop_error<F: FnOnce(Self::ErrValue)>(self, cb: F) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                cb(e);
                None
            }
        }
    }
}
