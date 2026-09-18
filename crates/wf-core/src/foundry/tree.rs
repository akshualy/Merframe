use std::collections::{BTreeMap, HashMap};

use wf_data::{Component, Item};
use wf_inventory::Inventory;

use super::stock::{fill_slots, tree_stock};
use super::tab::component_name;
use super::{CraftDetails, CraftNode, CraftSummary, MissingComponent, NeededItem};
use crate::catalog::{Catalog, RECIPE_PREFIX};

const FARMED_TYPES: &[&str] = &[
    "Resource",
    "Gem",
    "Cut Gem",
    "Alloy",
    "Misc",
    "Pet Resource",
];

pub(crate) fn details(
    inventory: &Inventory,
    catalog: &Catalog,
    unique_name: &str,
) -> Option<CraftDetails> {
    let item = catalog
        .item(unique_name)
        .filter(|item| item.components.is_some())?;
    let tree = craft_tree(inventory, catalog, item);
    Some(CraftDetails {
        summary: craft_summary(catalog, unique_name, &tree),
        missing: missing_components(inventory, catalog, item),
        tree,
    })
}

fn craft_summary(catalog: &Catalog, unique_name: &str, tree: &[CraftNode]) -> CraftSummary {
    let cost = build_cost(catalog, unique_name);
    let mut summary = CraftSummary {
        credits: cost.credits,
        build_secs: cost.secs,
        ..CraftSummary::default()
    };
    for node in tree {
        let branch = branch_summary(catalog, node);
        summary.credits += branch.credits;
        summary.build_secs += branch.build_secs;
        summary.shortest_secs = summary.shortest_secs.max(branch.shortest_secs);
    }
    summary.shortest_secs += cost.secs;
    let (blueprints, resources) = shopping_list(tree);
    summary.blueprints_needed = blueprints;
    summary.resources_needed = resources;
    summary
}

fn shopping_list(tree: &[CraftNode]) -> (Vec<NeededItem>, Vec<NeededItem>) {
    let mut totals: BTreeMap<&str, NeededItem> = BTreeMap::new();
    for node in tree {
        gather_leaves(node, &mut totals);
    }
    let mut blueprints: Vec<NeededItem> = Vec::new();
    let mut resources: Vec<NeededItem> = Vec::new();
    for needed in totals.into_values().filter(|needed| needed.amount > 0) {
        if needed.unique_name.contains("Blueprint") || needed.unique_name.starts_with(RECIPE_PREFIX)
        {
            blueprints.push(needed);
        } else {
            resources.push(needed);
        }
    }
    blueprints.sort_by(|a, b| a.name.cmp(&b.name));
    resources.sort_by(|a, b| a.name.cmp(&b.name));
    (blueprints, resources)
}

fn gather_leaves<'a>(node: &'a CraftNode, totals: &mut BTreeMap<&'a str, NeededItem>) {
    if !node.children.is_empty() {
        for child in &node.children {
            gather_leaves(child, totals);
        }
        return;
    }
    if node.covered {
        return;
    }
    let wanted = node.crafts_queued + node.short_by / node.per_craft.max(1);
    let entry = totals
        .entry(node.unique_name.as_str())
        .or_insert_with(|| NeededItem {
            unique_name: node.unique_name.clone(),
            name: node.name.clone(),
            image_name: node.image_name.clone(),
            amount: 0,
        });
    entry.amount += wanted;
}

struct BuildCost {
    credits: i64,
    secs: i64,
}

fn build_cost(catalog: &Catalog, unique_name: &str) -> BuildCost {
    let component = catalog.component(unique_name).map(|(_, part)| part);
    let item = catalog.item(unique_name);
    BuildCost {
        credits: component
            .and_then(|part| part.build_price)
            .or_else(|| item.and_then(|item| item.build_price))
            .map_or(0, i64::from),
        secs: component
            .and_then(|part| part.build_time)
            .or_else(|| item.and_then(|item| item.build_time))
            .map_or(0, i64::from),
    }
}

fn craft_runs(node: &CraftNode) -> i64 {
    if node.stocked || node.children.is_empty() {
        return 0;
    }
    let outstanding = node.short_by.max(0) + node.per_craft - 1;
    (node.crafts_queued + outstanding) / node.per_craft
}

