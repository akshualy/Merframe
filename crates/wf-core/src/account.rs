use std::collections::{HashMap, HashSet};

use serde::Serialize;
use wf_data::Item;
use wf_inventory::Inventory;

use crate::catalog::{Catalog, Stock, own_blueprint};
use crate::foundry::FoundryStock;
use crate::identity::ItemTable;
use crate::inventory_view::{ItemStatus, held_part_counts};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum WorldOfferOwnership {
    Missing,
    Owned,
    Mastered,
    InFoundry,
}

pub(crate) struct Account {
    pub(crate) inventory: Inventory,
    pub(crate) stock: Stock,
    pub(crate) foundry: FoundryStock,
    affinity: HashMap<String, u64>,
    built: HashSet<String>,
    owned: HashSet<String>,
    pending_builds: HashMap<String, i64>,
    part_copies: HashMap<String, i64>,
}

fn pending_builds(inventory: &Inventory, catalog: &Catalog) -> HashMap<String, i64> {
    let mut builds = HashMap::new();
    for recipe in &inventory.pending_recipes {
        let component = catalog
            .component_for_stock(&recipe.item_type)
            .map_or(recipe.item_type.as_str(), |(_, component)| {
                component.unique_name.as_str()
            });
        *builds.entry(component.to_owned()).or_insert(0) += 1;
    }
    builds
}

impl Account {
    pub(crate) fn new(inventory: Inventory, catalog: &Catalog, items: &ItemTable) -> Self {
        Self {
            stock: Stock::new(&inventory),
            foundry: FoundryStock::new(&inventory),
            affinity: inventory
                .affinity_index()
                .into_iter()
                .map(|(item_type, xp)| (item_type.to_owned(), xp))
                .collect(),
            built: inventory
                .built_item_types()
                .into_iter()
                .map(str::to_owned)
                .collect(),
            owned: inventory.held_item_types().map(str::to_owned).collect(),
            pending_builds: pending_builds(&inventory, catalog),
            part_copies: held_part_counts(&inventory, items),
            inventory,
        }
    }

    pub(crate) fn affinity_of(&self, unique_name: &str) -> u64 {
        self.affinity.get(unique_name).copied().unwrap_or_default()
    }

    pub(crate) fn built(&self, unique_name: &str) -> bool {
        self.built.contains(unique_name)
    }

    pub(crate) fn owns(&self, unique_name: &str) -> bool {
        self.owned.contains(unique_name)
    }

