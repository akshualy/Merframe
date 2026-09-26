use std::cell::RefCell;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use wf_inventory::Inventory;

use crate::catalog::DUCATS_ITEM;
use crate::delta::ItemDelta;
use crate::error::{CoreError, Result};
use crate::favourites::Favourites;
use crate::stats::AYA_ITEM;
use crate::trade::Trade;

const MIGRATIONS: [(i64, &str); 1] = [(1, include_str!("sql/0001_init.sql"))];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct SnapshotId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub taken_at: DateTime<Utc>,
    pub last_sync_oid: String,
    pub plat: i64,
    pub credits: i64,
    pub endo: i64,
    pub ducats: i64,
    pub mr: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StatPoint {
    pub snapshot: SnapshotId,
    pub at: DateTime<Utc>,
    pub plat: i64,
    pub credits: i64,
    pub endo: i64,
    pub ducats: i64,
    pub aya: Option<i64>,
    pub mr: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

impl TimeRange {
    pub fn all() -> Self {
        Self {
            from: DateTime::<Utc>::MIN_UTC,
            to: DateTime::<Utc>::MAX_UTC,
        }
    }

    pub fn since(from: DateTime<Utc>) -> Self {
        Self {
            from,
            to: DateTime::<Utc>::MAX_UTC,
        }
    }

    pub fn new(from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        Self { from, to }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredDelta {
    pub item_type: String,
    pub category: String,
    pub delta: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredTrade {
    pub id: i64,
    pub at: DateTime<Utc>,
    pub partner: Option<String>,
    pub trade: Trade,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelicOpening {
    pub id: i64,
    pub at: DateTime<Utc>,
    pub relic: String,
    pub reward_item: String,
    pub player_count: i64,
}

enum InventoryCache {
    File(PathBuf),
    Held(RefCell<Option<String>>),
}

impl InventoryCache {
    fn read(&self) -> Result<Option<String>> {
        match self {
            Self::File(path) => match std::fs::read(path) {
                Ok(compressed) => gunzip(&compressed, path).map(Some),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(source) => Err(CoreError::Read {
                    path: path.clone(),
                    source,
                }),
            },
            Self::Held(json) => Ok(json.borrow().clone()),
        }
    }

    fn write(&self, json: &str) -> Result<()> {
        match self {
            Self::File(path) => write_atomically(path, &gzip(json.as_bytes())?),
            Self::Held(held) => {
                *held.borrow_mut() = Some(json.to_owned());
                Ok(())
            }
        }
    }
}

pub struct Store {
    connection: Connection,
    inventory: InventoryCache,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        restrict(path)?;
        Self::prepare(
            connection,
            InventoryCache::File(path.with_file_name("inventory.json.gz")),
        )
    }

    pub fn in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        Self::prepare(connection, InventoryCache::Held(RefCell::new(None)))
    }

    fn prepare(connection: Connection, inventory: InventoryCache) -> Result<Self> {
        connection.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        let store = Self {
            connection,
            inventory,
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
        )?;
        let current: i64 = self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM migrations",
            [],
            |row| row.get(0),
        )?;
        let latest = MIGRATIONS[MIGRATIONS.len() - 1].0;
        if current > latest {
            return Err(CoreError::SchemaVersion(current));
        }
        let transaction = self.connection.unchecked_transaction()?;
        for (version, sql) in MIGRATIONS.iter().filter(|(version, _)| *version > current) {
            transaction.execute_batch(sql)?;
            transaction.execute(
                "INSERT INTO migrations (version, applied_at) VALUES (?1, ?2)",
                params![version, Utc::now().timestamp_millis()],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn cached_inventory(&self) -> Result<Option<String>> {
        self.inventory.read()
    }

    pub fn cache_inventory(&self, json: &str) -> Result<()> {
        self.inventory.write(json)
    }

    pub fn record_snapshot(&self, inventory: &Inventory, at: DateTime<Utc>) -> Result<SnapshotId> {
        if let Some(latest) = self.latest_snapshot()? {
            if latest.last_sync_oid == inventory.last_inventory_sync.as_str() {
                tracing::debug!(snapshot = latest.id.0, "Inventory sync unchanged");
                return Ok(latest.id);
            }
            if let (Some(stored), Some(incoming)) = (
                wf_inventory::oid_seconds(&latest.last_sync_oid),
                inventory.last_inventory_sync.seconds(),
            ) && incoming < stored
            {
                tracing::debug!(
                    snapshot = latest.id.0,
                    stored = latest.last_sync_oid,
                    incoming = inventory.last_inventory_sync.as_str(),
                    "Inventory sync is older than the newest stored snapshot"
                );
                return Ok(latest.id);
            }
        }
        self.connection.execute(
            "INSERT INTO snapshots (taken_at, last_sync_oid, plat, credits, endo, ducats, aya, mr) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                at.timestamp_millis(),
                inventory.last_inventory_sync.as_str(),
                inventory.premium_credits,
                inventory.regular_credits,
                inventory.fusion_points,
                inventory.counted(DUCATS_ITEM),
                inventory.counted(AYA_ITEM),
                i64::from(inventory.player_level),
            ],
        )?;
        Ok(SnapshotId(self.connection.last_insert_rowid()))
    }

    pub fn latest_snapshot(&self) -> Result<Option<Snapshot>> {
        let snapshot = self
            .connection
            .query_row(
                "SELECT id, taken_at, last_sync_oid, plat, credits, endo, ducats, mr \
                 FROM snapshots ORDER BY id DESC LIMIT 1",
                [],
                snapshot_from_row,
            )
            .optional()?;
        snapshot.transpose()
    }

    pub fn snapshot(&self, id: SnapshotId) -> Result<Option<Snapshot>> {
        let snapshot = self
            .connection
            .query_row(
                "SELECT id, taken_at, last_sync_oid, plat, credits, endo, ducats, mr \
                 FROM snapshots WHERE id = ?1",
                params![id.0],
                snapshot_from_row,
            )
            .optional()?;
        snapshot.transpose()
    }

    pub fn snapshot_before(&self, id: SnapshotId) -> Result<Option<Snapshot>> {
        let snapshot = self
            .connection
            .query_row(
                "SELECT id, taken_at, last_sync_oid, plat, credits, endo, ducats, mr \
                 FROM snapshots WHERE id < ?1 ORDER BY id DESC LIMIT 1",
                params![id.0],
                snapshot_from_row,
            )
            .optional()?;
        snapshot.transpose()
    }

    pub fn stats_series(&self, range: TimeRange) -> Result<Vec<StatPoint>> {
        let mut statement = self.connection.prepare(
            "SELECT id, taken_at, plat, credits, endo, ducats, aya, mr FROM snapshots \
             WHERE taken_at >= ?1 AND taken_at <= ?2 ORDER BY taken_at",
        )?;
        let rows = statement.query_map(
            params![range.from.timestamp_millis(), range.to.timestamp_millis()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )?;
        let mut points = Vec::new();
        for row in rows {
            let (id, taken_at, plat, credits, endo, ducats, aya, mr) = row?;
            points.push(StatPoint {
                snapshot: SnapshotId(id),
                at: datetime(taken_at)?,
                plat,
                credits,
                endo,
                ducats,
                aya,
                mr,
            });
        }
        Ok(points)
    }

    pub fn record_deltas(
        &self,
        from: SnapshotId,
        to: SnapshotId,
        deltas: &[ItemDelta],
    ) -> Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO deltas (from_snapshot, to_snapshot, item_type, category, delta) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for delta in deltas {
                statement.execute(params![
                    from.0,
                    to.0,
                    delta.item_type,
                    delta.category,
                    delta.delta()
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn deltas(&self, to: SnapshotId) -> Result<Vec<StoredDelta>> {
        let mut statement = self.connection.prepare(
            "SELECT item_type, category, delta FROM deltas WHERE to_snapshot = ?1 ORDER BY id",
        )?;
        let deltas = statement
            .query_map(params![to.0], |row| {
                Ok(StoredDelta {
                    item_type: row.get(0)?,
                    category: row.get(1)?,
                    delta: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<StoredDelta>>>()?;
        Ok(deltas)
    }

    pub fn record_trade(
        &self,
        at: DateTime<Utc>,
        partner: Option<&str>,
        trade: &Trade,
    ) -> Result<i64> {
        let items_json = serde_json::to_string(trade)?;
        self.connection.execute(
            "INSERT INTO trades (at, partner, items_json, plat) VALUES (?1, ?2, ?3, ?4)",
            params![at.timestamp_millis(), partner, items_json, trade.plat],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn trades(&self, range: TimeRange) -> Result<Vec<StoredTrade>> {
        let mut statement = self.connection.prepare(
            "SELECT id, at, partner, items_json FROM trades \
             WHERE at >= ?1 AND at <= ?2 ORDER BY at",
        )?;
        let rows = statement.query_map(
            params![range.from.timestamp_millis(), range.to.timestamp_millis()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        let mut trades = Vec::new();
        for row in rows {
            let (id, at, partner, items_json) = row?;
            trades.push(StoredTrade {
                id,
                at: datetime(at)?,
                partner,
                trade: serde_json::from_str(&items_json)?,
            });
        }
        Ok(trades)
    }

    pub fn record_relic_opening(
        &self,
        at: DateTime<Utc>,
        relic: &str,
        reward_item: &str,
        player_count: usize,
    ) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO relic_openings (at, relic, reward_item, player_count) \
             VALUES (?1, ?2, ?3, ?4)",
            params![at.timestamp_millis(), relic, reward_item, player_count],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn relic_openings(&self, range: TimeRange) -> Result<Vec<RelicOpening>> {
        let mut statement = self.connection.prepare(
            "SELECT id, at, relic, reward_item, player_count FROM relic_openings \
             WHERE at >= ?1 AND at <= ?2 ORDER BY at",
        )?;
        let rows = statement.query_map(
            params![range.from.timestamp_millis(), range.to.timestamp_millis()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )?;
        let mut openings = Vec::new();
        for row in rows {
            let (id, at, relic, reward_item, player_count) = row?;
            openings.push(RelicOpening {
                id,
                at: datetime(at)?,
                relic,
                reward_item,
                player_count,
            });
        }
        Ok(openings)
    }

    pub fn favourites(&self) -> Result<Favourites> {
        let mut statement = self
            .connection
            .prepare("SELECT unique_name FROM favourites")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Favourites>>()?)
    }

    pub fn toggle_favourite(&self, unique_name: &str, at: DateTime<Utc>) -> Result<bool> {
        let removed = self.connection.execute(
            "DELETE FROM favourites WHERE unique_name = ?1",
            params![unique_name],
        )?;
        if removed > 0 {
            return Ok(false);
        }
        self.connection.execute(
            "INSERT INTO favourites (unique_name, since) VALUES (?1, ?2)",
            params![unique_name, at.timestamp_millis()],
        )?;
        Ok(true)
    }

    pub fn setting<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let raw: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        match raw {
            Some(value) => Ok(Some(serde_json::from_str(&value)?)),
            None => Ok(None),
        }
    }

    pub fn set_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let encoded = serde_json::to_string(value)?;
        self.connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, encoded],
        )?;
        Ok(())
    }
}

fn snapshot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Snapshot>> {
    let id: i64 = row.get(0)?;
    let taken_at: i64 = row.get(1)?;
    let last_sync_oid: String = row.get(2)?;
    let plat: i64 = row.get(3)?;
    let credits: i64 = row.get(4)?;
    let endo: i64 = row.get(5)?;
    let ducats: i64 = row.get(6)?;
    let mr: i64 = row.get(7)?;
    Ok(datetime(taken_at).map(|taken_at| Snapshot {
        id: SnapshotId(id),
        taken_at,
        last_sync_oid,
        plat,
        credits,
        endo,
        ducats,
        mr,
    }))
}

fn datetime(millis: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp_millis(millis).ok_or(CoreError::Timestamp(millis))
}

fn gzip(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).map_err(CoreError::Compress)?;
    encoder.finish().map_err(CoreError::Compress)
}

fn gunzip(bytes: &[u8], path: &Path) -> Result<String> {
    let mut decoder = GzDecoder::new(bytes);
    let mut out = String::new();
    std::io::Read::read_to_string(&mut decoder, &mut out).map_err(|source| {
        CoreError::Decompress {
            path: path.to_path_buf(),
            source,
        }
    })?;
    Ok(out)
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let partial = path.with_extension("part");
    std::fs::write(&partial, bytes).map_err(|source| CoreError::Io {
        path: partial.clone(),
        source,
    })?;
    restrict(&partial)?;
    std::fs::rename(&partial, path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(unix)]
fn restrict(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|source| {
        CoreError::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::delta;

    fn at(millis: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(millis).unwrap()
    }

    #[test]
    fn snapshot_round_trip() {
        let store = Store::in_memory().unwrap();
        let inventory = fixtures::inventory();
        let id = store.record_snapshot(&inventory, at(1_000_000)).unwrap();

        let latest = store.latest_snapshot().unwrap().unwrap();
        assert_eq!(latest.id, id);
        assert_eq!(latest.last_sync_oid, "6a9eeb1f000000000000c001");
        assert_eq!(latest.plat, 5249);
        assert_eq!(latest.credits, 90_814_797);
        assert_eq!(latest.endo, 189_082);
        assert_eq!(latest.ducats, 937);
        assert_eq!(latest.mr, 14);
        assert_eq!(latest.taken_at, at(1_000_000));
    }

    #[test]
    fn unchanged_sync_oid() {
        let store = Store::in_memory().unwrap();
        let inventory = fixtures::inventory();
        let first = store.record_snapshot(&inventory, at(1_000_000)).unwrap();
        let second = store.record_snapshot(&inventory, at(2_000_000)).unwrap();
        assert_eq!(first, second);
        assert_eq!(store.stats_series(TimeRange::all()).unwrap().len(), 1);
    }

    #[test]
    fn older_sync_oid_refused() {
        let store = Store::in_memory().unwrap();
        let newer_json =
            fixtures::INVENTORY.replacen("6a9eeb1f000000000000c001", "6a9fb57d000000000000c101", 1);
        let newer = Inventory::parse(&newer_json).unwrap();
        let newest = store.record_snapshot(&newer, at(2_000_000)).unwrap();

        let older = fixtures::inventory();
        assert_eq!(
            older.last_inventory_sync.as_str(),
            "6a9eeb1f000000000000c001"
        );
        let reused = store.record_snapshot(&older, at(3_000_000)).unwrap();
        assert_eq!(reused, newest);
        assert_eq!(store.stats_series(TimeRange::all()).unwrap().len(), 1);
        assert_eq!(
            store.latest_snapshot().unwrap().unwrap().last_sync_oid,
            "6a9fb57d000000000000c101"
        );
    }

    #[test]
    fn sync_oid_in_same_second() {
        let store = Store::in_memory().unwrap();
        let first = store
            .record_snapshot(&fixtures::inventory(), at(1_000_000))
            .unwrap();
        let sibling_json =
            fixtures::INVENTORY.replacen("6a9eeb1f000000000000c001", "6a9eeb1f000000000000c000", 1);
        let sibling = Inventory::parse(&sibling_json).unwrap();
        let second = store.record_snapshot(&sibling, at(1_000_500)).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn changed_sync_oid_appends() {
        let store = Store::in_memory().unwrap();
        let inventory = fixtures::inventory();
        let first = store.record_snapshot(&inventory, at(1_000_000)).unwrap();

        let changed_json =
            fixtures::INVENTORY.replacen("6a9eeb1f000000000000c001", "6a9eeb1f000000000000c002", 1);
        let changed = Inventory::parse(&changed_json).unwrap();
        let second = store.record_snapshot(&changed, at(2_000_000)).unwrap();
        assert_ne!(first, second);

        let series = store.stats_series(TimeRange::all()).unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].at, at(1_000_000));
        assert_eq!(series[1].snapshot, second);

        let narrowed = store.stats_series(TimeRange::since(at(1_500_000))).unwrap();
        assert_eq!(narrowed.len(), 1);
        assert_eq!(store.snapshot_before(second).unwrap().unwrap().id, first);
    }

    #[test]
    fn deltas_per_snapshot_pair() {
        let store = Store::in_memory().unwrap();
        let inventory = fixtures::inventory();
        let first = store.record_snapshot(&inventory, at(1_000_000)).unwrap();
        let changed_json =
            fixtures::INVENTORY.replacen("6a9eeb1f000000000000c001", "6a9eeb1f000000000000c002", 1);
        let changed = Inventory::parse(&changed_json).unwrap();
        let second = store.record_snapshot(&changed, at(2_000_000)).unwrap();

        let deltas = vec![delta::ItemDelta {
            item_type: String::from("/Lotus/Types/Items/MiscItems/Ferrite"),
            category: String::from("MiscItems"),
            before: 10,
            after: 25,
        }];
        store.record_deltas(first, second, &deltas).unwrap();
        let stored = store.deltas(second).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].delta, 15);
        assert_eq!(stored[0].category, "MiscItems");
        assert!(store.deltas(first).unwrap().is_empty());
    }

    #[test]
    fn reopen_on_disk_store() {
        let dir = std::env::temp_dir().join("wf-core-store-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("merframe.sqlite3");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }

        let inventory = fixtures::inventory();
        let id = {
            let store = Store::open(&path).unwrap();
            store.record_snapshot(&inventory, at(1_000_000)).unwrap()
        };

        let reopened = Store::open(&path).unwrap();
        let latest = reopened.latest_snapshot().unwrap().unwrap();
        assert_eq!(latest.id, id);
        assert_eq!(
            reopened.record_snapshot(&inventory, at(3_000_000)).unwrap(),
            id
        );
        drop(reopened);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn inventory_cache_file() {
        let dir = std::env::temp_dir().join("wf-core-store-inventory-cache");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("merframe.sqlite");
        let cache = dir.join("inventory.json.gz");

        let store = Store::open(&path).unwrap();
        assert!(store.cached_inventory().unwrap().is_none());
        store.cache_inventory(fixtures::INVENTORY).unwrap();
        assert!(!dir.join("inventory.json.part").exists());
        assert!(std::fs::read(&cache).unwrap().starts_with(&[0x1f, 0x8b]));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            let mode = std::fs::metadata(&cache).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        drop(store);

        let reopened = Store::open(&path).unwrap();
        assert_eq!(
            reopened.cached_inventory().unwrap().as_deref(),
            Some(fixtures::INVENTORY)
        );
        drop(reopened);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn trades_and_relic_openings() {
        let store = Store::in_memory().unwrap();
        let (partner, trade) = crate::trade::parse_trade_description(
            "Are you sure you want to accept this trade? You are offering\nForma Blueprint x 2\nand will receive from SomePlayer the following:\nPlatinum x 45\n",
        )
        .unwrap();
        store
            .record_trade(at(5_000_000), partner.as_deref(), &trade)
            .unwrap();
        let trades = store.trades(TimeRange::all()).unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].partner.as_deref(), Some("SomePlayer"));
        assert_eq!(trades[0].trade.plat, 45);
        assert_eq!(trades[0].trade.offered[0].count, 2);

        store
            .record_relic_opening(at(6_000_000), "Lith K12", "Forma Blueprint", 4)
            .unwrap();
        let openings = store.relic_openings(TimeRange::all()).unwrap();
        assert_eq!(openings.len(), 1);
        assert_eq!(openings[0].relic, "Lith K12");
        assert_eq!(openings[0].player_count, 4);
    }

    #[test]
    fn aya_in_series() {
        let store = Store::in_memory().unwrap();
        store
            .record_snapshot(&fixtures::inventory(), at(1_000_000))
            .unwrap();
        let series = store.stats_series(TimeRange::all()).unwrap();
        assert_eq!(series[0].aya, Some(11));
        assert_eq!(series[0].ducats, 937);
    }

    #[test]
    fn typed_settings() {
        let store = Store::in_memory().unwrap();
        assert_eq!(store.setting::<Vec<String>>("tiers").unwrap(), None);
        store
            .set_setting("tiers", &vec!["Lith".to_owned(), "Meso".to_owned()])
            .unwrap();
        store.set_setting("tiers", &vec!["Axi".to_owned()]).unwrap();
        assert_eq!(
            store.setting::<Vec<String>>("tiers").unwrap(),
            Some(vec!["Axi".to_owned()])
        );
    }

    #[test]
    fn riven_data_setting_round_trip() {
        const RIVEN_DATA: &str = include_str!("../../../fixtures/riven_data.json");
        let document = wf_market::parse_riven_data(RIVEN_DATA).expect("riven data");
        let store = Store::in_memory().unwrap();
        store.set_setting("riven_data", &document).unwrap();
        let read = store
            .setting::<wf_market::RivenData>("riven_data")
            .unwrap()
            .unwrap();
        assert_eq!(read, document);
    }

    #[test]
    fn favourites_toggle_and_reopen() {
        let dir = std::env::temp_dir().join("wf-core-store-favourites-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("merframe.sqlite3");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        let mirage = "/Lotus/Powersuits/Mirage/MirageWarframe";
        let styanax = "/Lotus/Types/Recipes/WarframeRecipes/StyanaxPrimeBlueprint";
        {
            let store = Store::open(&path).unwrap();
            assert!(store.favourites().unwrap().is_empty());
            assert!(store.toggle_favourite(mirage, at(1_000_000)).unwrap());
            assert!(store.toggle_favourite(styanax, at(1_000_001)).unwrap());
            assert!(!store.toggle_favourite(mirage, at(1_000_002)).unwrap());
            let favourites = store.favourites().unwrap();
            assert_eq!(favourites.len(), 1);
            assert!(favourites.contains(styanax));
            assert!(!favourites.contains(mirage));
        }

        let reopened = Store::open(&path).unwrap();
        let favourites = reopened.favourites().unwrap();
        assert_eq!(favourites.names().collect::<Vec<_>>(), [styanax]);
        let since: i64 = reopened
            .connection
            .query_row(
                "SELECT since FROM favourites WHERE unique_name = ?1",
                params![styanax],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(since, 1_000_001);
        drop(reopened);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn migrate_twice() {
        let store = Store::in_memory().unwrap();
        store.migrate().unwrap();
        let applied: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(applied, i64::try_from(MIGRATIONS.len()).unwrap());
    }
}
