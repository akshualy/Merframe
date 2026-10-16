use std::collections::HashMap;

use wf_data::Component;
use wf_inventory::{CountedItem, Inventory};

use crate::catalog::{aliased_count, forma_count};
use crate::identity::{ambassador_blueprint, blueprint_of};

pub(crate) struct FoundryStock {
    recipes: HashMap<String, i64>,
    misc: HashMap<String, i64>,
    weapons: HashMap<String, i64>,
    collected: HashMap<String, i64>,
}

fn first_counts(items: &[CountedItem]) -> HashMap<String, i64> {
    let mut counts = HashMap::new();
    for item in items {
        counts
            .entry(item.item_type.clone())
            .or_insert(item.item_count);
    }
    counts
}

impl FoundryStock {
    pub(crate) fn new(inventory: &Inventory) -> Self {
        let mut weapons: HashMap<String, i64> = HashMap::new();
        for item in [&inventory.melee, &inventory.pistols, &inventory.long_guns]
            .into_iter()
            .flatten()
        {
            *weapons.entry(item.item_type.clone()).or_insert(0) += 1;
        }
        let mut collected: HashMap<String, i64> = HashMap::new();
        for item in inventory.equipment() {
            *collected.entry(item.item_type.clone()).or_insert(0) += 1;
        }
        for item in &inventory.misc_items {
            *collected.entry(item.item_type.clone()).or_insert(0) += item.item_count;
        }
        for item in &inventory.recipes {
            *collected.entry(item.item_type.clone()).or_insert(0) += if item.item_count == 0 {
                1
            } else {
                item.item_count
            };
        }
        Self {
            recipes: first_counts(&inventory.recipes),
            misc: first_counts(&inventory.misc_items),
            weapons,
            collected,
        }
    }

    fn held(&self, unique_name: &str) -> i64 {
        if unique_name.contains("Blueprint") {
            return self.recipes.get(unique_name).copied().unwrap_or_default();
        }
        if unique_name.contains("Weapons")
            && let Some(count) = self.weapons.get(unique_name)
        {
            return *count;
        }
        if let Some(count) = self.misc.get(unique_name) {
            return *count;
        }

        blueprint_of(unique_name)
            .and_then(|blueprint| self.recipes.get(&blueprint).copied())
            .unwrap_or_default()
    }

    pub(super) fn component(&self, unique_name: &str, name: &str) -> i64 {
        if let Some(blueprint) = ambassador_blueprint(unique_name) {
            return self.held(&blueprint);
        }
        if name.contains("Forma") {
            return forma_count(|item_type| self.held(item_type));
        }

        aliased_count(unique_name, |item_type| self.held(item_type))
    }

    fn collected(&self, unique_name: &str) -> i64 {
        self.collected.get(unique_name).copied().unwrap_or_default()
    }

    pub(super) fn in_tree(&self, unique_name: &str) -> i64 {
        let stock = self.collected(unique_name);
        if stock > 0 {
            return stock;
        }

        blueprint_of(unique_name).map_or(0, |blueprint| self.collected(&blueprint))
    }

    pub(super) fn fill_slots(&self, components: &[Component]) -> Vec<SlotFill> {
        let stock: Vec<i64> = components
            .iter()
            .map(|component| self.component(&component.unique_name, &component.name))
            .collect();
        let mut shared: HashMap<&str, i64> = HashMap::new();
        for (component, held) in components.iter().zip(&stock) {
            let entry = shared.entry(component.unique_name.as_str()).or_insert(0);
            *entry = (*entry).max(*held);
        }
        components
            .iter()
            .zip(stock)
            .map(|(component, held)| {
                let required = i64::from(component.item_count);
                let left = shared.entry(component.unique_name.as_str()).or_insert(0);
                let filled = if *left < required {
                    std::mem::take(left)
                } else {
                    *left -= required;
                    held
                };
                SlotFill {
                    required,
                    filled,
                    satisfied: filled >= required,
                }
            })
            .collect()
    }
}

pub(super) struct SlotFill {
    pub required: i64,
    pub filled: i64,
    pub satisfied: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures::{
        self, SINGLE_KAMA, TRINITY_PRIME_SYSTEMS, TRINITY_PRIME_SYSTEMS_BLUEPRINT,
    };
    use crate::catalog::{FORMA_BLUEPRINT, FORMA_ITEM};

    const SNIPER_BARREL: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/CrpArSniperBarrel";
    const AMBASSADOR_BARREL_BLUEPRINT: &str =
        "/Lotus/Types/Recipes/Weapons/WeaponParts/AmbassadorBarrelBlueprint";
    const KEY_BLUEPRINT: &str = "/Lotus/Types/Recipes/Components/DerelictKeyBlueprint";

    fn stock() -> FoundryStock {
        FoundryStock::new(&fixtures::inventory_stocked(
            &[("Melee", SINGLE_KAMA), ("Melee", SINGLE_KAMA)],
            &[
                ("Recipes", TRINITY_PRIME_SYSTEMS_BLUEPRINT, 3),
                ("Recipes", TRINITY_PRIME_SYSTEMS_BLUEPRINT, 4),
                ("Recipes", AMBASSADOR_BARREL_BLUEPRINT, 1),
                ("Recipes", KEY_BLUEPRINT, 0),
                ("Recipes", FORMA_BLUEPRINT, 2),
                ("MiscItems", FORMA_ITEM, 5),
            ],
        ))
    }

    #[test]
    fn component_counts() {
        let stock = stock();
        assert_eq!(stock.component(SINGLE_KAMA, "Kama"), 2);
        assert_eq!(
            stock.component(TRINITY_PRIME_SYSTEMS_BLUEPRINT, "Blueprint"),
            3
        );
        assert_eq!(stock.component(TRINITY_PRIME_SYSTEMS, "Systems"), 3);
        assert_eq!(stock.component(SNIPER_BARREL, "Barrel"), 1);
        assert_eq!(stock.component(FORMA_ITEM, "Forma"), 7);
        assert_eq!(stock.component(KEY_BLUEPRINT, "Blueprint"), 0);
    }

    #[test]
    fn tree_counts() {
        let stock = stock();
        assert_eq!(stock.in_tree(SINGLE_KAMA), 2);
        assert_eq!(stock.in_tree(TRINITY_PRIME_SYSTEMS_BLUEPRINT), 7);
        assert_eq!(stock.in_tree(TRINITY_PRIME_SYSTEMS), 7);
        assert_eq!(stock.in_tree(KEY_BLUEPRINT), 1);
        assert_eq!(stock.in_tree(FORMA_ITEM), 5);
        assert_eq!(stock.in_tree(SNIPER_BARREL), 0);
    }
}
