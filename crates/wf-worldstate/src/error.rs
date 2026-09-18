#[derive(Debug, thiserror::Error)]
pub enum WorldStateError {
    #[error("WorldState json: {0}")]
    Parse(#[from] serde_json::Error),
    #[cfg(feature = "fetch")]
    #[error("WorldState fetch: {0}")]
    Fetch(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, WorldStateError>;