    pub(crate) fn pending_builds(&self, component: &str) -> i64 {
        self.pending_builds
            .get(component)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn part_copies(&self, part: &str) -> i64 {
        self.part_copies.get(part).copied().unwrap_or_default()
    }

    pub(crate) fn building(&self, item: &Item) -> i64 {
        own_blueprint(item).map_or(0, |blueprint| self.pending_builds(blueprint))
    }

    pub(crate) fn in_foundry(&self, item: &Item) -> bool {
        item.components
            .iter()
            .flatten()
            .any(|component| self.pending_builds(&component.unique_name) > 0)
    }

    pub(crate) fn mastered(&self, item: &Item) -> bool {
        self.affinity_of(&item.unique_name) >= item.affinity_cap()
    }

    pub(crate) fn built_or_building(&self, item: &Item) -> bool {
        self.built(&item.unique_name) || self.building(item) > 0
    }

    pub(crate) fn holds(&self, item: &Item) -> bool {
        self.built_or_building(item) || self.mastered(item)
    }

    pub(crate) fn status(&self, item: &Item) -> ItemStatus {
        ItemStatus {
            built: self.built(&item.unique_name),
            mastered: self.mastered(item),
        }
    }

    pub(crate) fn world_offer_ownership(
        &self,
        unique_name: &str,
        catalog: &Catalog,
    ) -> WorldOfferOwnership {
        let item = catalog.item(unique_name);
        if item.is_some_and(|item| self.in_foundry(item)) {
            WorldOfferOwnership::InFoundry
        } else if item.is_some_and(|item| self.mastered(item)) {
            WorldOfferOwnership::Mastered
        } else if self.owns(unique_name) {
            WorldOfferOwnership::Owned
        } else {
            WorldOfferOwnership::Missing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    use crate::catalog::fixtures::{EXCALIBUR, drop_entries, entries, inventory_with};
    use crate::catalog::{FORMA_BLUEPRINT, fixtures};

    const LOKI_PRIME: &str = "/Lotus/Powersuits/Loki/LokiPrime";
    const UNKNOWN: &str = "/Lotus/Powersuits/Nobody/Nobody";

    #[test]
    fn fixture_account() {
        let catalog = fixtures::mastery_catalog();
        let loki = catalog.item(LOKI_PRIME).unwrap();
        let inventory = fixtures::inventory();
        let xp = inventory
            .xp_info
            .iter()
            .find(|entry| entry.item_type == LOKI_PRIME)
            .unwrap()
            .xp;
        let account = Account::new(inventory, &catalog, &ItemTable::build(&catalog));
        assert_eq!(account.affinity_of(LOKI_PRIME), xp);
        assert!(account.built(LOKI_PRIME));
        assert!(account.mastered(loki));
        assert!(account.holds(loki));
        assert_eq!(account.stock.count(FORMA_BLUEPRINT), 90);
        assert_eq!(account.affinity_of(UNKNOWN), 0);
        assert!(!account.built(UNKNOWN));
    }

    #[test]
    fn sold_but_mastered_is_held() {
        let catalog = fixtures::mastery_catalog();
        let loki = catalog.item(LOKI_PRIME).unwrap();
        let account = Account::new(
            fixtures::inventory_stocked(&[], &[]),
            &catalog,
            &ItemTable::build(&catalog),
        );
        assert!(!account.built(LOKI_PRIME));
        assert!(account.mastered(loki));
        assert!(account.holds(loki));
        assert_eq!(account.stock.count(FORMA_BLUEPRINT), 0);

        let fresh = Account::new(
            fixtures::inventory_stocked(&[("Suits", LOKI_PRIME)], &[]),
            &catalog,
            &ItemTable::build(&catalog),
        );
        assert!(fresh.built(LOKI_PRIME));
    }

    fn unranked(value: &mut Value) {
        value["XPInfo"] = json!([]);
        drop_entries(value, &["Suits"], EXCALIBUR);
    }

    fn excalibur_ownership(edit: impl FnOnce(&mut Value)) -> WorldOfferOwnership {
        let catalog = fixtures::catalog();
        let account = Account::new(inventory_with(edit), &catalog, &ItemTable::build(&catalog));
        account.world_offer_ownership(EXCALIBUR, &catalog)
    }

    #[test]
    fn world_offer_ownership_ladder() {
        assert_eq!(excalibur_ownership(|_| {}), WorldOfferOwnership::Mastered);
        assert_eq!(excalibur_ownership(unranked), WorldOfferOwnership::Missing);
        assert_eq!(
            excalibur_ownership(|value| {
                unranked(value);
                entries(value, "Suits").push(json!({
                    "ItemType": EXCALIBUR,
                    "ItemId": { "$oid": "00000000000000000000beef" },
                    "XP": 0,
                }));
            }),
            WorldOfferOwnership::Owned
        );
        assert_eq!(
            excalibur_ownership(|value| {
                entries(value, "PendingRecipes").push(json!({
                    "ItemType": "/Lotus/Types/Recipes/WarframeRecipes/ExcaliburHelmetBlueprint",
                    "CompletionDate": { "$date": { "$numberLong": "1787000100000" } },
                    "ItemId": { "$oid": "0123456789abcdef01234567" }
                }));
            }),
            WorldOfferOwnership::InFoundry
        );
    }
}
