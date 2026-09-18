use std::collections::{HashMap, HashSet};

use wf_data::Item;
use wf_inventory::Inventory;

use super::xp::Ledger;
use super::{
    Acquisition, CategoryTotals, Level, MasteryComponent, MasteryGroup, MasteryItem, MasteryOptions,
};
use crate::catalog::{Catalog, REFINEMENTS, Stock, component_image, part_identity, part_name};
use crate::prices::{PriceSource, market_slug};
use crate::view::View;

pub(super) const FOUNDERS_ITEMS: [&str; 3] = [
    "/Lotus/Powersuits/Excalibur/ExcaliburPrime",
    "/Lotus/Weapons/Tenno/Pistol/LatoPrime",
    "/Lotus/Weapons/Tenno/Melee/LongSword/SkanaPrime",
];

const PRIME_COLLECTION_CATEGORIES: [&str; 8] = [
    "Warframes",
    "Archwing",
    "Primary",
    "Secondary",
    "Melee",
    "Arch-Gun",
    "Arch-Melee",
    "Pets",
];

fn is_sentinel_weapon(item: &Item) -> bool {
    item.product_category.as_deref() == Some("SentinelWeapons")
}

pub(super) fn group_of(item: &Item) -> MasteryGroup {
    if is_sentinel_weapon(item) {
        return MasteryGroup::Companions;
    }
    match item.category.as_str() {
        "Warframes" | "Archwing" => MasteryGroup::Warframes,
        "Sentinels" | "Pets" => MasteryGroup::Companions,
        "Primary" | "Secondary" | "Melee" | "Arch-Gun" | "Arch-Melee" => MasteryGroup::Weapons,
        _ => MasteryGroup::Other,
    }
}

pub(crate) fn kind_of(item: &Item) -> &'static str {
    match group_of(item) {
        MasteryGroup::Warframes => {
            if item.is_archwing() {
                "arch"
            } else if item.is_necramech() {
                "necramech"
            } else {
                "warframe"
            }
        }
        MasteryGroup::Companions => "companion",
        MasteryGroup::Weapons => match item.category.as_str() {
            "Primary" => "primary",
            "Secondary" => "secondary",
            "Melee" => "melee",
            _ => "arch",
        },
        MasteryGroup::Other => "modular",
    }
}

pub(crate) fn includes_founders(inventory: &Inventory, chosen: Option<bool>) -> bool {
    chosen.unwrap_or_else(|| inventory.is_founder())
}

pub(crate) fn masterable(catalog: &Catalog, include_founders: bool) -> impl Iterator<Item = &Item> {
    catalog
        .items()
        .filter(|item| item.masterable())
        .filter(|item| !item.name.starts_with("Kavasa Prime"))
        .filter(move |item| {
            include_founders || !FOUNDERS_ITEMS.contains(&item.unique_name.as_str())
        })
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a platinum cost is a small non-negative number"
)]
fn market_cost(missing: &[&MasteryComponent], prices: &dyn PriceSource) -> (u64, bool) {
    let quoted: Vec<(&MasteryComponent, f64)> = missing
        .iter()
        .filter_map(|component| {
            let plat = prices.plat(&market_slug(&component.name))?;
            (plat > 0.0).then_some((*component, plat))
        })
        .collect();
    let plat_cost = quoted
        .iter()
        .map(|(component, plat)| {
            let amount = (component.required - component.owned).max(0).unsigned_abs();
            (plat * amount as f64).round().max(0.0) as u64
        })
        .sum();
    let unpriced_blueprint = missing.iter().any(|component| {
        component.name.contains("Blueprint")
            && !quoted
                .iter()
                .any(|(other, _)| other.unique_name == component.unique_name)
    });
    (plat_cost, !unpriced_blueprint && !quoted.is_empty())
}

fn components_of(ledger: &Ledger<'_>, item: &Item) -> Vec<MasteryComponent> {
    item.components
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|component| {
            let owned = ledger.stock.count(&component.unique_name);
            let required = i64::from(component.item_count);
            MasteryComponent {
                unique_name: component.unique_name.clone(),
                name: part_name(item, component),
                image_name: component_image(item, component),
                owned,
                required,
                enough: owned >= required,
            }
        })
        .collect()
}

struct OwnedRelicDrops<'a> {
    by_reward: HashMap<&'a str, Vec<(f64, f64)>>,
}