fn branch_summary(catalog: &Catalog, node: &CraftNode) -> CraftSummary {
    let runs = craft_runs(node);
    if runs == 0 {
        return CraftSummary::default();
    }
    let cost = build_cost(catalog, &node.unique_name);
    let mut summary = CraftSummary {
        credits: runs * cost.credits,
        build_secs: runs * cost.secs,
        ..CraftSummary::default()
    };
    for child in &node.children {
        let branch = branch_summary(catalog, child);
        summary.credits += branch.credits;
        summary.build_secs += branch.build_secs;
        summary.shortest_secs = summary.shortest_secs.max(branch.shortest_secs);
    }
    summary.shortest_secs += runs * cost.secs;
    summary
}

fn missing_components(
    inventory: &Inventory,
    catalog: &Catalog,
    item: &Item,
) -> Vec<MissingComponent> {
    let components = item.components.as_deref().unwrap_or_default();
    fill_slots(inventory, components)
        .into_iter()
        .zip(components)
        .filter(|(slot, _)| !slot.satisfied)
        .map(|(slot, component)| MissingComponent {
            unique_name: component.unique_name.clone(),
            name: component_name(catalog, item, component),
            image_name: catalog
                .icon_for(&component.unique_name)
                .or_else(|| component.image_name.clone()),
            owned: slot.filled,
            required: slot.required,
        })
        .collect()
}

fn per_craft(catalog: &Catalog, unique_name: &str) -> i64 {
    let quantity = catalog
        .item(unique_name)
        .and_then(|item| item.build_quantity);
    match quantity {
        Some(quantity) if quantity > 0 => i64::from(quantity),
        _ => 1,
    }
}

fn craft_tree(inventory: &Inventory, catalog: &Catalog, item: &Item) -> Vec<CraftNode> {
    let mut path: Vec<&str> = Vec::new();
    let mut nodes: Vec<CraftNode> = item
        .components
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|component| node(inventory, catalog, item, component, &mut path))
        .collect();
    let mut pool = MaterialPool::over(&nodes);
    for node in &mut nodes {
        settle(node, &mut pool);
    }
    nodes
}

fn node<'a>(
    inventory: &Inventory,
    catalog: &'a Catalog,
    parent: &Item,
    component: &'a Component,
    path: &mut Vec<&'a str>,
) -> CraftNode {
    let recipe = if path.contains(&component.unique_name.as_str()) {
        None
    } else {
        sub_recipe(inventory, catalog, &component.unique_name)
    };
    let children = match recipe {
        None => Vec::new(),
        Some(recipe) => {
            path.push(component.unique_name.as_str());
            let children = recipe
                .components
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|child| node(inventory, catalog, recipe, child, path))
                .collect();
            path.pop();
            children
        }
    };
    CraftNode {
        unique_name: component.unique_name.clone(),
        name: component_name(catalog, parent, component),
        image_name: catalog
            .icon_for(&component.unique_name)
            .or_else(|| component.image_name.clone()),
        required: i64::from(component.item_count),
        owned: tree_stock(inventory, &component.unique_name),
        per_craft: per_craft(catalog, &component.unique_name),
        short_by: 0,
        crafts_queued: 0,
        craftable: false,
        stocked: false,
        covered: false,
        children,
    }
}

struct MaterialPool {
    left: HashMap<String, i64>,
}

impl MaterialPool {
    fn over(nodes: &[CraftNode]) -> Self {
        let mut pool = Self {
            left: HashMap::new(),
        };
        for node in nodes {
            pool.seed(node);
        }
        pool
    }

    fn seed(&mut self, node: &CraftNode) {
        self.left
            .entry(node.unique_name.clone())
            .or_insert(node.owned);
        for child in &node.children {
            self.seed(child);
        }
    }

    fn take(&mut self, unique_name: &str, wanted: i64) -> i64 {
        let Some(left) = self.left.get_mut(unique_name) else {
            return 0;
        };
        if *left < wanted {
            std::mem::take(left)
        } else {
            *left -= wanted;
            wanted
        }
    }
}

