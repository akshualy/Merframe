use std::collections::HashMap;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use tokio::sync::Mutex;
use wf_market::{Etagged, MarketError, WeaponAuctions, riven_auctions};

const CHECK_PAUSE: Duration = Duration::from_secs(600);
const CRAWL_PAUSE: Duration = Duration::from_secs(30);

struct Cached {
    client: Etagged<WeaponAuctions>,
    document: Option<WeaponAuctions>,
    fresh_until: Option<Instant>,
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
                fresh_until: None,
            });

        let fresh = cached
            .fresh_until
            .is_some_and(|until| Instant::now() < until);
        if !fresh {
            let pause = match cached.client.fetch().await {
                Ok(Some(document)) => {
                    cached.document = Some(document);
                    CHECK_PAUSE
                }
                Ok(None) => CHECK_PAUSE,
                Err(MarketError::Http(StatusCode::NOT_FOUND, _)) => {
                    cached.document = None;
                    CHECK_PAUSE
                }
                Err(MarketError::Http(StatusCode::SERVICE_UNAVAILABLE, _)) => {
                    cached.document = None;
                    CRAWL_PAUSE
                }
                Err(error) => return Err(error),
            };
            cached.fresh_until = Some(Instant::now() + pause);
        }

        Ok(cached.document.clone())
    }
}
