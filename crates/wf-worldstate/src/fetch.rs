use std::time::Duration;

use crate::Result;
use crate::{WORLD_STATE_URL, WorldState};

pub async fn fetch_body(client: &reqwest::Client) -> Result<String> {
    Ok(client
        .get(WORLD_STATE_URL)
        .timeout(Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?)
}

pub async fn fetch(client: &reqwest::Client) -> Result<WorldState> {
    WorldState::parse(&fetch_body(client).await?)
}