fn settle(node: &mut CraftNode, pool: &mut MaterialPool) {
    let taken = pool.take(&node.unique_name, node.required);
    if taken == node.required {
        node.stocked = true;
        mark_covered(node);
        return;
    }
    node.short_by = node.required - taken;
    while node.short_by > 0 && !node.children.is_empty() {
        for child in &mut node.children {
            settle(child, pool);
        }
        if !node
            .children
            .iter()
            .all(|child| child.stocked || child.craftable)
        {
            break;
        }
        node.short_by -= node.per_craft;
        node.crafts_queued += node.per_craft;
    }
    node.craftable = node.short_by == 0;
}

fn mark_covered(node: &mut CraftNode) {
    node.covered = true;
    if node.required > node.owned {
        node.short_by = node.required - node.owned;
    }
    for child in &mut node.children {
        mark_covered(child);
    }
}

fn farmed_resource(item: &Item) -> bool {
    item.category == "Resources" || FARMED_TYPES.contains(&item.item_type.as_str())
}

fn blueprint_in_stock(inventory: &Inventory, item: &Item) -> bool {
    item.components
        .iter()
        .flatten()
        .filter(|component| component.unique_name.ends_with("Blueprint"))
        .any(|component| tree_stock(inventory, &component.unique_name) > 0)
}

