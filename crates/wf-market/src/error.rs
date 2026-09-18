use std::time::Duration;

use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum MarketError {
    #[error("Warframe.market {0}: {1}")]
    Http(StatusCode, String),
    #[error("Rate limited, next request in {} s", .0.as_secs())]
    RateLimited(Duration),
    #[error("Unauthorized")]
    Unauthorized,
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
    #[error("Response without a data field")]
    MissingData,
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    #[error("Websocket: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Login response without a JWT authorization header")]
    MissingJwt,
    #[error("Request limiter closed")]
    LimiterClosed,
}

impl MarketError {
    pub fn brief(&self) -> String {
        match self {
            Self::Http(status, body) => format!("Warframe.market {status}, {} bytes", body.len()),
            error => error.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, MarketError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brief_error() {
        let error = MarketError::Http(
            StatusCode::BAD_GATEWAY,
            "<html>upstream is down</html>".to_owned(),
        );
        assert_eq!(error.brief(), "Warframe.market 502 Bad Gateway, 29 bytes");
        assert_eq!(
            error.to_string(),
            "Warframe.market 502 Bad Gateway: <html>upstream is down</html>"
        );
        assert_eq!(MarketError::Unauthorized.brief(), "Unauthorized");
        assert_eq!(
            MarketError::RateLimited(Duration::from_secs(30)).brief(),
            "Rate limited, next request in 30 s"
        );
    }
}
