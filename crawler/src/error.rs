use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("API error: code={code}, message={message}")]
    Api { code: i32, message: String },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("WBI signing error: {0}")]
    Wbi(String),

    #[error("Protobuf decode error: {0}")]
    Protobuf(#[from] prost::DecodeError),

    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<u64> },

    #[error("Max retries exceeded for video {bvid}")]
    MaxRetries { bvid: String },

    #[error("Config error: {0}")]
    Config(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, CrawlerError>;
