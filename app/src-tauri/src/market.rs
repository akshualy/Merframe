use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use wf_core::{InventoryTab, MarketListings, PriceSource};
use wf_market::{Auction, Item, Order, OrderType, UserStatus};

use crate::state::{AppState, lock, read, write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketCategory {
    Parts,
    Relics,
    Mods,
    Arcanes,
    Sets,
    Misc,
}

impl MarketCategory {
    fn of(item: &Item) -> Self {
        let tagged = |tag: &str| item.tags.iter().any(|owned| owned == tag);
        if tagged("component") || tagged("blueprint") || item.slug.contains("kavasa") {
            return Self::Parts;
        }
        if tagged("relic") {
            return Self::Relics;
        }
        if tagged("mod") {
            return Self::Mods;
        }
        if tagged("arcane") || tagged("arcane_enhancement") {
            return Self::Arcanes;
        }
        if tagged("set") {
            return Self::Sets;
        }
        Self::Misc
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Presence {
    pub status: Option<UserStatus>,
    pub auto: bool,
}

impl Presence {
    pub fn wanted(self, game_detected: bool) -> Option<UserStatus> {
        if self.auto {
            return Some(if game_detected {
                UserStatus::Ingame
            } else {
                UserStatus::Invisible
            });
        }
        self.status
    }
}

pub struct ItemTable {
    items: Vec<Item>,
    by_id: HashMap<String, usize>,
    fetched_at: DateTime<Utc>,
}

impl ItemTable {
    fn new(items: Vec<Item>, fetched_at: DateTime<Utc>) -> Self {
        let by_id = items
            .iter()
            .enumerate()
            .map(|(index, item)| (item.id.clone(), index))
            .collect();
        Self {
            items,
            by_id,
            fetched_at,
        }
    }

    fn stale(&self, now: DateTime<Utc>) -> bool {
        now.signed_duration_since(self.fetched_at) > Duration::hours(12)
    }

    fn get(&self, id: &str) -> Option<&Item> {
        self.by_id.get(id).map(|index| &self.items[*index])
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }
}

pub async fn item_table(state: &Arc<AppState>) -> Option<Arc<ItemTable>> {
    let mut cache = state.market_items.lock().await;
    let cached = cache.clone();
    let now = Utc::now();
    if let Some(table) = &cached
        && !table.stale(now)
    {
        return cached;
    }
    match state.market().items().await {
        Ok(items) => {
            let table = Arc::new(ItemTable::new(items, now));
            *cache = Some(Arc::clone(&table));
            Some(table)
        }
        Err(error) => {
            tracing::warn!(
                error = %error.brief(),
                stale = cached.is_some(),
                "Item list request to warframe.market failed",
            );
            cached
        }
    }
}

pub fn english_name(item: &Item) -> String {
    match item.i18n.get("en") {
        Some(entry) => entry.name.clone(),
        None => item.slug.clone(),
    }
}

pub fn english_thumb(item: &Item) -> String {
    item.i18n
        .get("en")
        .map(|entry| entry.thumb.clone())
        .unwrap_or_default()
}

#[derive(Debug, Default)]
pub struct Holdings {
    parts: HashMap<String, i64>,
    misc: HashMap<String, i64>,
    sets: HashMap<String, i64>,
    ranked_mods: HashMap<(String, u32), i64>,
    mods: HashMap<String, i64>,
    ranked_arcanes: HashMap<(String, u32), i64>,
    relics: HashMap<(String, String), i64>,
}

fn add<K: std::hash::Hash + Eq>(counts: &mut HashMap<K, i64>, key: K, count: i64) {
    *counts.entry(key).or_insert(0) += count;
}

impl Holdings {
    pub fn of(tab: &InventoryTab) -> Self {
        let mut holdings = Self::default();
        for row in &tab.parts {
            add(&mut holdings.parts, row.market_slug.clone(), row.count);
        }
        for row in &tab.misc {
            add(&mut holdings.misc, row.market_slug.clone(), row.count);
        }
        for row in &tab.sets {
            add(&mut holdings.sets, row.market_slug.clone(), row.count);
        }
        for row in &tab.mods {
            add(&mut holdings.mods, row.market_slug.clone(), row.count);
            add(
                &mut holdings.ranked_mods,
                (row.market_slug.clone(), row.rank.unwrap_or(0)),
                row.count,
            );
        }
        for row in &tab.arcanes {
            add(
                &mut holdings.ranked_arcanes,
                (row.market_slug.clone(), row.rank.unwrap_or(0)),
                row.count,
            );
        }
        for row in &tab.relics {
            add(
                &mut holdings.relics,
                (row.relic.clone(), row.refinement.to_lowercase()),
                row.count,
            );
        }
        holdings
    }

    fn count(
        &self,
        category: MarketCategory,
        item: &Item,
        order: &Order,
        take_rank_into_account: bool,
    ) -> i64 {
        let rank = order.rank.unwrap_or(0);
        let slug = item.slug.as_str();
        match category {
            MarketCategory::Parts => self.parts.get(slug).copied().unwrap_or(0),
            MarketCategory::Misc => self.misc.get(slug).copied().unwrap_or(0),
            MarketCategory::Sets => self.sets.get(slug).copied().unwrap_or(0),
            MarketCategory::Mods => {
                if take_rank_into_account {
                    self.ranked_mods
                        .get(&(slug.to_owned(), rank))
                        .copied()
                        .unwrap_or(0)
                } else {
                    self.mods.get(slug).copied().unwrap_or(0)
                }
            }
            MarketCategory::Arcanes => self
                .ranked_arcanes
                .get(&(slug.to_owned(), rank))
                .copied()
                .unwrap_or(0),
            MarketCategory::Relics => {
                let relic = english_name(item);
                let relic = relic.strip_suffix(" Relic").unwrap_or(&relic);
                let refinement = order.subtype.as_deref().unwrap_or_default().to_lowercase();
                self.relics
                    .get(&(relic.to_owned(), refinement))
                    .copied()
                    .unwrap_or(0)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderRow {
    pub id: String,
    pub order_type: OrderType,
    pub item_id: String,
    pub slug: String,
    pub name: String,
    pub thumb: String,
    pub category: MarketCategory,
    pub platinum: u32,
    pub quantity: u32,
    pub per_trade: Option<u32>,
    pub rank: Option<u32>,
    pub subtype: Option<String>,
    pub visible: bool,
    pub updated_at: DateTime<Utc>,
    pub owned: i64,
    pub show_warning: bool,
    pub lowest: Option<f64>,
    pub lowest_from_rank_zero: bool,
}

pub fn order_row(
    order: &Order,
    item: &Item,
    holdings: &Holdings,
    prices: &dyn PriceSource,
    take_rank_into_account: bool,
) -> OrderRow {
    let category = MarketCategory::of(item);
    let owned = holdings.count(category, item, order, take_rank_into_account);
    let ranked = matches!(category, MarketCategory::Mods | MarketCategory::Arcanes);
    let rank = order.rank.unwrap_or(0);
    let mut lowest = prices.plat(&item.slug);
    let mut from_rank_zero = ranked && rank > 0;
    if from_rank_zero
        && item.max_rank == Some(rank)
        && let Some(at_rank) = prices.plat_max_rank(&item.slug)
    {
        lowest = Some(at_rank);
        from_rank_zero = false;
    }
    if lowest.is_none() {
        from_rank_zero = false;
    }
    OrderRow {
        id: order.id.clone(),
        order_type: order.order_type,
        item_id: order.item_id.clone(),
        slug: item.slug.clone(),
        name: english_name(item),
        thumb: english_thumb(item),
        category,
        platinum: order.platinum,
        quantity: order.quantity,
        per_trade: order.per_trade,
        rank: order.rank,
        subtype: order.subtype.clone(),
        visible: order.visible,
        updated_at: order.updated_at,
        owned,
        show_warning: order.order_type == OrderType::Sell && owned < i64::from(order.quantity),
        lowest,
        lowest_from_rank_zero: from_rank_zero,
    }
}

#[derive(Debug, Default, Clone)]
pub struct Held {
    pub orders: Vec<OrderRow>,
    pub auctions: Vec<Auction>,
}

#[derive(Default)]
pub struct Listings {
    held: RwLock<Held>,
}

impl Listings {
    pub fn held(&self) -> Held {
        read(&self.held).clone()
    }

    fn remember(&self, orders: Option<Vec<OrderRow>>, auctions: Option<Vec<Auction>>) -> Held {
        let mut held = write(&self.held);
        if let Some(orders) = orders {
            held.orders = orders;
        }
        if let Some(auctions) = auctions {
            held.auctions = auctions;
        }
        held.clone()
    }
}

pub fn remember_listings(
    state: &AppState,
    orders: Option<Vec<OrderRow>>,
    auctions: Option<Vec<Auction>>,
) {
    let held = state.listings.remember(orders, auctions);
    lock(&state.core).set_market_listings(MarketListings::new(
        held.orders.iter().map(|row| row.slug.as_str()),
        &held.auctions,
    ));
}

pub async fn order_rows(state: &Arc<AppState>, orders: &[Order]) -> Option<Vec<OrderRow>> {
    let table = item_table(state).await?;
    let holdings = lock(&state.core)
        .inventory_tab()
        .map(|tab| Holdings::of(&tab))
        .unwrap_or_default();
    let take_rank_into_account = read(&state.settings).market.take_rank_into_account;
    Some(
        orders
            .iter()
            .filter_map(|order| {
                let item = table.get(&order.item_id)?;
                Some(order_row(
                    order,
                    item,
                    &holdings,
                    state.prices.as_ref(),
                    take_rank_into_account,
                ))
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use chrono::{DateTime, Utc};
    use wf_core::{
        InventoryTab, ItemStatus, MiscRow, ModRow, PartRow, PartSet, PriceSource, Prices, RelicRow,
        SetRow, UpgradePrices, VaultStatus,
    };
    use wf_market::{Item, ItemLocalization, Order, OrderType};

    use super::*;

    struct TestPrices;

    impl PriceSource for TestPrices {
        fn plat(&self, market_slug: &str) -> Option<f64> {
            (market_slug == "primed_continuity").then_some(120.0)
        }

        fn plat_max_rank(&self, market_slug: &str) -> Option<f64> {
            (market_slug == "primed_continuity").then_some(320.0)
        }
    }

    fn item(slug: &str, name: &str, tags: &[&str], max_rank: Option<u32>) -> Item {
        Item {
            id: format!("id-{slug}"),
            slug: slug.to_owned(),
            game_ref: String::new(),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            max_rank,
            subtypes: None,
            max_amber_stars: None,
            max_cyan_stars: None,
            tradable: Some(true),
            bulk_tradable: None,
            vaulted: None,
            ducats: None,
            rarity: None,
            i18n: HashMap::from([(
                "en".to_owned(),
                ItemLocalization {
                    name: name.to_owned(),
                    description: None,
                    icon: String::new(),
                    thumb: format!("thumbs/{slug}.png"),
                },
            )]),
        }
    }

    fn order(item_id: &str, quantity: u32, rank: Option<u32>, subtype: Option<&str>) -> Order {
        Order {
            id: format!("order-{item_id}"),
            order_type: OrderType::Sell,
            platinum: 20,
            quantity,
            per_trade: None,
            subtype: subtype.map(str::to_owned),
            rank,
            amber_stars: None,
            cyan_stars: None,
            visible: true,
            updated_at: moment(),
            item_id: item_id.to_owned(),
            user: None,
        }
    }

    fn moment() -> DateTime<Utc> {
        DateTime::from_timestamp(1_757_410_800, 0).unwrap()
    }

    #[test]
    fn item_table_lookup() {
        let first = item("braton_prime_set", "Braton Prime Set", &["set"], None);
        let second = item("primed_continuity", "Primed Continuity", &["mod"], Some(10));
        let table = ItemTable::new(vec![first.clone(), second.clone()], moment());

        assert_eq!(
            table
                .items()
                .iter()
                .map(|item| item.slug.as_str())
                .collect::<Vec<_>>(),
            ["braton_prime_set", "primed_continuity"]
        );
        assert_eq!(
            table
                .get("id-primed_continuity")
                .map(|item| item.slug.as_str()),
            Some("primed_continuity")
        );
    }

    fn part(slug: &str, count: i64) -> PartRow {
        PartRow {
            name: slug.to_owned(),
            unique_name: slug.to_owned(),
            image_name: None,
            count,
            prices: Prices::default(),
            set: PartSet {
                name: String::new(),
                complete: false,
            },
            vault: None,
            item: ItemStatus {
                built: false,
                mastered: false,
            },
            prime: true,
            market_slug: slug.to_owned(),
            favourite: false,
            order_placed: false,
        }
    }

    fn upgrade(slug: &str, rank: Option<u32>, count: i64) -> ModRow {
        ModRow {
            name: slug.to_owned(),
            unique_name: slug.to_owned(),
            image_name: None,
            market_thumb: None,
            count,
            rank,
            max_rank: None,
            prices: UpgradePrices {
                sell: None,
                sell_max_rank: None,
                is_floor: false,
                buy: None,
            },
            equipped_in: Vec::new(),
            rarity: None,
            prime: false,
            market_slug: slug.to_owned(),
            favourite: false,
            order_placed: false,
        }
    }

    fn relic(name: &str, refinement: &'static str, count: i64) -> RelicRow {
        RelicRow {
            relic: name.to_owned(),
            tier: "Lith".to_owned(),
            refinement,
            image_name: None,
            count,
            vault: VaultStatus::Unknown,
            unique_name: name.to_owned(),
            plat: None,
            favourite: false,
            order_placed: false,
        }
    }

    fn tab() -> InventoryTab {
        InventoryTab {
            parts: vec![part("braton_prime_barrel", 2)],
            mods: vec![
                upgrade("primed_continuity", None, 3),
                upgrade("primed_continuity", Some(10), 1),
            ],
            arcanes: vec![
                upgrade("arcane_energize", None, 4),
                upgrade("arcane_energize", Some(5), 1),
            ],
            relics: vec![
                relic("Lith A1", "Intact", 7),
                relic("Lith A1", "Radiant", 2),
            ],
            misc: vec![MiscRow {
                name: "forma".to_owned(),
                unique_name: "forma".to_owned(),
                image_name: None,
                count: 5,
                ducats: None,
                plat: None,
                market_slug: "forma".to_owned(),
                favourite: false,
                order_placed: false,
            }],
            sets: vec![SetRow {
                set_name: "Mirage Prime Set".to_owned(),
                unique_name: "mirage_prime".to_owned(),
                image_name: None,
                owned_parts: 4,
                total_parts: 4,
                count: 1,
                complete: true,
                item: ItemStatus {
                    built: false,
                    mastered: true,
                },
                vault: None,
                prices: Prices::default(),
                market_slug: "mirage_prime_set".to_owned(),
                favourite: false,
                order_placed: false,
                components: Vec::new(),
            }],
            totals: BTreeMap::new(),
        }
    }

    #[test]
    fn presence_follows_game() {
        use wf_market::UserStatus;

        use super::*;

        let untouched = Presence::default();
        assert_eq!(untouched.wanted(true), None);

        let manual = Presence {
            status: Some(UserStatus::Online),
            auto: false,
        };
        assert_eq!(manual.wanted(true), Some(UserStatus::Online));
        assert_eq!(manual.wanted(false), Some(UserStatus::Online));

        let automatic = Presence {
            status: Some(UserStatus::Online),
            auto: true,
        };
        assert_eq!(automatic.wanted(true), Some(UserStatus::Ingame));
        assert_eq!(automatic.wanted(false), Some(UserStatus::Invisible));
    }

    #[test]
    fn category_from_tags() {
        assert_eq!(
            MarketCategory::of(&item(
                "braton_prime_barrel",
                "Braton Prime Barrel",
                &["component", "prime"],
                None
            )),
            MarketCategory::Parts
        );
        assert_eq!(
            MarketCategory::of(&item("lith_a1_relic", "Lith A1 Relic", &["relic"], None)),
            MarketCategory::Relics
        );
        assert_eq!(
            MarketCategory::of(&item(
                "primed_continuity",
                "Primed Continuity",
                &["mod", "rare"],
                Some(10)
            )),
            MarketCategory::Mods
        );
        assert_eq!(
            MarketCategory::of(&item(
                "arcane_energize",
                "Arcane Energize",
                &["arcane_enhancement"],
                Some(5)
            )),
            MarketCategory::Arcanes
        );
        assert_eq!(
            MarketCategory::of(&item(
                "mirage_prime_set",
                "Mirage Prime Set",
                &["set", "prime"],
                None
            )),
            MarketCategory::Sets
        );
        assert_eq!(
            MarketCategory::of(&item("forma", "Forma", &["misc"], None)),
            MarketCategory::Misc
        );
        assert_eq!(
            MarketCategory::of(&item(
                "kavasa_prime_collar_blueprint",
                "Kavasa Prime Kubrow Collar Blueprint",
                &["prime"],
                None
            )),
            MarketCategory::Parts
        );
    }

    #[test]
    fn oversold_sell_order_flagged() {
        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let part = item(
            "braton_prime_barrel",
            "Braton Prime Barrel",
            &["component"],
            None,
        );
        let row = order_row(
            &order(&part.id, 2, None, None),
            &part,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(row.owned, 2);
        assert!(!row.show_warning);

        let row = order_row(
            &order(&part.id, 3, None, None),
            &part,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(row.owned, 2);
        assert!(row.show_warning);
    }

    #[test]
    fn buy_order_not_flagged() {
        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let part = item(
            "braton_prime_barrel",
            "Braton Prime Barrel",
            &["component"],
            None,
        );
        let mut wanted = order(&part.id, 9, None, None);
        wanted.order_type = OrderType::Buy;
        let row = order_row(&wanted, &part, &holdings, &prices, true);
        assert_eq!(row.owned, 2);
        assert!(!row.show_warning);
    }

    #[test]
    fn mod_rank_counting_setting() {
        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let mod_item = item("primed_continuity", "Primed Continuity", &["mod"], Some(10));

        let ranked = order_row(
            &order(&mod_item.id, 1, Some(10), None),
            &mod_item,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(ranked.owned, 1);
        let unranked = order_row(
            &order(&mod_item.id, 1, Some(0), None),
            &mod_item,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(unranked.owned, 3);

        let pooled = order_row(
            &order(&mod_item.id, 1, Some(10), None),
            &mod_item,
            &holdings,
            &prices,
            false,
        );
        assert_eq!(pooled.owned, 4);
    }

    #[test]
    fn arcane_ranks_separate() {
        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let arcane = item(
            "arcane_energize",
            "Arcane Energize",
            &["arcane_enhancement"],
            Some(5),
        );
        let maxed = order_row(
            &order(&arcane.id, 1, Some(5), None),
            &arcane,
            &holdings,
            &prices,
            false,
        );
        assert_eq!(maxed.owned, 1);
        let unranked = order_row(
            &order(&arcane.id, 1, None, None),
            &arcane,
            &holdings,
            &prices,
            false,
        );
        assert_eq!(unranked.owned, 4);
    }

    #[test]
    fn relic_order_by_refinement() {
        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let relic_item = item("lith_a1_relic", "Lith A1 Relic", &["relic"], None);
        let intact = order_row(
            &order(&relic_item.id, 1, None, Some("intact")),
            &relic_item,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(intact.owned, 7);
        let radiant = order_row(
            &order(&relic_item.id, 3, None, Some("radiant")),
            &relic_item,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(radiant.owned, 2);
        assert!(radiant.show_warning);
    }

    #[test]
    fn set_order_counts_full_sets() {
        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let set = item("mirage_prime_set", "Mirage Prime Set", &["set"], None);
        let row = order_row(
            &order(&set.id, 1, None, None),
            &set,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(row.owned, 1);
        assert_eq!(row.category, MarketCategory::Sets);
    }

    #[test]
    fn lowest_price_at_rank() {
        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let mod_item = item("primed_continuity", "Primed Continuity", &["mod"], Some(10));

        let unranked = order_row(
            &order(&mod_item.id, 1, Some(0), None),
            &mod_item,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(unranked.lowest, Some(120.0));
        assert!(!unranked.lowest_from_rank_zero);

        let maxed = order_row(
            &order(&mod_item.id, 1, Some(10), None),
            &mod_item,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(maxed.lowest, Some(320.0));
        assert!(!maxed.lowest_from_rank_zero);

        let halfway = order_row(
            &order(&mod_item.id, 1, Some(5), None),
            &mod_item,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(halfway.lowest, Some(120.0));
        assert!(halfway.lowest_from_rank_zero);
    }

    #[test]
    fn unpriced_item() {
        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let set = item("mirage_prime_set", "Mirage Prime Set", &["set"], None);
        let row = order_row(
            &order(&set.id, 1, None, None),
            &set,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(row.lowest, None);
        assert!(!row.lowest_from_rank_zero);
    }

    #[test]
    fn listings_keep_other_side() {
        const AUCTIONS: &str =
            include_str!("../../../crates/wf-market/tests/fixtures/auctions_my.json");
        let auctions = wf_market::parse_v1_auctions(AUCTIONS).unwrap();
        let listings = Listings::default();
        assert!(listings.held().orders.is_empty());
        assert!(listings.held().auctions.is_empty());

        let holdings = Holdings::of(&tab());
        let prices = TestPrices;
        let barrel = item(
            "braton_prime_barrel",
            "Braton Prime Barrel",
            &["component"],
            None,
        );
        let rows = vec![order_row(
            &order(&barrel.id, 1, None, None),
            &barrel,
            &holdings,
            &prices,
            true,
        )];
        listings.remember(Some(rows), Some(auctions));
        assert_eq!(listings.held().orders.len(), 1);
        assert_eq!(listings.held().auctions.len(), 2);

        listings.remember(Some(Vec::new()), None);
        let held = listings.held();
        assert!(held.orders.is_empty());
        assert_eq!(
            held.auctions.len(),
            2,
            "an order refresh leaves the riven auctions alone"
        );
    }
}