fn sub_recipe<'a>(
    inventory: &Inventory,
    catalog: &'a Catalog,
    unique_name: &str,
) -> Option<&'a Item> {
    catalog
        .item(unique_name)
        .filter(|item| item.components.is_some())
        .filter(|item| !farmed_resource(item) || blueprint_in_stock(inventory, item))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{FORMA_ITEM, fixtures};
    use crate::foundry::stock::component_stock;

    const DUAL_KAMAS: &str = "/Lotus/Weapons/Tenno/Melee/DualKamas/DualKamas";
    const SINGLE_KAMA: &str = "/Lotus/Weapons/Tenno/Melee/DualKamas/SingleKama";
    const LUA_LENS: &str = "/Lotus/Upgrades/Focus/PowerLensLua";
    const EIDOLON_LENS: &str = "/Lotus/Upgrades/Focus/PowerLensOstron";
    const GREATER_LENS: &str = "/Lotus/Upgrades/Focus/PowerLensGreater";
    const MORPHICS: &str = "/Lotus/Types/Items/MiscItems/Morphic";
    const NEURAL_SENSOR: &str = "/Lotus/Types/Items/MiscItems/NeuralSensor";
    const DETONITE_INJECTOR: &str = "/Lotus/Types/Items/Research/ChemComponent";
    const CONTROL_MODULE: &str = "/Lotus/Types/Items/MiscItems/ControlModule";

    fn recipe<'a>(catalog: &'a Catalog, unique_name: &str) -> &'a Item {
        catalog
            .item(unique_name)
            .filter(|item| item.components.is_some())
            .unwrap()
    }

    fn child<'a>(nodes: &'a [CraftNode], unique_name: &str) -> &'a CraftNode {
        nodes
            .iter()
            .find(|node| node.unique_name == unique_name)
            .unwrap()
    }

    fn node_depth(node: &CraftNode) -> usize {
        1 + node
            .children
            .iter()
            .map(node_depth)
            .max()
            .unwrap_or_default()
    }

    #[test]
    fn duplicate_slots() {
        let catalog = fixtures::foundry_catalog();
        let dual_kamas = recipe(&catalog, DUAL_KAMAS);
        assert_eq!(
            dual_kamas
                .components
                .iter()
                .flatten()
                .filter(|component| component.unique_name == SINGLE_KAMA)
                .count(),
            2
        );

        let none = fixtures::inventory_stocked(&[], &[]);
        assert_eq!(
            missing_components(&none, &catalog, dual_kamas)
                .iter()
                .filter(|missing| missing.unique_name == SINGLE_KAMA)
                .count(),
            2
        );

        let one = fixtures::inventory_stocked(&[("Melee", SINGLE_KAMA)], &[]);
        assert_eq!(component_stock(&one, SINGLE_KAMA, "Kama"), 1);
        let missing = missing_components(&one, &catalog, dual_kamas);
        let kamas: Vec<&MissingComponent> = missing
            .iter()
            .filter(|missing| missing.unique_name == SINGLE_KAMA)
            .collect();
        assert_eq!(kamas.len(), 1);
        assert_eq!(kamas[0].owned, 0);
        assert_eq!(kamas[0].required, 1);

        let two =
            fixtures::inventory_stocked(&[("Melee", SINGLE_KAMA), ("Melee", SINGLE_KAMA)], &[]);
        assert_eq!(component_stock(&two, SINGLE_KAMA, "Kama"), 2);
        assert!(
            missing_components(&two, &catalog, dual_kamas)
                .iter()
                .all(|missing| missing.unique_name != SINGLE_KAMA)
        );
    }

    #[test]
    fn lens_tree_depth() {
        let catalog = fixtures::foundry_catalog();
        let inventory = fixtures::inventory_stocked(&[], &[]);
        let tree = craft_tree(&inventory, &catalog, recipe(&catalog, LUA_LENS));

        let eidolon = child(&tree, EIDOLON_LENS);
        let greater = child(&eidolon.children, GREATER_LENS);
        let forma = child(&greater.children, FORMA_ITEM);
        let morphics = child(&forma.children, MORPHICS);
        assert!(morphics.children.is_empty());
        assert!(
            forma.children.iter().all(|node| node.children.is_empty()),
            "Morphics, Neural Sensors, Neurodes and Orokin Cell are farmed, not crafted"
        );
        assert_eq!(node_depth(eidolon), 4);
        assert_eq!(tree.iter().map(node_depth).max().unwrap(), 4);
    }

    #[test]
    fn researched_control_module_expands() {
        let catalog = fixtures::foundry_catalog();
        let bare = fixtures::inventory_stocked(&[], &[]);
        let bare_tree = craft_tree(&bare, &catalog, recipe(&catalog, DETONITE_INJECTOR));
        let leaf = child(&bare_tree, CONTROL_MODULE);
        assert!(leaf.children.is_empty());

        let researched = fixtures::inventory_stocked(
            &[],
            &[(
                "Recipes",
                "/Lotus/Types/Recipes/Components/ControlModuleResourceBlueprint",
                1,
            )],
        );
        let researched_tree =
            craft_tree(&researched, &catalog, recipe(&catalog, DETONITE_INJECTOR));
        let expanded = child(&researched_tree, CONTROL_MODULE);
        assert_eq!(expanded.children.len(), 5);
        assert_eq!(
            child(
                &expanded.children,
                "/Lotus/Types/Items/MiscItems/AlloyPlate"
            )
            .required,
            10_000
        );
        assert!(
            child(&expanded.children, "/Lotus/Types/Items/MiscItems/Salvage")
                .children
                .is_empty()
        );
    }

    #[test]
    fn shared_neural_sensor_stock() {
        let catalog = fixtures::foundry_catalog();
        let inventory = fixtures::inventory_stocked(&[], &[("MiscItems", NEURAL_SENSOR, 5)]);
        let tree = craft_tree(&inventory, &catalog, recipe(&catalog, DUAL_KAMAS));

        let kamas: Vec<&CraftNode> = tree
            .iter()
            .filter(|node| node.unique_name == SINGLE_KAMA)
            .collect();
        assert_eq!(kamas.len(), 2);

        let first = child(&kamas[0].children, NEURAL_SENSOR);
        assert_eq!(first.owned, 5);
        assert_eq!(first.required, 5);
        assert!(first.stocked);
        assert!(first.children.is_empty());

        let second = child(&kamas[1].children, NEURAL_SENSOR);
        assert_eq!(second.owned, 5);
        assert!(!second.stocked);
        assert_eq!(second.short_by, 5);
        assert!(!second.craftable);
    }

    #[test]
    fn stocked_forma_node() {
        let catalog = fixtures::foundry_catalog();
        let inventory = fixtures::inventory_stocked(&[], &[("MiscItems", FORMA_ITEM, 1)]);
        let tree = craft_tree(&inventory, &catalog, recipe(&catalog, LUA_LENS));
        let greater = child(&child(&tree, EIDOLON_LENS).children, GREATER_LENS);
        let forma = child(&greater.children, FORMA_ITEM);
        assert_eq!(forma.owned, 1);
        assert!(forma.stocked);
        assert!(forma.covered);
        assert!(!forma.craftable);
        assert_eq!(forma.crafts_queued, 0);
        assert!(forma.children.iter().all(|child| child.covered));
        assert_eq!(forma.per_craft, 1);
    }

    #[test]
    fn summary_credits_and_time() {
        let catalog = fixtures::foundry_catalog();
        let empty = fixtures::inventory_stocked(&[], &[]);
        let bare = details(&empty, &catalog, DUAL_KAMAS).unwrap();
        assert_eq!(bare.summary.credits, 60_000);
        assert_eq!(bare.summary.build_secs, 129_600);
        assert_eq!(bare.summary.shortest_secs, 86_400);

        let both_kamas =
            fixtures::inventory_stocked(&[("Melee", SINGLE_KAMA), ("Melee", SINGLE_KAMA)], &[]);
        let stocked = details(&both_kamas, &catalog, DUAL_KAMAS).unwrap();
        assert_eq!(stocked.summary.credits, 20_000);
        assert_eq!(stocked.summary.build_secs, 43_200);
        assert_eq!(stocked.summary.shortest_secs, 43_200);
    }

    #[test]
    fn summary_shopping_list() {
        let catalog = fixtures::foundry_catalog();
        let empty = fixtures::inventory_stocked(&[], &[]);
        let bare = details(&empty, &catalog, DUAL_KAMAS).unwrap();
        let named = |list: &[NeededItem]| -> Vec<(String, i64)> {
            list.iter()
                .map(|needed| (needed.name.clone(), needed.amount))
                .collect()
        };
        assert!(
            bare.summary
                .blueprints_needed
                .iter()
                .all(|needed| needed.unique_name.contains("Blueprint"))
        );
        assert!(
            bare.summary
                .resources_needed
                .iter()
                .all(|needed| !needed.unique_name.contains("Blueprint"))
        );
        let resources = named(&bare.summary.resources_needed);
        assert!(
            resources.windows(2).all(|pair| pair[0].0 <= pair[1].0),
            "the shopping list reads alphabetically, got {resources:?}"
        );
        assert!(
            resources.iter().all(|(_, amount)| *amount > 0),
            "nothing already covered is listed"
        );
        assert_eq!(
            resources,
            [
                (String::from("Circuits"), 1800),
                (String::from("Ferrite"), 2400),
                (String::from("Neural Sensors"), 10),
                (String::from("Orokin Cell"), 1),
                (String::from("Polymer Bundle"), 2400),
            ],
            "both identical Kama slots expand and their leaves add up"
        );
        assert_eq!(
            named(&bare.summary.blueprints_needed),
            [
                (String::from("Dual Kamas Blueprint"), 1),
                (String::from("Kama Blueprint"), 2),
            ]
        );

        let both_kamas =
            fixtures::inventory_stocked(&[("Melee", SINGLE_KAMA), ("Melee", SINGLE_KAMA)], &[]);
        let stocked = details(&both_kamas, &catalog, DUAL_KAMAS).unwrap();
        assert!(
            !named(&stocked.summary.blueprints_needed)
                .iter()
                .any(|(name, _)| name == "Kama Blueprint"),
            "a covered slot takes its whole subtree out of the list"
        );
    }

    #[test]
    fn lua_lens_summary() {
        let catalog = fixtures::foundry_catalog();
        let inventory = fixtures::inventory_stocked(&[], &[]);
        let lens = details(&inventory, &catalog, LUA_LENS).unwrap();
        let morphics = child(
            &child(
                &child(&child(&lens.tree, EIDOLON_LENS).children, GREATER_LENS).children,
                FORMA_ITEM,
            )
            .children,
            MORPHICS,
        );
        assert!(morphics.children.is_empty());
        assert_eq!(lens.summary.credits, 25_000 + 25_000 + 25_000 + 35_000);
        assert_eq!(lens.summary.build_secs, 86_400 * 3 + 82_800);
        assert_eq!(lens.summary.shortest_secs, 86_400 * 3 + 82_800);
    }
}
