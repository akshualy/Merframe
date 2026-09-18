use std::collections::HashMap;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use tokio::sync::Mutex;
use wf_market::{Etagged, MarketError, WeaponAuctions, riven_auctions};

struct Cached {
    client: Etagged<WeaponAuctions>,
    document: Option<WeaponAuctions>,
    checked_at: Option<Instant>,
}

#[derive(Default)]
pub struct AuctionCache {
    weapons: Mutex<HashMap<String, Cached>>,
}

impl AuctionCache {
    pub async fn weapon(
        &self,
        http: &reqwest::Client,
        weapon_slug: &str,
    ) -> Result<Option<WeaponAuctions>, MarketError> {
        let mut weapons = self.weapons.lock().await;
        let cached = weapons
            .entry(weapon_slug.to_owned())
            .or_insert_with(|| Cached {
                client: riven_auctions(http.clone(), weapon_slug),
                document: None,
                checked_at: None,
            });
        let fresh = match cached.checked_at {
            Some(at) => at.elapsed() < Duration::from_secs(600),
            None => false,
        };
        if !fresh {
            match cached.client.fetch().await {
                Ok(Some(document)) => cached.document = Some(document),
                Ok(None) => {}
                Err(MarketError::Http(StatusCode::NOT_FOUND, _)) => cached.document = None,
                Err(error) => return Err(error),
            }
            cached.checked_at = Some(Instant::now());
        }
        Ok(cached.document.clone())
    }
}
