use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use wf_inventory::Inventory;

pub(crate) const COUNTED_CATEGORIES: [&str; 6] = [
    "MiscItems",
    "Recipes",
    "RawUpgrades",
    "Consumables",
    "LevelKeys",
    "FusionTreasures",
];

pub(crate) const EQUIPMENT_CATEGORIES: [&str; 14] = [
    "Suits",
    "LongGuns",
    "Pistols",
    "Melee",
    "SpaceSuits",
    "SpaceGuns",
    "SpaceMelee",
    "Sentinels",
    "SentinelWeapons",
    "MechSuits",
    "Hoverboards",
    "MoaPets",
    "KubrowPets",
    "DataKnives",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDelta {
    pub item_type: String,
    pub category: String,
    pub before: i64,
    pub after: i64,
}

impl ItemDelta {
    pub fn delta(&self) -> i64 {
        self.after - self.before
    }
}

pub(crate) fn compute(prev: &Inventory, next: &Inventory) -> Vec<ItemDelta> {
    let mut deltas = Vec::new();
    for (index, category) in COUNTED_CATEGORIES.into_iter().enumerate() {
        let before = counted_map(prev, index);
        let after = counted_map(next, index);
        push_changes(&mut deltas, category, &before, &after);
    }
    for (index, category) in EQUIPMENT_CATEGORIES.into_iter().enumerate() {
        let before = equipment_map(prev, index);
        let after = equipment_map(next, index);
        push_changes(&mut deltas, category, &before, &after);
    }
    deltas
}

fn counted_map(inventory: &Inventory, index: usize) -> BTreeMap<&str, i64> {
    let mut map = BTreeMap::new();
    for item in inventory.counted_categories()[index] {
        *map.entry(item.item_type.as_str()).or_insert(0) += item.item_count;
    }
    map
}

fn equipment_map(inventory: &Inventory, index: usize) -> BTreeMap<&str, i64> {
    let mut map = BTreeMap::new();
    for item in inventory.equipment_categories()[index] {
        map.insert(item.item_type.as_str(), 1);
    }
    map
}

fn push_changes(
    deltas: &mut Vec<ItemDelta>,
    category: &str,
    before: &BTreeMap<&str, i64>,
    after: &BTreeMap<&str, i64>,
) {
    for (item_type, after_count) in after {
        let before_count = before.get(item_type).copied().unwrap_or(0);
        if before_count != *after_count {
            deltas.push(ItemDelta {
                item_type: (*item_type).to_owned(),
                category: category.to_owned(),
                before: before_count,
                after: *after_count,
            });
        }
    }
    for (item_type, before_count) in before {
        if !after.contains_key(item_type) {
            deltas.push(ItemDelta {
                item_type: (*item_type).to_owned(),
                category: category.to_owned(),
                before: *before_count,
                after: 0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;

    fn mutated(inventory_json: &str) -> Inventory {
        let with_more_ferrite = inventory_json.replacen(
            r#"{"ItemCount":51923112,"ItemType":"/Lotus/Types/Items/MiscItems/Ferrite"}"#,
            r#"{"ItemCount":51923212,"ItemType":"/Lotus/Types/Items/MiscItems/Ferrite"}"#,
            1,
        );
        assert_ne!(with_more_ferrite, inventory_json);
        let with_new_suit = with_more_ferrite.replacen(
            r#""Suits":[{"#,
            r#""Suits":[{"ItemType":"/Lotus/Powersuits/Ninja/Ninja","XP":0,"ItemId":{"$oid":"000000000000000000000001"}},{"#,
            1,
        );
        let with_xp_only_change = with_new_suit.replacen(r#""XP":752834"#, r#""XP":752999"#, 1);
        assert_ne!(with_xp_only_change, with_new_suit);
        Inventory::parse(&with_xp_only_change).unwrap()
    }

    #[test]
    fn counts_and_equipment_not_xp() {
        let previous = fixtures::inventory();
        let next = mutated(fixtures::INVENTORY);
        let deltas = compute(&previous, &next);

        let ferrite = deltas
            .iter()
            .find(|delta| delta.item_type.ends_with("MiscItems/Ferrite"))
            .unwrap();
        assert_eq!(ferrite.category, "MiscItems");
        assert_eq!(ferrite.delta(), 100);

        let suit = deltas
            .iter()
            .find(|delta| delta.item_type == "/Lotus/Powersuits/Ninja/Ninja")
            .unwrap();
        assert_eq!(suit.category, "Suits");
        assert_eq!((suit.before, suit.after), (0, 1));

        assert!(
            !deltas
                .iter()
                .any(|delta| delta.item_type.contains("TnHackingDevice")),
            "xp-only changes must not produce deltas"
        );
        assert_eq!(deltas.len(), 2);
    }

    #[test]
    fn removed_item() {
        let previous = fixtures::inventory();
        let stripped = fixtures::INVENTORY.replacen(
            r#"{"ItemCount":51923112,"ItemType":"/Lotus/Types/Items/MiscItems/Ferrite"},"#,
            "",
            1,
        );
        let next = Inventory::parse(&stripped).unwrap();
        let deltas = compute(&previous, &next);
        let ferrite = deltas
            .iter()
            .find(|delta| delta.item_type.ends_with("MiscItems/Ferrite"))
            .unwrap();
        assert_eq!(ferrite.after, 0);
        assert_eq!(ferrite.delta(), -51_923_112);
    }

    #[test]
    fn identical_inventories() {
        let inventory = fixtures::inventory();
        assert!(compute(&inventory, &inventory).is_empty());
    }
}
