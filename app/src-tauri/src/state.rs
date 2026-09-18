use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::{Mutex as AsyncMutex, Notify};
use wf_core::{Catalog, Core, PriceCache, PriceSource, Store};
use wf_data::load_or_fetch;
use wf_market::{Client, Platform, PriceTable};
use wf_worldstate::WorldState;

use crate::auctions::AuctionCache;
use crate::focus::GameFocus;
use crate::images::ImageCache;
use crate::market::{ItemTable, Listings, Presence};
use crate::overlay::{OverlaySupport, Overlays};
use crate::settings::{self, MarketAccount, Settings};

pub const DATA_DIR: &str = "Merframe";

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn read<T>(cell: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    match cell.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn write<T>(cell: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    match cell.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Default)]
pub struct SharedPrices {
    cache: Mutex<PriceCache>,
}

impl SharedPrices {
    pub fn load(&self, table: &PriceTable, now: DateTime<Utc>) -> usize {
        lock(&self.cache).load(table, now)
    }

    pub fn checked(&self, now: DateTime<Utc>) {
        lock(&self.cache).checked(now);
    }

    pub fn checked_at(&self) -> Option<DateTime<Utc>> {
        lock(&self.cache).checked_at()
    }
}

impl PriceSource for SharedPrices {
    fn plat(&self, market_slug: &str) -> Option<f64> {
        lock(&self.cache).plat(market_slug)
    }

    fn plat_max_rank(&self, market_slug: &str) -> Option<f64> {
        lock(&self.cache).plat_max_rank(market_slug)
    }

