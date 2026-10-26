#[derive(Debug, thiserror::Error)]
pub enum WorldStateError {
    #[error("WorldState json: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Arbitration forecast line {line}: {reason}")]
    Forecast { line: usize, reason: &'static str },
    #[error("Arbitration forecast: {0}")]
    ForecastShape(&'static str),
    #[error("Arbitration tiers: {0}")]
    Tiers(&'static str),
    #[cfg(feature = "fetch")]
    #[error("WorldState fetch: {0}")]
    Fetch(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, WorldStateError>;
