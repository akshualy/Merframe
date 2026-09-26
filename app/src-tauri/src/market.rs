use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use wf_core::{InventoryTab, MarketListings, MarketStock, ModRow, PriceSource, SetRow};
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

    pub fn by_name(&self, name: &str) -> Option<&Item> {
        self.items.iter().find(|item| {
            item.i18n
                .get("en")
                .is_some_and(|localized| wf_core::same_part(name, &localized.name))
        })
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

#[derive(Default)]
pub struct Holdings<'a> {
    stock: Option<MarketStock<'a>>,
    sets: &'a [SetRow],
    mods: &'a [ModRow],
    arcanes: &'a [ModRow],
}

fn unveiled(name: &str) -> &str {
    name.strip_suffix(" (Veiled)").unwrap_or(name)
}

fn owned_upgrades(rows: &[ModRow], item: &Item, rank: Option<u32>) -> i64 {
    let listed = english_name(item);
    rows.iter()
        .filter(|row| {
            unveiled(&row.name).eq_ignore_ascii_case(unveiled(&listed))
                || row.unique_name == item.game_ref
        })
        .filter(|row| rank.is_none_or(|rank| row.rank.unwrap_or(0) == rank))
        .map(|row| row.count)
        .sum()
}

impl<'a> Holdings<'a> {
    pub fn of(stock: Option<MarketStock<'a>>, tab: &'a InventoryTab) -> Self {
        Self {
            stock,
            sets: &tab.sets,
            mods: &tab.mods,
            arcanes: &tab.arcanes,
        }
    }

    fn count(
        &self,
        category: MarketCategory,
        item: &Item,
        order: &Order,
        take_rank_into_account: bool,
    ) -> i64 {
        let rank = order.rank.unwrap_or(0);
        match category {
            MarketCategory::Parts => self.stock.map_or(0, |stock| stock.part(item)),
            MarketCategory::Misc => self
                .stock
                .map_or(0, |stock| stock.misc(item, &english_name(item))),
            MarketCategory::Sets => self
                .sets
                .iter()
                .filter(|row| row.market_slug == item.slug)
                .map(|row| row.count)
                .sum(),
            MarketCategory::Mods => {
                owned_upgrades(self.mods, item, take_rank_into_account.then_some(rank))
            }
            MarketCategory::Arcanes => owned_upgrades(self.arcanes, item, Some(rank)),
            MarketCategory::Relics => self.stock.map_or(0, |stock| {
                stock.relic(item, order.subtype.as_deref().unwrap_or_default())
            }),
        }
    }
}

