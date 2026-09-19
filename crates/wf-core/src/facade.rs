use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use wf_inventory::Inventory;
use wf_log::Event as LogEvent;
use wf_market::{RivenAttribute, RivenData, WeaponAuctions};
use wf_worldstate::WorldState;

use crate::catalog::{Catalog, REQUIEM_MARKER};
use crate::comparables::{self, ComparedStat, RivenComparables};
use crate::delta;
use crate::error::Result;
use crate::events::{self, AlertSettings, CoreEvent, Engine};
use crate::export::{ExportBundle, export};
use crate::favourites::Favourites;
use crate::foundry::{self, FoundryTab};
use crate::inventory_view::{self, InventoryTab};
use crate::listings::MarketListings;
use crate::mastery::{self, MasteryOptions, MasteryTab};
use crate::prices::PriceSource;
use crate::relic_planner::{self, MissingPart, RelicPlan, RelicSource, RewardScreen};
use crate::resources::{self, ResourcesTab};
use crate::rivens::{Grader, RivenRow, RivensTab};
use crate::stats::{self, DailyCount, StatsSummary};
use crate::store::{
    RelicOpening, Snapshot, SnapshotId, StatPoint, Store, StoredDelta, StoredTrade, TimeRange,
};
use crate::view::View;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RelicPlannerTab {
    pub squad_size: u32,
    pub only_owned: bool,
    pub void_traces: i64,
    pub missing_parts: Vec<MissingPart>,
    pub plans: Vec<RelicPlan>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatsTab {
    pub series: Vec<StatPoint>,
    pub trades: Vec<StoredTrade>,
    pub relic_openings: Vec<RelicOpening>,
    pub latest_deltas: Vec<StoredDelta>,
    pub relics_per_day: Vec<DailyCount>,
    pub trades_per_day: Vec<DailyCount>,
    pub days_played: Vec<DailyCount>,
    pub summary: Option<StatsSummary>,
}

pub struct Core {
    store: Store,
    catalog: Catalog,
    prices: Arc<dyn PriceSource + Send + Sync>,
    engine: Engine,
    inventory: Option<Inventory>,
    world: Option<WorldState>,
    riven_attributes: Vec<RivenAttribute>,
    riven_data: Option<RivenData>,
    latest_snapshot: Option<SnapshotId>,
    own_relic_reward: Option<String>,
    reward_screen_recorded: bool,
    favourites: Favourites,
    pending_rerolls: HashMap<String, String>,
    listings: MarketListings,
}

impl Core {
    pub fn new(
        store: Store,
        catalog: Catalog,
        prices: Arc<dyn PriceSource + Send + Sync>,
        alerts: AlertSettings,
    ) -> Result<Self> {
        let favourites = store.favourites()?;
        Ok(Self {
            store,
            catalog,
            prices,
            engine: Engine::new(alerts),
            inventory: None,
            world: None,
            riven_attributes: Vec::new(),
            riven_data: None,
            latest_snapshot: None,
            own_relic_reward: None,
            reward_screen_recorded: false,
            favourites,
            pending_rerolls: HashMap::new(),
            listings: MarketListings::default(),
        })
    }

    pub fn set_alert_settings(&mut self, settings: AlertSettings) {
        self.engine.set_settings(settings);
    }

    pub fn set_riven_attributes(&mut self, attributes: Vec<RivenAttribute>) {
        self.riven_attributes = attributes;
    }

    pub fn set_riven_data(&mut self, data: RivenData) {
        self.riven_data = Some(data);
    }

    pub fn set_market_listings(&mut self, listings: MarketListings) {
        self.listings = listings;
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn toggle_favourite(&mut self, unique_name: &str) -> Result<bool> {
        let favourite = self.store.toggle_favourite(unique_name, Utc::now())?;
        self.favourites = self.store.favourites()?;
        Ok(favourite)
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn inventory(&self) -> Option<&Inventory> {
        self.inventory.as_ref()
    }

    pub fn load_snapshot(&mut self) -> Result<Option<Snapshot>> {
        let Some(snapshot) = self.store.latest_snapshot()? else {
            return Ok(None);
        };
        let Some(json) = self.store.cached_inventory()? else {
            return Ok(None);
        };
        let inventory = Inventory::parse(&json)?;
        tracing::info!(
            snapshot = snapshot.id.0,
            last_sync = snapshot.last_sync_oid,
            "Snapshot restored as the current inventory"
        );
        self.latest_snapshot = Some(snapshot.id);
        self.inventory = Some(inventory);
        Ok(Some(snapshot))
    }

    pub fn ingest_inventory(&mut self, json: &str, now: DateTime<Utc>) -> Result<Vec<CoreEvent>> {
        let inventory = Inventory::parse(json)?;
        let previous = self.store.latest_snapshot()?;
        let id = self.store.record_snapshot(&inventory, now)?;
        let mut changes = 0;
        if previous.as_ref().is_none_or(|snapshot| snapshot.id != id) {
            if let Some(previous) = previous
                && let Some(cached) = self.store.cached_inventory()?
            {
                let deltas = delta::compute(&Inventory::parse(&cached)?, &inventory);
                self.store.record_deltas(previous.id, id, &deltas)?;
                changes = deltas.len();
            }
            self.store.cache_inventory(json)?;
        }
        tracing::info!(snapshot = id.0, changes, "Inventory ingested");
        self.latest_snapshot = Some(id);
        let events = events::inventory_events(&inventory, changes);
        self.inventory = Some(inventory);
        Ok(events)
    }

    pub fn ingest_world_state(&mut self, json: &str, now: DateTime<Utc>) -> Result<Vec<CoreEvent>> {
        let world = WorldState::parse(json)?;
        let events = self.engine.handle_world_state(&world, now);
        self.world = Some(world);
        Ok(events)
    }

    pub fn evaluate_world_state(&mut self, now: DateTime<Utc>) -> Vec<CoreEvent> {
        let Some(world) = self.world.as_ref() else {
            return Vec::new();
        };
        self.engine.handle_world_state(world, now)
    }

    pub fn handle_log_event(
        &mut self,
        event: &LogEvent,
        now: DateTime<Utc>,
    ) -> Result<Vec<CoreEvent>> {
        if let LogEvent::OwnRelicReward { store_item, .. } = event {
            self.own_relic_reward = Some(store_item.clone());
        }
        if matches!(event, LogEvent::RelicRewardScreenOpened) {
            self.reward_screen_recorded = false;
        }
        let events = self.engine.handle_log_event(event, now);
        self.record(&events, now)?;
        Ok(events)
    }

    pub fn reward_screen_generation(&self) -> u64 {
        self.engine.reward_screen_generation()
    }

    pub fn reward_player_count(&self) -> usize {
        self.engine.reward_player_count()
    }

    pub fn start_reward_scan(&mut self) -> bool {
        self.engine.start_reward_scan()
    }

    pub fn handle_reward_screen(
        &mut self,
        now: DateTime<Utc>,
        generation: u64,
        rewards: Vec<String>,
    ) -> Result<Vec<CoreEvent>> {
        let events = self.engine.handle_reward_screen(generation, rewards);
        self.record(&events, now)?;
        Ok(events)
    }

    fn record(&mut self, events: &[CoreEvent], now: DateTime<Utc>) -> Result<()> {
        for event in events {
            match event {
                CoreEvent::TradeCompleted { at, partner, trade } => {
                    self.store.record_trade(*at, partner.as_deref(), trade)?;
                }
                CoreEvent::RelicRewardScreen { relic, rewards } => {
                    if let (Some(relic), Some(reward)) = (relic, &self.own_relic_reward)
                        && !relic.contains(REQUIEM_MARKER)
                        && !std::mem::replace(&mut self.reward_screen_recorded, true)
                    {
                        self.store
                            .record_relic_opening(now, relic, reward, rewards.len())?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn view(&self) -> Option<View<'_>> {
        Some(View {
            inventory: self.inventory.as_ref()?,
            catalog: &self.catalog,
            prices: self.prices.as_ref(),
            favourites: &self.favourites,
            listings: &self.listings,
        })
    }

    pub fn inventory_tab(&self) -> Option<InventoryTab> {
        Some(inventory_view::tab(&self.view()?))
    }

    pub fn foundry_tab(
        &self,
        include_founders: Option<bool>,
        now: DateTime<Utc>,
    ) -> Option<FoundryTab> {
        let inventory = self.inventory.as_ref()?;
        Some(foundry::tab(
            inventory,
            &self.catalog,
            self.world.as_ref(),
            include_founders,
            now,
            &self.favourites,
        ))
    }

    pub fn craft_tree(&self, unique_name: &str) -> Option<foundry::CraftDetails> {
        let inventory = self.inventory.as_ref()?;
        foundry::details(inventory, &self.catalog, self.prices.as_ref(), unique_name)
    }

    pub fn mastery_tab(&self, options: MasteryOptions) -> Option<MasteryTab> {
        Some(mastery::tab(&self.view()?, options))
    }

    pub fn resources_tab(&self) -> Option<ResourcesTab> {
        let inventory = self.inventory.as_ref()?;
        Some(resources::tab(inventory, &self.catalog))
    }

    pub fn relic_planner_tab(&self, squad_size: u32, only_owned: bool) -> Option<RelicPlannerTab> {
        let view = self.view()?;
        let missing_parts = relic_planner::missing_parts(view.inventory, view.catalog);
        let wanted: Vec<String> = missing_parts
            .iter()
            .map(|part| part.unique_name.clone())
            .collect();
        Some(RelicPlannerTab {
            squad_size,
            only_owned,
            void_traces: relic_planner::void_traces(view.inventory),
            plans: relic_planner::plan(&view, &wanted, squad_size, only_owned),
            missing_parts,
        })
    }

    pub fn owned_relic_slugs(&self) -> Option<Vec<String>> {
        let inventory = self.inventory.as_ref()?;
        Some(
            inventory
                .relics()
                .filter(|(_, count)| *count > 0)
                .filter_map(|(unique_name, _)| {
                    let (relic, _) = self.catalog.relic_by_unique_name(unique_name)?;
                    relic.market_info.as_ref().map(|info| info.url_name.clone())
                })
                .collect(),
        )
    }

    pub fn relics_for(&self, part_unique_name: &str) -> Option<Vec<RelicSource>> {
        let inventory = self.inventory.as_ref()?;
        Some(relic_planner::relics_for(
            part_unique_name,
            inventory,
            &self.catalog,
        ))
    }

    pub fn recommend(&self, rewards: &[String]) -> Option<RewardScreen> {
        Some(relic_planner::recommend(&self.view()?, rewards))
    }

    fn grader(&self) -> Grader<'_> {
        Grader::new(
            &self.catalog,
            &self.riven_attributes,
            self.riven_data.as_ref(),
        )
    }

    pub fn rivens_tab(&self) -> Option<RivensTab> {
        let inventory = self.inventory.as_ref()?;
        Some(self.grader().tab(inventory, &self.listings))
    }

    pub fn riven_cycled(&self, row: &RivenRow, current: &str, pending: &str) -> Option<RivenRow> {
        self.grader().cycled(row, current, pending)
    }

    pub fn riven_at_station(
        &self,
        item_id: &str,
        shown: &str,
        offered: Option<&str>,
    ) -> Option<RivenRow> {
        self.grader()
            .at_station(item_id, shown, offered, self.inventory())
    }

    pub fn riven_comparables(
        &self,
        stats: &[ComparedStat],
        auctions: &WeaponAuctions,
    ) -> RivenComparables {
        comparables::comparables(stats, auctions, &self.riven_attributes)
    }

    pub fn riven_in_dialog(&self, mod_type: &str, fingerprint: &str) -> Option<RivenRow> {
        self.grader()
            .in_dialog(mod_type, fingerprint, self.inventory())
    }

    pub fn apply_reroll(&mut self, item_id: &str, current: &str, pending: &str) -> bool {
        if !self.set_riven_fingerprint(item_id, current) {
            return false;
        }
        self.pending_rerolls
            .insert(item_id.to_owned(), pending.to_owned());
        true
    }

    pub fn discard_pending_roll(&mut self, item_id: &str) -> bool {
        self.pending_rerolls.remove(item_id).is_some()
    }

    pub fn keep_pending_roll(&mut self, item_id: &str) -> bool {
        let Some(pending) = self.pending_rerolls.remove(item_id) else {
            return false;
        };
        self.set_riven_fingerprint(item_id, &pending)
    }

    fn set_riven_fingerprint(&mut self, item_id: &str, fingerprint: &str) -> bool {
        let Some(upgrade) = self.inventory.as_mut().and_then(|inventory| {
            inventory
                .upgrades
                .iter_mut()
                .find(|upgrade| upgrade.item_id.as_str() == item_id)
        }) else {
            return false;
        };
        upgrade.upgrade_fingerprint = Some(fingerprint.to_owned());
        true
    }

    pub fn stats_tab(&self, range: TimeRange, now: DateTime<Utc>) -> Result<StatsTab> {
        let latest_deltas = match self.latest_snapshot {
            Some(id) => self.store.deltas(id)?,
            None => Vec::new(),
        };
        let series = stats::daily_series(&self.store.stats_series(range)?);
        let trades = self.store.trades(range)?;
        let relic_openings = self.store.relic_openings(range)?;
        let earliest = [
            series.first().map(|point| point.at),
            trades.first().map(|trade| trade.at),
            relic_openings.first().map(|opening| opening.at),
        ]
        .into_iter()
        .flatten()
        .min();
        let span = stats::day_span(range, earliest, now);
        let summary = self
            .inventory
            .as_ref()
            .map(|inventory| stats::summary(inventory, &self.catalog, &series));
        Ok(StatsTab {
            relics_per_day: stats::daily_counts(
                relic_openings.iter().map(|opening| opening.at),
                span,
            ),
            trades_per_day: stats::daily_counts(trades.iter().map(|trade| trade.at), span),
            days_played: stats::daily_counts(series.iter().map(|point| point.at), span),
            series,
            trades,
            relic_openings,
            latest_deltas,
            summary,
        })
    }

    pub fn export(&self, dir: &Path, now: DateTime<Utc>) -> Result<()> {
        let (Some(inventory), Some(rivens), Some(foundry)) = (
            self.inventory_tab(),
            self.rivens_tab(),
            self.foundry_tab(None, now),
        ) else {
            return Ok(());
        };
        export(
            dir,
            &ExportBundle {
                inventory: &inventory,
                rivens: &rivens,
                foundry: &foundry,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::events::{CyclePhase, FissureFilter, InventorySummary, TimerAlerts};
    use crate::prices::FixedPrices;

    fn at(millis: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(millis).unwrap()
    }

    fn core_over(store: Store) -> Core {
        Core::new(
            store,
            fixtures::catalog(),
            Arc::new(FixedPrices::new([("styanax_prime_blueprint", 100.0)])),
            AlertSettings::default(),
        )
        .unwrap()
    }

    fn core() -> Core {
        core_over(Store::in_memory().unwrap())
    }

    const REROLLED_ITEM: &str = "f29623082864e554d187a3f7";
    const ROLL_75: &str = r#"{"compat":"/Lotus/Weapons/Archon/Melee/DualDaggers/ArchonDualDaggersPlayerWep","lim":294146365,"lvlReq":11,"lvl":8,"rerolls":75,"pol":"AP_DEFENSE","buffs":[{"Tag":"WeaponFreezeDamageMod","Value":708669601},{"Tag":"WeaponToxinDamageMod","Value":526133492},{"Tag":"WeaponFireRateMod","Value":397284473}],"curses":[]}"#;
    const OFFERED_75: &str = r#"{"compat":"/Lotus/Weapons/Archon/Melee/DualDaggers/ArchonDualDaggersPlayerWep","lim":294146365,"lvlReq":11,"lvl":8,"rerolls":75,"pol":"AP_DEFENSE","buffs":[{"Tag":"WeaponFreezeDamageMod","Value":741320000},{"Tag":"WeaponCritDamageMod","Value":903450000},{"Tag":"WeaponFireDamageMod","Value":512780000}],"curses":[{"Tag":"SlideAttackCritChanceMod","Value":268940000}]}"#;

    fn rerolled_row(core: &Core) -> RivenRow {
        core.rivens_tab()
            .expect("rivens tab")
            .unveiled
            .into_iter()
            .find(|row| row.item_id == REROLLED_ITEM)
            .unwrap()
    }

    #[test]
    fn reroll_keep_and_discard() {
        let mut core = core();
        core.ingest_inventory(fixtures::INVENTORY, at(1_000))
            .unwrap();
        assert_eq!(rerolled_row(&core).rerolls, 74);
        assert!(!core.apply_reroll("000000000000000000000000", ROLL_75, OFFERED_75));

        assert!(core.apply_reroll(REROLLED_ITEM, ROLL_75, OFFERED_75));
        let current = rerolled_row(&core);
        assert_eq!(current.rerolls, 75);
        assert_eq!(current.attributes.len(), 3);
        assert_eq!(current.attributes[1].tag, "WeaponToxinDamageMod");

        assert!(core.keep_pending_roll(REROLLED_ITEM));
        let kept = rerolled_row(&core);
        assert_eq!(kept.rerolls, 75);
        assert_eq!(kept.attributes.len(), 4);
        assert_eq!(kept.attributes[1].tag, "WeaponCritDamageMod");
        assert!(!core.keep_pending_roll(REROLLED_ITEM));

        assert!(core.apply_reroll(REROLLED_ITEM, ROLL_75, OFFERED_75));
        assert!(core.discard_pending_roll(REROLLED_ITEM));
        assert!(!core.keep_pending_roll(REROLLED_ITEM));
        assert_eq!(rerolled_row(&core).attributes.len(), 3);
    }

    #[test]
    fn same_inventory_twice() {
        let mut core = core();
        let events = core
            .ingest_inventory(fixtures::INVENTORY, at(1_000_000))
            .unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            CoreEvent::InventoryUpdated(InventorySummary { changes, mr, .. }) => {
                assert_eq!(*changes, 0);
                assert_eq!(*mr, 14);
            }
            other => panic!("unexpected event {other:?}"),
        }

        core.ingest_inventory(fixtures::INVENTORY, at(2_000_000))
            .unwrap();
        let stats = core.stats_tab(TimeRange::all(), Utc::now()).unwrap();
        assert_eq!(stats.series.len(), 1);
        assert!(stats.latest_deltas.is_empty());
    }

    #[test]
    fn load_cached_snapshot() {
        let dir = std::env::temp_dir().join("wf-core-facade-cached-inventory");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("merframe.sqlite");

        let mut writer = core_over(Store::open(&path).unwrap());
        assert!(writer.load_snapshot().unwrap().is_none());
        writer
            .ingest_inventory(fixtures::INVENTORY, at(1_000_000))
            .unwrap();
        drop(writer);
        assert!(dir.join("inventory.json.gz").exists());

        let mut reader = core_over(Store::open(&path).unwrap());
        assert!(reader.inventory().is_none());
        let snapshot = reader.load_snapshot().unwrap().unwrap();
        assert_eq!(snapshot.last_sync_oid, "6a9eeb1f000000000000c001");
        assert_eq!(reader.latest_snapshot, Some(snapshot.id));
        assert!(reader.inventory().is_some());
        assert!(reader.inventory_tab().is_some());
        let stats = reader.stats_tab(TimeRange::all(), Utc::now()).unwrap();
        assert_eq!(stats.series.len(), 1);
        assert!(stats.latest_deltas.is_empty());
        drop(reader);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn changed_inventory_deltas() {
        let mut core = core();
        core.ingest_inventory(fixtures::INVENTORY, at(1_000_000))
            .unwrap();
        let changed = fixtures::INVENTORY
            .replacen("6a9eeb1f000000000000c001", "6a9eeb1f000000000000c002", 1)
            .replacen(
                r#"{"ItemCount":51923112,"ItemType":"/Lotus/Types/Items/MiscItems/Ferrite"}"#,
                r#"{"ItemCount":51923260,"ItemType":"/Lotus/Types/Items/MiscItems/Ferrite"}"#,
                1,
            );
        let events = core.ingest_inventory(&changed, at(90_000_000)).unwrap();
        match &events[0] {
            CoreEvent::InventoryUpdated(InventorySummary { changes, .. }) => {
                assert_eq!(*changes, 1);
            }
            other => panic!("unexpected event {other:?}"),
        }
        let stats = core.stats_tab(TimeRange::all(), Utc::now()).unwrap();
        assert_eq!(stats.series.len(), 2);
        assert_eq!(stats.latest_deltas.len(), 1);
        assert_eq!(stats.latest_deltas[0].delta, 148);
    }

    #[test]
    fn two_snapshots_same_day() {
        let mut core = core();
        core.ingest_inventory(fixtures::INVENTORY, at(1_000_000))
            .unwrap();
        let richer = fixtures::INVENTORY
            .replacen("6a9eeb1f000000000000c001", "6a9eeb1f000000000000c002", 1)
            .replacen(r#""PremiumCredits":5249"#, r#""PremiumCredits":5427"#, 1);
        core.ingest_inventory(&richer, at(80_000_000)).unwrap();
        let stats = core.stats_tab(TimeRange::all(), at(80_000_000)).unwrap();
        assert_eq!(stats.series.len(), 1);
        assert_eq!(stats.series[0].at, at(0));
        assert_eq!(stats.series[0].plat, 5427);
        assert_eq!(stats.days_played.len(), 1);
        assert_eq!(stats.days_played[0].count, 1);
        assert_eq!(stats.summary.unwrap().snapshot_days, 1);
    }

    #[test]
    fn tabs_after_ingest() {
        let mut core = core();
        assert!(core.inventory_tab().is_none());
        core.ingest_inventory(fixtures::INVENTORY, at(1_000_000))
            .unwrap();
        assert!(core.inventory_tab().is_some());
        assert!(core.foundry_tab(None, at(1_000_000)).is_some());
        assert_eq!(
            core.mastery_tab(MasteryOptions::default()).unwrap().rank,
            14
        );
        assert!(core.resources_tab().is_some());
        let planner = core
            .relic_planner_tab(relic_planner::DEFAULT_SQUAD_SIZE, true)
            .unwrap();
        assert_eq!(planner.missing_parts.len(), 8);
        assert_eq!(planner.plans.len(), 1);
        assert!(core.rivens_tab().is_some());
        let summary = core
            .stats_tab(TimeRange::all(), at(1_000_000))
            .unwrap()
            .summary
            .unwrap();
        assert_eq!(summary.snapshot_days, 1);
        assert!(summary.prime_owned > 0);
        assert!(summary.prime_owned <= summary.prime_total);
        assert!(summary.prime_percent > 0.0);
        assert!(
            core.relics_for("/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsComponent")
                .is_some_and(|sources| !sources.is_empty())
        );
    }

    #[test]
    fn favourite_toggle_across_tabs() {
        let mut core = core();
        core.ingest_inventory(fixtures::INVENTORY, at(1_000_000))
            .unwrap();
        let relic = "/Lotus/Types/Game/Projections/T4VoidProjectionStyanaxPrimeABronze";
        assert!(core.favourites.is_empty());

        assert!(core.toggle_favourite(relic).unwrap());
        assert!(core.favourites.contains(relic));
        let starred = core
            .inventory_tab()
            .unwrap()
            .relics
            .into_iter()
            .filter(|row| row.favourite)
            .map(|row| row.unique_name)
            .collect::<Vec<_>>();
        assert_eq!(starred, vec![relic.to_owned()]);
        assert!(
            core.relic_planner_tab(relic_planner::DEFAULT_SQUAD_SIZE, true)
                .unwrap()
                .plans
                .iter()
                .any(|plan| plan.favourite)
        );

        assert!(!core.toggle_favourite(relic).unwrap());
        assert!(core.favourites.is_empty());
        assert!(
            core.inventory_tab()
                .unwrap()
                .relics
                .iter()
                .all(|row| !row.favourite)
        );
    }

    #[test]
    fn chat_tab_notified_once() {
        let mut core = core();
        let now = at(4_000_000);
        let tab = LogEvent::ChatTabAdded {
            channel: "Fexampletenno".to_owned(),
        };
        let first = core.handle_log_event(&tab, now).expect("chat tab");
        assert_eq!(
            first,
            vec![CoreEvent::NewConversation {
                channel: "Fexampletenno".to_owned(),
                player: "exampletenno".to_owned(),
            }]
        );
        let second = core.handle_log_event(&tab, now).expect("chat tab");
        assert!(second.is_empty());
        core.set_alert_settings(AlertSettings::default());
        let third = core.handle_log_event(&tab, now).expect("chat tab");
        assert!(third.is_empty());
        let region = core
            .handle_log_event(
                &LogEvent::ChatTabAdded {
                    channel: "Q_EN_EU".to_owned(),
                },
                now,
            )
            .expect("chat tab");
        assert!(region.is_empty());
    }

    #[test]
    fn requiem_opening_skipped() {
        let mut core = core();
        let now = at(3_000_000);

        core.handle_log_event(
            &LogEvent::RelicEquipDialog {
                relic: "Requiem III".to_owned(),
            },
            now,
        )
        .unwrap();
        core.handle_log_event(
            &LogEvent::OwnRelicReward {
                account_id: "5f0a1b2c3d4e5f6a7b8c9d0e".to_owned(),
                store_item: "/Lotus/StoreItems/Upgrades/Mods/Immortal/RequiemModA".to_owned(),
            },
            now,
        )
        .unwrap();
        let events = core
            .handle_reward_screen(
                now,
                core.reward_screen_generation(),
                vec!["/Lotus/StoreItems/Upgrades/Mods/Immortal/RequiemModA".to_owned()],
            )
            .unwrap();
        assert_eq!(events.len(), 1);

        let stats = core
            .stats_tab(TimeRange::all(), now + chrono::TimeDelta::days(2))
            .unwrap();
        assert!(stats.relic_openings.is_empty());
    }

    #[test]
    fn trade_and_opening_recorded() {
        let mut core = core();
        let now = at(3_000_000);

        core.handle_log_event(
            &LogEvent::RelicEquipDialog {
                relic: "Axi A21".to_owned(),
            },
            now,
        )
        .unwrap();
        core.handle_log_event(
            &LogEvent::OwnRelicReward {
                account_id: "5f0a1b2c3d4e5f6a7b8c9d0e".to_owned(),
                store_item: "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/StyanaxPrimeBlueprint"
                    .to_owned(),
            },
            now,
        )
        .unwrap();
        let events = core
            .handle_reward_screen(
                now,
                core.reward_screen_generation(),
                vec![
                    "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/StyanaxPrimeBlueprint"
                        .to_owned(),
                ],
            )
            .unwrap();
        assert_eq!(events.len(), 1);

        core.handle_log_event(
            &LogEvent::TradeDialogOpened {
                description: "Are you sure you want to accept this trade? You are offering Styanax Prime Blueprint in exchange for 90 Platinum.".to_owned(),
            },
            now,
        )
        .unwrap();
        core.handle_log_event(&LogEvent::TradeSuccessful, now)
            .unwrap();

        let stats = core
            .stats_tab(TimeRange::all(), now + chrono::TimeDelta::days(2))
            .unwrap();
        assert_eq!(stats.trades.len(), 1);
        assert_eq!(stats.trades[0].trade.plat, 90);
        assert_eq!(stats.relic_openings.len(), 1);
        assert_eq!(stats.relic_openings[0].relic, "Axi A21");
        assert_eq!(stats.relic_openings[0].player_count, 1);
        assert_eq!(
            stats
                .trades_per_day
                .iter()
                .map(|entry| entry.count)
                .collect::<Vec<_>>(),
            vec![1, 0, 0]
        );
        assert_eq!(
            stats
                .relics_per_day
                .iter()
                .map(|entry| entry.count)
                .collect::<Vec<_>>(),
            vec![1, 0, 0]
        );
        assert!(stats.summary.is_none());

        assert!(
            core.recommend(&[
                "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/StyanaxPrimeBlueprint".to_owned()
            ])
            .is_none()
        );
    }

    #[test]
    fn world_state_alerts_and_timers() {
        let mut core = core();
        core.set_alert_settings(AlertSettings {
            fissure_notifications_enabled: true,
            fissure_filters: vec![FissureFilter {
                tier: "Lith".to_owned(),
                ..FissureFilter::default()
            }],
            timers: TimerAlerts::from_iter([CyclePhase::EarthDay, CyclePhase::EarthNight]),
            timer_lead_secs: 86_400,
        });
        let world_state = include_str!("../../../fixtures/worldState.json");
        let early = at(1_788_801_669_000);
        let now = at(1_788_807_069_000);
        let first = core
            .ingest_world_state(world_state, early)
            .expect("world state");
        assert!(
            first
                .iter()
                .all(|event| matches!(event, CoreEvent::TimerAlert { .. }))
        );
        let events = core
            .ingest_world_state(world_state, now)
            .expect("world state");
        assert!(events.iter().any(|event| match event {
            CoreEvent::FissureAlert { fissure } => fissure.tier == "Lith",
            _ => false,
        }));
        core.ingest_inventory(fixtures::INVENTORY, now).unwrap();
        let foundry = core.foundry_tab(None, now).unwrap();
        assert!(!foundry.timers.is_empty());
    }

    #[test]
    fn export_needs_inventory() {
        let mut core = core();
        let dir = std::env::temp_dir().join("wf-core-facade-export");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        core.export(&dir, at(0)).unwrap();
        assert!(!dir.exists());

        core.ingest_inventory(fixtures::INVENTORY, at(1_000_000))
            .unwrap();
        core.export(&dir, at(1_000_000)).unwrap();
        assert!(dir.join("parts.json").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
