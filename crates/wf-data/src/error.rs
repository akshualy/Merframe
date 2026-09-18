pub type Result<T> = std::result::Result<T, DataError>;

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("{0} json: {1}")]
    Parse(&'static str, #[source] serde_json::Error),
    #[cfg(feature = "fetch")]
    #[error("Download: {0}")]
    Network(#[source] reqwest::Error),
    #[cfg(feature = "fetch")]
    #[error("Cache file {path}: {source}")]
    CacheIo {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[cfg(feature = "fetch")]
    #[error("Cache metadata json: {0}")]
    ParseCacheMeta(#[source] serde_json::Error),
    #[cfg(feature = "fetch")]
    #[error("No cached copy of {0} and the download failed")]
    NoCacheAvailable(String),
}