impl<'a> OwnedRelicDrops<'a> {
    #[allow(
        clippy::cast_precision_loss,
        reason = "owned counts stay far below 2^53"
    )]
    fn new(catalog: &'a Catalog, stock: &Stock<'_>) -> Self {
        let mut by_reward: HashMap<&'a str, Vec<(f64, f64)>> = HashMap::new();
        for relic in catalog.relics() {
            for refinement in REFINEMENTS {
                let owned = relic
                    .unique_names
                    .get(&refinement)
                    .map_or(0, |unique_name| stock.count(unique_name));
                if owned <= 0 {
                    continue;
                }
                let copies = owned.unsigned_abs() as f64;
                for reward in relic.rewards_for(refinement) {
                    by_reward
                        .entry(part_identity(&reward.item_unique_name))
                        .or_default()
                        .push((reward.chance, copies));
                }
            }
        }
        Self { by_reward }
    }

    fn probability(&self, item: &MasteryItem) -> f64 {
        if !item.name.contains("Prime") {
            return 0.0;
        }
        let mut probability = 1.0_f64;
        for component in item.components.iter().filter(|component| !component.enough) {
            let mut none = 1.0_f64;
            for (chance, copies) in self
                .by_reward
                .get(part_identity(&component.unique_name))
                .map(Vec::as_slice)
                .unwrap_or_default()
            {
                none *= (1.0 - chance / 100.0).powf(*copies);
            }
            probability *= 1.0 - none;
        }
        probability
    }
}

pub(super) fn items(view: &View, options: MasteryOptions) -> Vec<MasteryItem> {
    let View {
        inventory,
        catalog,
        prices,
        favourites,
        ..
    } = *view;
    let owned_types = inventory.owned_item_types();
    let ledger = Ledger::new(inventory);
    let pending: HashSet<&str> = inventory
        .pending_recipes
        .iter()
        .map(|recipe| recipe.item_type.as_str())
        .collect();
    let mut rows: Vec<MasteryItem> = masterable(
        catalog,
        includes_founders(inventory, options.include_founders_items),
    )
    .map(|item| {
        let affinity = ledger.affinity_of(&item.unique_name);
        let current_level = item.mastery_rank_at(affinity);
        let max_level = item.max_mastery_rank();
        let components = components_of(&ledger, item);
        let pending_in_foundry = components
            .iter()
            .any(|component| pending.contains(component.unique_name.as_str()));
        let missing: Vec<&MasteryComponent> = components
            .iter()
            .filter(|component| !component.enough)
            .collect();
        let (plat_cost, purchasable) = market_cost(&missing, prices);
        MasteryItem {
            unique_name: item.unique_name.clone(),
            name: item.name.clone(),
            kind: kind_of(item),
            group: group_of(item),
            image_name: item.image_name.clone(),
            owned: owned_types.contains(item.unique_name.as_str()) || pending_in_foundry,
            mastered: affinity >= item.affinity_cap(),
            level: Level {
                current: current_level,
                max: max_level,
                xp_remaining: u64::from(
                    item.mastery_per_rank() * max_level.saturating_sub(current_level),
                ),
            },
            acquisition: Acquisition {
                missing_parts: missing.len(),
                plat_cost,
                purchasable,
                relic_probability: 0.0,
            },
            favourite: favourites.contains(&item.unique_name),
            components,
        }
    })
    .collect();
    let drops = OwnedRelicDrops::new(catalog, &ledger.stock);
    for row in &mut rows {
        row.acquisition.relic_probability = drops.probability(row);
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

pub(crate) fn unmastered_types<'a>(
    inventory: &Inventory,
    catalog: &'a Catalog,
) -> HashSet<&'a str> {
    let affinity = inventory.affinity_index();
    masterable(catalog, inventory.is_founder())
        .filter(|item| {
            affinity
                .get(item.unique_name.as_str())
                .copied()
                .unwrap_or_default()
                < item.affinity_cap()
        })
        .map(|item| item.unique_name.as_str())
        .collect()
}

pub(super) fn totals(items: &[MasteryItem]) -> (CategoryTotals, CategoryTotals, CategoryTotals) {
    let count = |group: MasteryGroup| {
        let max = items.iter().filter(|item| item.group == group).count();
        let current = items
            .iter()
            .filter(|item| item.group == group && item.mastered)
            .count();
        CategoryTotals::counted(current, max)
    };
    (
        count(MasteryGroup::Warframes),
        count(MasteryGroup::Weapons),
        count(MasteryGroup::Companions),
    )
}

