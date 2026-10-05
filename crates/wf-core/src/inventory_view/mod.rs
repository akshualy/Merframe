use std::collections::BTreeSet;
use std::ops::Index;

use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use wf_data::{Rarity, Refinement, catch_grade, misc_item_name};

use crate::catalog::{Catalog, VaultStatus, display_name_from_path, part_name, refinement_name};
use crate::identity::{ItemRecord, unlisted_upgrade};
use crate::prices::Prices;
use crate::view::View;

mod misc;
mod parts;
mod relics;
mod upgrades;

pub(crate) use misc::misc;
pub(crate) use parts::{complete_sets, held_parts, parts, sets};
pub(crate) use relics::relics;
pub(crate) use upgrades::{UpgradeKind, arcanes, mods, upgrade_kind};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ItemSummary {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub market_slug: Option<String>,
    pub prime: bool,
    pub vault: Option<VaultStatus>,
}

impl ItemSummary {
    pub fn new(unique_name: &str, record: &ItemRecord) -> Self {
        Self {
            unique_name: unique_name.to_owned(),
            name: record.name.clone(),
            image_name: record.image_name.clone(),
            market_slug: record.market_slug.clone(),
            prime: record.prime,
            vault: record.vault,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ItemIndex {
    items: IndexSet<ItemSummary>,
    holders: IndexMap<(String, Vec<usize>), ModHolder>,
    favourites: BTreeSet<usize>,
}

impl Index<usize> for ItemIndex {
    type Output = ItemSummary;

    fn index(&self, item: usize) -> &ItemSummary {
        &self.items[item]
    }
}

impl ItemIndex {
    fn add(&mut self, summary: ItemSummary) -> usize {
        self.items.insert_full(summary).0
    }

    fn add_marked(&mut self, summary: ItemSummary, favourite: bool) -> usize {
        let item = self.add(summary);
        if favourite {
            self.favourites.insert(item);
        }
        item
    }

    fn add_holder(
        &mut self,
        key: (String, Vec<usize>),
        holder: impl FnOnce() -> ModHolder,
    ) -> usize {
        let entry = self.holders.entry(key);
        let position = entry.index();
        entry.or_insert_with(holder);
        position
    }

    fn holder(&self, holder: usize) -> &ModHolder {
        &self.holders[holder]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ItemStatus {
    pub built: bool,
    pub mastered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SetLink {
    pub item: usize,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct PartRow {
    pub item: usize,
    pub count: i64,
    pub prices: Prices,
    pub set: SetLink,
    pub status: ItemStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct UpgradePrices {
    pub sell: Option<f64>,
    pub sell_max_rank: Option<f64>,
    pub is_floor: bool,
    pub buy: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ModHolder {
    pub item_id: String,
    pub name: String,
    pub custom_name: Option<String>,
    pub image_name: Option<String>,
    pub rank: Option<u32>,
    pub takes_orokin_reactor: bool,
    pub orokin_upgrade: bool,
    pub exilus_adapter: bool,
    pub configs: Vec<usize>,
    pub forma: u32,
    pub archon_shards: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ModRow {
    pub item: usize,
    pub count: i64,
    pub rank: Option<u32>,
    pub max_rank: Option<u32>,
    pub prices: UpgradePrices,
    pub rarity: Option<Rarity>,
    pub equipped_in: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RelicRow {
    pub item: usize,
    pub tier: String,
    pub refinement: Refinement,
    pub count: i64,
    pub plat: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SculptureStars {
    pub amber_filled: u32,
    pub cyan_filled: u32,
    pub amber_sockets: u32,
    pub cyan_sockets: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct MiscRow {
    pub item: usize,
    pub count: i64,
    pub ducats: Option<u32>,
    pub plat: Option<f64>,
    pub market_subtype: Option<String>,
    pub stars: Option<SculptureStars>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SetComponent {
    pub item: usize,
    pub owned: i64,
    pub required: i64,
    pub enough: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SetRow {
    pub item: usize,
    pub owned_parts: usize,
    pub total_parts: usize,
    pub count: i64,
    pub complete: bool,
    pub status: ItemStatus,
    pub prices: Prices,
    pub components: Vec<SetComponent>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct InventoryTab {
    pub items: Vec<ItemSummary>,
    pub holders: Vec<ModHolder>,
    pub favourites: Vec<usize>,
    pub selling: Vec<usize>,
    pub buying: Vec<usize>,
    pub parts: Vec<PartRow>,
    pub mods: Vec<ModRow>,
    pub arcanes: Vec<ModRow>,
    pub relics: Vec<RelicRow>,
    pub misc: Vec<MiscRow>,
    pub sets: Vec<SetRow>,
}

pub(crate) fn tab(view: &View) -> InventoryTab {
    let mut index = ItemIndex::default();
    let parts = parts(view, &mut index);
    let mods = mods(view, &mut index);
    let arcanes = arcanes(view, &mut index);
    let relics = relics(view, &mut index);
    let misc = misc(view, &mut index);
    let sets = sets(view, &mut index);
    let mut selling = Vec::new();
    let mut buying = Vec::new();
    for (item, summary) in (0..).zip(&index.items) {
        let Some(slug) = summary.market_slug.as_deref() else {
            continue;
        };
        let orders = view.listings.orders_for(slug);
        if orders.sell {
            selling.push(item);
        }
        if orders.buy {
            buying.push(item);
        }
    }
    InventoryTab {
        favourites: index.favourites.into_iter().collect(),
        items: index.items.into_iter().collect(),
        holders: index.holders.into_values().collect(),
        selling,
        buying,
        parts,
        mods,
        arcanes,
        relics,
        misc,
        sets,
    }
}

pub(crate) fn catalogued_name(catalog: &Catalog, unique_name: &str) -> Option<String> {
    if let Some((relic, refinement)) = catalog.relic_by_unique_name(unique_name) {
        return Some(format!(
            "{} Relic ({})",
            relic.name,
            refinement_name(refinement)
        ));
    }
    if let Some((base, grade)) = catch_grade(unique_name)
        && let Some(item) = catalog.item(&base)
    {
        return Some(format!("{} ({grade})", item.name));
    }
    if let Some(item) = catalog.item(unique_name) {
        return Some(item.name.clone());
    }
    if let Some((item, component)) = catalog.component(unique_name) {
        return Some(part_name(item, component));
    }
    if let Some(upgrade) = unlisted_upgrade(unique_name) {
        return Some(upgrade.name.to_owned());
    }
    misc_item_name(unique_name).map(str::to_owned)
}

pub(crate) fn display_name(catalog: &Catalog, unique_name: &str) -> String {
    let stocked = unique_name
        .strip_prefix("/Lotus/StoreItems/")
        .map(|tail| format!("/Lotus/{tail}"));
    let unique_name = stocked.as_deref().unwrap_or(unique_name);
    catalogued_name(catalog, unique_name).unwrap_or_else(|| display_name_from_path(unique_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{fixtures, names_a_prime};
    use crate::identity::ItemTable;
    use crate::listings::MarketListings;
    use crate::prices::FixedPrices;
    use crate::view::Fixture;

    pub(super) fn prices() -> FixedPrices {
        FixedPrices::new([
            ("trinity_prime_systems_blueprint", 14.0),
            ("braton_prime_barrel", 8.0),
            ("braton_prime_set", 45.0),
            ("axi_a21_relic", 5.0),
        ])
    }

    impl ItemIndex {
        pub(super) fn favourite(&self, item: usize) -> bool {
            self.favourites.contains(&item)
        }
    }

    #[test]
    fn vault_pill() {
        let stocked = Fixture::new(
            fixtures::catalog(),
            fixtures::inventory_owning(&[(
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint",
                1,
            )]),
        )
        .with_prices(prices());
        let (rows, index) = stocked.rows(parts);
        assert!(index[rows[0].item].vault.is_some());

        let (mod_rows, _) = Fixture::new(fixtures::catalog(), fixtures::inventory())
            .with_prices(prices())
            .rows(mods);
        assert!(!mod_rows.is_empty());

        let (sets, index) = stocked.rows(sets);
        assert!(sets.iter().all(|row| {
            let set = &index[row.item];
            set.vault.is_some() == names_a_prime(&set.name)
        }));
    }

    #[test]
    fn index_shares_equal_summaries() {
        let catalog = fixtures::catalog();
        let items = ItemTable::build(&catalog);
        let barrel = items
            .get("/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel")
            .unwrap();
        let mut index = ItemIndex::default();
        let first = index.add(ItemSummary::new(&barrel.unique_name, barrel));
        let again = index.add(ItemSummary::new(&barrel.unique_name, barrel));
        let renamed = index.add(ItemSummary {
            name: "Braton Prime Barrel Listing".to_owned(),
            ..ItemSummary::new(&barrel.unique_name, barrel)
        });
        assert_eq!(first, again);
        assert_ne!(first, renamed);
        assert_eq!(index[renamed].name, "Braton Prime Barrel Listing");
        assert_eq!(index.items.len(), 2);
    }

    #[test]
    fn one_entry_per_item() {
        let fixture =
            Fixture::new(fixtures::catalog(), fixtures::inventory()).with_prices(prices());
        let tab = tab(&fixture.view());
        assert!(!tab.misc.is_empty());
        let distinct: std::collections::HashSet<&ItemSummary> = tab.items.iter().collect();
        assert_eq!(distinct.len(), tab.items.len());
        let referenced: BTreeSet<usize> = tab
            .parts
            .iter()
            .flat_map(|row| [row.item, row.set.item])
            .chain(tab.mods.iter().map(|row| row.item))
            .chain(tab.arcanes.iter().map(|row| row.item))
            .chain(tab.relics.iter().map(|row| row.item))
            .chain(tab.misc.iter().map(|row| row.item))
            .chain(tab.sets.iter().flat_map(|row| {
                std::iter::once(row.item).chain(row.components.iter().map(|part| part.item))
            }))
            .collect();
        assert_eq!(referenced, (0..).take(tab.items.len()).collect());
        let held: BTreeSet<usize> = tab
            .mods
            .iter()
            .chain(&tab.arcanes)
            .flat_map(|row| row.equipped_in.iter().copied())
            .collect();
        assert_eq!(held, (0..).take(tab.holders.len()).collect());
        let distinct: std::collections::HashSet<&ModHolder> = tab.holders.iter().collect();
        assert_eq!(distinct.len(), tab.holders.len());
        assert!(
            tab.relics
                .iter()
                .any(|row| row.count > 0 && row.plat.is_some_and(|plat| plat >= 5.0))
        );
        assert!(tab.favourites.is_empty());
    }

    #[test]
    fn orders_mark_items() {
        let fixture = Fixture {
            listings: MarketListings::new(
                [
                    ("braton_prime_barrel", wf_market::OrderType::Sell),
                    ("braton_prime_set", wf_market::OrderType::Buy),
                ],
                &[],
            ),
            ..Fixture::new(fixtures::catalog(), favourite_fixture()).with_prices(prices())
        };
        let tab = tab(&fixture.view());
        let slugs = |items: &[usize]| -> Vec<Option<&str>> {
            items
                .iter()
                .map(|&item| tab.items[item].market_slug.as_deref())
                .collect()
        };
        assert_eq!(slugs(&tab.selling), [Some("braton_prime_barrel")]);
        assert_eq!(slugs(&tab.buying), [Some("braton_prime_set")]);
    }

    fn favourite_fixture() -> wf_inventory::Inventory {
        fixtures::inventory_owning(&[
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                3,
            ),
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock",
                1,
            ),
            (
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint",
                1,
            ),
        ])
    }

    #[test]
    fn favourites_across_tab() {
        let mut fixture =
            Fixture::new(fixtures::catalog(), favourite_fixture()).with_prices(prices());
        let plain = tab(&fixture.view());
        assert!(!plain.mods.is_empty());
        assert!(!plain.arcanes.is_empty());
        assert!(plain.favourites.is_empty());

        let name = |item: usize| plain.items[item].unique_name.clone();
        fixture.favourites = [
            name(plain.parts[0].item),
            name(plain.mods[0].item),
            name(plain.arcanes[0].item),
            name(plain.relics[0].item),
            name(plain.misc[0].item),
        ]
        .into_iter()
        .collect();
        let marked = tab(&fixture.view());
        let favourite = |item: usize| marked.favourites.contains(&item);
        assert!(favourite(marked.parts[0].item));
        assert!(favourite(marked.mods[0].item));
        assert!(favourite(marked.arcanes[0].item));
        assert!(favourite(marked.relics[0].item));
        assert!(favourite(marked.misc[0].item));
        assert_eq!(
            marked
                .relics
                .iter()
                .filter(|row| favourite(row.item))
                .count(),
            1,
            "a favourited relic stack leaves the other refinements alone"
        );
        assert_eq!(
            marked.misc.iter().filter(|row| favourite(row.item)).count(),
            1,
            "a favourited misc item marks one row"
        );
    }

    #[test]
    fn favourite_set_marks_parts() {
        let mut fixture =
            Fixture::new(fixtures::catalog(), favourite_fixture()).with_prices(prices());
        let (rows, index) = fixture.rows(sets);
        let set = rows
            .into_iter()
            .find(|row| index[row.item].name == "Braton Prime")
            .expect("Braton Prime");
        assert!(!index.favourite(set.item));

        fixture.favourites = [index[set.item].unique_name.clone()].into_iter().collect();
        let (rows, index) = fixture.rows(sets);
        let marked = rows
            .into_iter()
            .find(|row| index[row.item].name == "Braton Prime")
            .expect("Braton Prime");
        assert!(index.favourite(marked.item));

        let (rows, index) = fixture.rows(parts);
        let in_braton = |row: &PartRow| index[row.set.item].name == "Braton Prime";
        let owned_parts: Vec<&PartRow> = rows.iter().filter(|row| in_braton(row)).collect();
        assert!(!owned_parts.is_empty());
        assert!(owned_parts.iter().all(|row| index.favourite(row.item)));
        assert!(
            rows.iter()
                .any(|row| !in_braton(row) && !index.favourite(row.item))
        );
    }
}
