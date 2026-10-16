use std::collections::{HashMap, HashSet};

use wf_data::Item;
use wf_inventory::Inventory;

use crate::catalog::Stock;
use crate::foundry::FoundryStock;
use crate::inventory_view::ItemStatus;

pub(crate) struct Account {
    pub(crate) inventory: Inventory,
    pub(crate) stock: Stock,
    pub(crate) foundry: FoundryStock,
    affinity: HashMap<String, u64>,
    built: HashSet<String>,
}

impl Account {
    pub(crate) fn new(inventory: Inventory) -> Self {
        Self {
            stock: Stock::new(&inventory),
            foundry: FoundryStock::new(&inventory),
            affinity: inventory
                .affinity_index()
                .into_iter()
                .map(|(item_type, xp)| (item_type.to_owned(), xp))
                .collect(),
            built: inventory
                .owned_item_types()
                .into_iter()
                .map(str::to_owned)
                .collect(),
            inventory,
        }
    }

    pub(crate) fn affinity_of(&self, unique_name: &str) -> u64 {
        self.affinity.get(unique_name).copied().unwrap_or_default()
    }

    pub(crate) fn built(&self, unique_name: &str) -> bool {
        self.built.contains(unique_name)
    }

    pub(crate) fn mastered(&self, item: &Item) -> bool {
        self.affinity_of(&item.unique_name) >= item.affinity_cap()
    }

    pub(crate) fn holds(&self, item: &Item) -> bool {
        self.built(&item.unique_name) || self.mastered(item)
    }

    pub(crate) fn status(&self, item: &Item) -> ItemStatus {
        ItemStatus {
            built: self.built(&item.unique_name),
            mastered: self.mastered(item),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let account = Account::new(inventory);
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
        let account = Account::new(fixtures::inventory_stocked(&[], &[]));
        assert!(!account.built(LOKI_PRIME));
        assert!(account.mastered(loki));
        assert!(account.holds(loki));
        assert_eq!(account.stock.count(FORMA_BLUEPRINT), 0);

        let fresh = Account::new(fixtures::inventory_stocked(&[("Suits", LOKI_PRIME)], &[]));
        assert!(fresh.built(LOKI_PRIME));
    }
}