fn is_prime_collection_item(item: &Item) -> bool {
    item.name.contains("Prime")
        && PRIME_COLLECTION_CATEGORIES.contains(&item.category.as_str())
        && !is_sentinel_weapon(item)
}

pub(crate) fn prime_ownership(
    inventory: &Inventory,
    catalog: &Catalog,
    options: MasteryOptions,
) -> (u32, u32) {
    let owned_types = inventory.owned_item_types();
    let ledger = Ledger::new(inventory);
    let mut owned = 0;
    let mut total = 0;
    for item in masterable(
        catalog,
        includes_founders(inventory, options.include_founders_items),
    )
    .filter(|item| is_prime_collection_item(item))
    {
        total += 1;
        if owned_types.contains(item.unique_name.as_str())
            || ledger.affinity_of(&item.unique_name) >= item.affinity_cap()
        {
            owned += 1;
        }
    }
    (owned, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::favourites::Favourites;
    use crate::listings::MarketListings;
    use crate::mastery::support::{
        entries, excluding_founders, founder_inventory, mutated, prices,
    };

    const MASTERED_PRIME_SUIT: &str = "/Lotus/Powersuits/Loki/LokiPrime";

    #[test]
    fn prime_collection_totals() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::mastery_catalog();
        let (owned, total) = prime_ownership(&inventory, &catalog, MasteryOptions::default());
        assert!(owned <= total);
        let primes: Vec<&Item> = masterable(&catalog, inventory.is_founder())
            .filter(|item| is_prime_collection_item(item))
            .collect();
        assert!(
            primes
                .iter()
                .all(|item| group_of(item) != MasteryGroup::Other)
        );
        assert!(
            !primes
                .iter()
                .any(|item| item.category == "Sentinels" || item.name.contains("Kavasa"))
        );
        assert_eq!(u32::try_from(primes.len()).unwrap(), total);
        let sold = mutated(|value| {
            entries(value, "Suits").retain(|entry| {
                entry.get("ItemType").and_then(serde_json::Value::as_str)
                    != Some(MASTERED_PRIME_SUIT)
            });
        });
        assert!(!sold.owned_item_types().contains(MASTERED_PRIME_SUIT));
        let (still_owned, sold_total) = prime_ownership(&sold, &catalog, MasteryOptions::default());
        assert_eq!(sold_total, total);
        assert_eq!(still_owned, owned);
    }

    #[test]
    fn prime_collection_with_founders() {
        let catalog = fixtures::mastery_catalog();
        let plain = prime_ownership(&fixtures::inventory(), &catalog, MasteryOptions::default());
        let founder = founder_inventory();
        let with_founders = prime_ownership(&founder, &catalog, MasteryOptions::default());
        let without_founders = prime_ownership(&founder, &catalog, excluding_founders());
        assert_eq!(with_founders.1 - plain.1, 3);
        assert_eq!(without_founders, plain);
        assert!(with_founders.0 < with_founders.1);
    }

    #[test]
    fn relic_probability_of_missing_part() {
        let inventory = fixtures::inventory_owning(&[
            ("/Lotus/Types/Game/Projections/T4VoidProjectionEBronze", 3),
            (
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeBlueprint",
                1,
            ),
            (
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeChassisComponent",
                1,
            ),
            (
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeHelmetComponent",
                1,
            ),
        ]);
        let catalog = fixtures::catalog();
        let rows = items(
            &View {
                inventory: &inventory,
                catalog: &catalog,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &MarketListings::default(),
            },
            MasteryOptions::default(),
        );
        let trinity = rows
            .iter()
            .find(|item| item.name == "Trinity Prime")
            .expect("Trinity Prime");
        let missing: Vec<&str> = trinity
            .components
            .iter()
            .filter(|component| !component.enough)
            .map(|component| component.name.as_str())
            .collect();
        assert_eq!(missing, vec!["Trinity Prime Systems"]);
        let expected = 1.0 - (1.0 - 25.33 / 100.0_f64).powf(3.0);
        assert!((trinity.acquisition.relic_probability - expected).abs() < 1e-9);
    }

    #[test]
    fn no_kavasa_prime() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::mastery_catalog();
        let rows = items(
            &View {
                inventory: &inventory,
                catalog: &catalog,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &MarketListings::default(),
            },
            MasteryOptions::default(),
        );
        assert!(
            !rows
                .iter()
                .any(|item| item.name.starts_with("Kavasa Prime"))
        );
    }
}