fn is_necramech_set(category: MarketCategory, name: &str) -> bool {
    category == MarketCategory::Sets
        && ["Bonewidow", "Voidrig", "Damaged Necramech"]
            .iter()
            .any(|necramech| name.contains(necramech))
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
    let name = english_name(item);
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
    let show_warning = order.order_type == OrderType::Sell
        && owned < i64::from(order.quantity)
        && !is_necramech_set(category, &name);
    OrderRow {
        id: order.id.clone(),
        order_type: order.order_type,
        item_id: order.item_id.clone(),
        slug: item.slug.clone(),
        name,
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
        show_warning,
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
    let take_rank_into_account = read(&state.settings).market.take_rank_into_account;
    let core = lock(&state.core);
    let tab = core.inventory_tab();
    let holdings = tab
        .as_ref()
        .map(|tab| Holdings::of(core.market_stock(), tab))
        .unwrap_or_default();
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
    use wf_core::{InventoryTab, ItemStatus, ModRow, PriceSource, Prices, SetRow, UpgradePrices};
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

    fn upgrade(name: &str, unique_name: &str, rank: Option<u32>, count: i64) -> ModRow {
        ModRow {
            name: name.to_owned(),
            unique_name: unique_name.to_owned(),
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
            market_slug: String::new(),
            favourite: false,
            order_placed: false,
        }
    }

    const CLASHING_FOREST: &str = "/Lotus/Weapons/Tenno/Melee/MeleeTrees/StaffCmbOneMeleeTree";
    const VEILED_RIFLE: &str = "/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare";

    fn tab() -> InventoryTab {
        InventoryTab {
            parts: Vec::new(),
            mods: vec![
                upgrade(
                    "Primed Continuity",
                    "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarPowerDurationModExpert",
                    None,
                    3,
                ),
                upgrade(
                    "Primed Continuity",
                    "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarPowerDurationModExpert",
                    Some(10),
                    1,
                ),
                upgrade(
                    "Fear Sense",
                    "/Lotus/Types/Friendly/Pets/CatbrowPetPrecepts/CatbrowTremorSensePrecept",
                    None,
                    25,
                ),
                upgrade("Staff Cmb One Melee Tree", CLASHING_FOREST, None, 158),
                upgrade("Staff Cmb One Melee Tree", CLASHING_FOREST, Some(3), 1),
                upgrade("Rifle Riven Mod (Veiled)", VEILED_RIFLE, Some(0), 2),
                upgrade(
                    "Rifle Riven Mod (Veiled)",
                    "/Lotus/Upgrades/Mods/Randomized/RawRifleRandomMod",
                    Some(0),
                    5,
                ),
            ],
            arcanes: vec![
                upgrade(
                    "Arcane Energize",
                    "/Lotus/Upgrades/CosmeticEnhancers/Utility/EnergyOnEnergyPickup",
                    None,
                    4,
                ),
                upgrade(
                    "Arcane Energize",
                    "/Lotus/Upgrades/CosmeticEnhancers/Utility/EnergyOnEnergyPickup",
                    Some(5),
                    1,
                ),
            ],
            relics: Vec::new(),
            misc: Vec::new(),
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
        let tab = tab();
        let holdings = Holdings::of(None, &tab);
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
        assert!(!row.show_warning);

        let row = order_row(
            &order(&set.id, 2, None, None),
            &set,
            &holdings,
            &prices,
            true,
        );
        assert_eq!(row.owned, 1);
        assert!(row.show_warning);
    }

    #[test]
    fn buy_order_not_flagged() {
        let tab = tab();
        let holdings = Holdings::of(None, &tab);
        let prices = TestPrices;
        let set = item("mirage_prime_set", "Mirage Prime Set", &["set"], None);
        let mut wanted = order(&set.id, 9, None, None);
        wanted.order_type = OrderType::Buy;
        let row = order_row(&wanted, &set, &holdings, &prices, true);
        assert_eq!(row.owned, 1);
        assert!(!row.show_warning);
    }

    #[test]
    fn necramech_set_never_flagged() {
        let set = item("voidrig_set", "Voidrig Set", &["set"], None);
        let row = order_row(
            &order(&set.id, 1, None, None),
            &set,
            &Holdings::default(),
            &TestPrices,
            true,
        );
        assert_eq!(row.owned, 0);
        assert!(!row.show_warning);
    }

    #[test]
    fn nothing_owned_without_an_inventory() {
        let holdings = Holdings::default();
        let barrel = item(
            "braton_prime_barrel",
            "Braton Prime Barrel",
            &["component"],
            None,
        );
        let row = order_row(
            &order(&barrel.id, 1, None, None),
            &barrel,
            &holdings,
            &TestPrices,
            true,
        );
        assert_eq!(row.owned, 0);
        assert!(row.show_warning);
    }

    #[test]
    fn mod_rank_counting_setting() {
        let tab = tab();
        let holdings = Holdings::of(None, &tab);
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

    fn owned(item: &Item, rank: Option<u32>) -> i64 {
        order_row(
            &order(&item.id, 1, rank, None),
            item,
            &Holdings::of(None, &tab()),
            &TestPrices,
            true,
        )
        .owned
    }

    #[test]
    fn renamed_mod_counts_by_name() {
        let fear_sense = item("sense_danger", "Fear Sense", &["mod"], Some(5));
        assert_eq!(owned(&fear_sense, Some(0)), 25);
    }

    #[test]
    fn mod_outside_the_export_counts_by_game_ref() {
        let mut stance = item("clashing_forest", "Clashing Forest", &["mod"], Some(3));
        stance.game_ref = CLASHING_FOREST.to_owned();
        assert_eq!(owned(&stance, Some(0)), 158);
        assert_eq!(owned(&stance, Some(3)), 1);
    }

    #[test]
    fn veiled_riven_counts_both_stacks() {
        let mut veiled = item(
            "rifle_riven_mod_(veiled)",
            "Rifle Riven Mod (Veiled)",
            &["mod"],
            None,
        );
        veiled.game_ref = VEILED_RIFLE.to_owned();
        assert_eq!(owned(&veiled, None), 7);
    }

    #[test]
    fn arcane_ranks_separate() {
        let tab = tab();
        let holdings = Holdings::of(None, &tab);
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
    fn set_order_counts_full_sets() {
        let tab = tab();
        let holdings = Holdings::of(None, &tab);
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
        let tab = tab();
        let holdings = Holdings::of(None, &tab);
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
        let tab = tab();
        let holdings = Holdings::of(None, &tab);
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

        let tab = tab();
        let holdings = Holdings::of(None, &tab);
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
