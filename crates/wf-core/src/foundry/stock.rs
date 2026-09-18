use std::collections::HashMap;

use wf_data::Component;
use wf_inventory::{CountedItem, Inventory};

use crate::catalog::{FORMA_BLUEPRINT, FORMA_ITEM, builds_from_its_own_blueprint};

fn first_count(items: &[CountedItem], item_type: &str) -> Option<i64> {
    items
        .iter()
        .find(|item| item.item_type == item_type)
        .map(|item| item.item_count)
}

#[allow(
    clippy::cast_possible_wrap,
    reason = "an inventory holds far fewer than 2^63 weapons"
)]
fn weapon_instances(inventory: &Inventory, unique_name: &str) -> Option<i64> {
    let count = [&inventory.melee, &inventory.pistols, &inventory.long_guns]
        .into_iter()
        .flatten()
        .filter(|item| item.item_type == unique_name)
        .count();
    (count > 0).then_some(count as i64)
}

fn held_count(inventory: &Inventory, unique_name: &str) -> i64 {
    if unique_name.contains("Blueprint") {
        return first_count(&inventory.recipes, unique_name).unwrap_or_default();
    }
    if unique_name.contains("Weapons")
        && let Some(count) = weapon_instances(inventory, unique_name)
    {
        return count;
    }
    if let Some(count) = first_count(&inventory.misc_items, unique_name) {
        return count;
    }
    if unique_name.contains("Component") {
        return first_count(
            &inventory.recipes,
            &unique_name.replace("Component", "Blueprint"),
        )
        .unwrap_or_default();
    }
    0
}

fn ambassador_blueprint(unique_name: &str) -> Option<String> {
    if !unique_name.contains("CrpArSniper") || unique_name.contains("Blueprint") {
        return None;
    }
    Some(format!(
        "{}Blueprint",
        unique_name.replace("CrpArSniper", "Ambassador")
    ))
}

pub(super) fn component_stock(inventory: &Inventory, unique_name: &str, name: &str) -> i64 {
    if let Some(blueprint) = ambassador_blueprint(unique_name) {
        return held_count(inventory, &blueprint);
    }
    if name.contains("Forma") {
        return held_count(inventory, FORMA_ITEM) + held_count(inventory, FORMA_BLUEPRINT);
    }
    let built = held_count(inventory, unique_name);
    if builds_from_its_own_blueprint(unique_name) {
        return built + held_count(inventory, &format!("{unique_name}Blueprint"));
    }
    built
}

#[allow(
    clippy::cast_possible_wrap,
    reason = "an inventory holds far fewer than 2^63 items"
)]
fn every_collection_count(inventory: &Inventory, unique_name: &str) -> i64 {
    let equipment = inventory
        .equipment()
        .filter(|item| item.item_type == unique_name)
        .count() as i64;
    let misc: i64 = inventory
        .misc_items
        .iter()
        .filter(|item| item.item_type == unique_name)
        .map(|item| item.item_count)
        .sum();
    let recipes: i64 = inventory
        .recipes
        .iter()
        .filter(|item| item.item_type == unique_name)
        .map(|item| {
            if item.item_count == 0 {
                1
            } else {
                item.item_count
            }
        })
        .sum();
    equipment + misc + recipes
}

pub(super) fn tree_stock(inventory: &Inventory, unique_name: &str) -> i64 {
    let stock = every_collection_count(inventory, unique_name);
    if stock > 0 || !unique_name.ends_with("Component") {
        return stock;
    }
    every_collection_count(inventory, &unique_name.replace("Component", "Blueprint"))
}

pub(super) struct SlotFill {
    pub required: i64,
    pub filled: i64,
    pub satisfied: bool,
}

pub(super) fn fill_slots(inventory: &Inventory, components: &[Component]) -> Vec<SlotFill> {
    let stock: Vec<i64> = components
        .iter()
        .map(|component| component_stock(inventory, &component.unique_name, &component.name))
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
