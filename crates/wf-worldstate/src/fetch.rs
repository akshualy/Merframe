use std::time::Duration;

use crate::Result;
use crate::{WORLD_STATE_URL, WorldState};

pub async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String> {
    Ok(client
        .get(url)
        .timeout(Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?)
}

pub async fn fetch(client: &reqwest::Client) -> Result<WorldState> {
    WorldState::parse(&fetch_text(client, WORLD_STATE_URL).await?)
}