    fn buy_plat(&self, market_slug: &str) -> Option<f64> {
        lock(&self.cache).buy_plat(market_slug)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InventorySource {
    #[default]
    None,
    Live,
    Cached,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GameStatus {
    pub game_detected: bool,
    pub pid: Option<u32>,
    pub scanning: bool,
    pub source: InventorySource,
    pub last_scan_at: Option<DateTime<Utc>>,
    pub last_scan_error: Option<String>,
    pub last_sync_oid: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub inventory_age_secs: Option<i64>,
    pub world_state_at: Option<DateTime<Utc>>,
    pub price_table_at: Option<DateTime<Utc>>,
    pub log_file: Option<String>,
    pub log_attached: bool,
    pub market_account: Option<MarketAccount>,
    pub market_unread: u32,
    pub overlay_support: OverlaySupport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueSession {
    pub pid: u32,
    pub clients: Option<wf_scan::HttpClients>,
}

pub type AppStateCell = OnceLock<Arc<AppState>>;

pub struct AppState {
    pub core: Mutex<Core>,
    pub market: RwLock<Arc<Client>>,
    pub market_items: AsyncMutex<Option<Arc<ItemTable>>>,
    pub market_activity: Mutex<Option<Instant>>,
    pub market_presence: RwLock<Presence>,
    pub market_presence_wake: Notify,
    pub listings: Listings,
    pub world: RwLock<Option<WorldState>>,
    pub prices: Arc<SharedPrices>,
    pub status: RwLock<GameStatus>,
    pub focus: GameFocus,
    pub settings: RwLock<Settings>,
    pub rescan: Notify,
    pub http_clients: Mutex<Option<QueueSession>>,
    pub capturing: AtomicBool,
    pub prices_wake: Notify,
    pub overlays: Overlays,
    pub auctions: AuctionCache,
    pub http: reqwest::Client,
    pub data_dir: PathBuf,
    pub images: ImageCache,
}

pub fn data_dir<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<PathBuf> {
    Ok(app
        .path()
        .data_dir()
        .context("Resolving the user data directory")?
        .join(DATA_DIR))
}

impl AppState {
    pub async fn build<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<Self> {
        let data_dir = data_dir(app)?;
        tokio::fs::create_dir_all(&data_dir)
            .await
            .with_context(|| format!("Creating {}", data_dir.display()))?;

        let http = reqwest::Client::builder()
            .user_agent(concat!("merframe/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .build()
            .context("Building the http client")?;

        let images = ImageCache::new(&data_dir, http.clone());
        images
            .prepare()
            .await
            .with_context(|| format!("Creating {}", images.dir().display()))?;

        let game_data = load_or_fetch(&data_dir.join("gamedata"), &http)
            .await
            .context("Loading the WFCD game data")?;
        let catalog = Catalog::new(game_data);
        let store =
            Store::open(&data_dir.join("merframe.sqlite")).context("Opening the local store")?;

        let settings = settings::load(app)?;
        let prices = Arc::new(SharedPrices::default());

        let price_source = Arc::clone(&prices) as Arc<dyn PriceSource + Send + Sync>;
        let core = Core::new(store, catalog, price_source, settings.alerts.clone())
            .context("Reading the local store")?;

        let mut market = Client::new(http.clone(), Platform::Pc);
        if let Some(token) = settings::token(app)? {
            market = market.with_token(token);
        }

        let status = GameStatus {
            market_account: settings::account(app)?,
            overlay_support: crate::overlay::probe(),
            ..GameStatus::default()
        };

        Ok(Self {
            core: Mutex::new(core),
            market: RwLock::new(Arc::new(market)),
            market_items: AsyncMutex::new(None),
            market_activity: Mutex::new(None),
            market_presence: RwLock::new(Presence::default()),
            market_presence_wake: Notify::new(),
            listings: Listings::default(),
            world: RwLock::new(None),
            prices,
            status: RwLock::new(status),
            focus: GameFocus::default(),
            settings: RwLock::new(settings),
            rescan: Notify::new(),
            http_clients: Mutex::new(None),
            capturing: AtomicBool::new(false),
            prices_wake: Notify::new(),
            overlays: Overlays::default(),
            auctions: AuctionCache::default(),
            http,
            data_dir,
            images,
        })
    }

    pub fn market(&self) -> Arc<Client> {
        Arc::clone(&read(&self.market))
    }

    pub fn status_snapshot(&self) -> GameStatus {
        let mut status = read(&self.status).clone();
        status.price_table_at = self.prices.checked_at();
        let core = lock(&self.core);
        match core.store().latest_snapshot() {
            Ok(Some(snapshot)) => {
                status.inventory_age_secs = Some(
                    Utc::now()
                        .signed_duration_since(snapshot.taken_at)
                        .num_seconds(),
                );
                status.last_sync_oid = Some(snapshot.last_sync_oid);
                status.last_sync_at = Some(snapshot.taken_at);
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, "Latest snapshot lookup failed"),
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use wf_core::PriceSource;

    use super::*;

    const TABLE: &str = include_str!("../../../crates/wf-market/tests/fixtures/prices_bulk.json");

    #[test]
    fn shared_prices_lookup() {
        let table = wf_market::parse_price_table(TABLE).unwrap();
        let now = DateTime::<Utc>::from_timestamp(1_757_410_800, 0).unwrap();
        let prices = SharedPrices::default();
        assert_eq!(prices.checked_at(), None);

        assert_eq!(prices.load(&table, now), 10);
        assert_eq!(prices.checked_at(), Some(now));
        assert_eq!(prices.plat("arcane_energize"), Some(55.0));
        assert_eq!(prices.plat_max_rank("arcane_energize"), Some(940.0));
        assert_eq!(prices.buy_plat("arcane_energize"), Some(40.0));
        assert_eq!(prices.plat("braton_prime_barrel"), Some(8.0));
        assert_eq!(prices.plat_max_rank("braton_prime_barrel"), None);
        assert_eq!(prices.plat("nothing_is_traded_here"), None);

        let later = now + chrono::TimeDelta::minutes(15);
        prices.checked(later);
        assert_eq!(prices.checked_at(), Some(later));
        assert_eq!(prices.plat("arcane_energize"), Some(55.0));
    }

    #[test]
    fn inventory_source_json() {
        let status = GameStatus::default();
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["source"], "none");
        assert_eq!(json["last_sync_at"], serde_json::Value::Null);

        let cached = GameStatus {
            source: InventorySource::Cached,
            ..GameStatus::default()
        };
        assert_eq!(serde_json::to_value(&cached).unwrap()["source"], "cached");
        let live = GameStatus {
            source: InventorySource::Live,
            ..GameStatus::default()
        };
        assert_eq!(serde_json::to_value(&live).unwrap()["source"], "live");
    }
}
