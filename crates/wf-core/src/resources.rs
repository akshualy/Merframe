use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wf_inventory::Inventory;

use crate::catalog::{Stock, part_identity};
use crate::foundry::{self, CraftNode, FoundryItem, is_blueprint};
use crate::view::View;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceSource {
    Held,
    Craftable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceScope {
    Mastery,
    All,
    Starred,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ResourceQuery {
    pub source: ResourceSource,
    pub scope: ResourceScope,
    pub kind: Option<String>,
    pub prime: Option<bool>,
    pub owned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceUse {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub wiki_url: Option<String>,
    pub amount: i64,
    pub favourite: bool,
    pub ready_to_build: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceRow {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub owned: i64,
    pub required: i64,
    pub deficit: i64,
    pub used_by: Vec<ResourceUse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourcesTab {
    pub resources: Vec<ResourceRow>,
    pub items: usize,
    pub credits: i64,
}

pub(crate) fn tab(
    view: &View<'_>,
    query: &ResourceQuery,
    include_founders: Option<bool>,
    now: DateTime<Utc>,
) -> ResourcesTab {
    let held = held_recipes(view.inventory);
    let stock = Stock::new(view.inventory);
    let items = foundry::items(
        view.inventory,
        view.catalog,
        None,
        include_founders,
        now,
        view.favourites,
    );
    let mut rows: BTreeMap<String, ResourceRow> = BTreeMap::new();
    let mut counted = 0;
    let mut credits = 0;
    for item in items.iter().filter(|item| selected(item, &held, query)) {
        let Some(details) =
            foundry::details(view.inventory, view.catalog, view.prices, &item.unique_name)
        else {
            continue;
        };
        let mut needed: BTreeMap<&str, (&CraftNode, i64)> = BTreeMap::new();
        for node in &details.tree {
            gather(node, 1, &mut needed);
        }
        for (node, amount) in needed.into_values() {
            let row = rows
                .entry(node.unique_name.clone())
                .or_insert_with(|| ResourceRow {
                    unique_name: node.unique_name.clone(),
                    name: node.name.clone(),
                    image_name: node.image_name.clone(),
                    owned: stock.count(&node.unique_name),
                    required: 0,
                    deficit: 0,
                    used_by: Vec::new(),
                });
            row.required += amount;
            row.used_by.push(ResourceUse {
                unique_name: item.unique_name.clone(),
                name: item.name.clone(),
                image_name: item.image_name.clone(),
                wiki_url: item.wiki_url.clone(),
                amount,
                favourite: item.favourite,
                ready_to_build: item.progress.ready_to_build,
            });
        }
        counted += 1;
        credits += details.summary.credits;
    }

    let mut resources: Vec<ResourceRow> = rows
        .into_values()
        .map(|mut row| {
            row.deficit = (row.required - row.owned).max(0);
            row.used_by.sort_by(|left, right| {
                right
                    .amount
                    .cmp(&left.amount)
                    .then_with(|| left.name.cmp(&right.name))
            });
            row
        })
        .collect();
    resources.sort_by(|left, right| left.name.cmp(&right.name));
    ResourcesTab {
        resources,
        items: counted,
        credits,
    }
}

fn gather<'a>(
    node: &'a CraftNode,
    runs: i64,
    needed: &mut BTreeMap<&'a str, (&'a CraftNode, i64)>,
) {
    if !node.children.is_empty() {
        if node.stocked {
            return;
        }
        let missing = (node.short_by + node.crafts_queued) * runs;
        let crafts = (missing + node.per_craft - 1) / node.per_craft;
        for child in &node.children {
            gather(child, crafts, needed);
        }
        return;
    }
    if is_blueprint(&node.unique_name) {
        return;
    }
    needed
        .entry(node.unique_name.as_str())
        .or_insert((node, 0))
        .1 += node.required * runs;
}

fn held_recipes(inventory: &Inventory) -> HashSet<&str> {
    inventory
        .recipes
        .iter()
        .filter(|recipe| recipe.item_count > 0)
        .map(|recipe| part_identity(&recipe.item_type))
        .collect()
}

fn selected(item: &FoundryItem, held: &HashSet<&str>, query: &ResourceQuery) -> bool {
    if item.progress.pending {
        return false;
    }
    let in_scope = match query.scope {
        ResourceScope::Mastery => !item.progress.owned && !item.mastered,
        ResourceScope::All => true,
        ResourceScope::Starred => item.favourite,
    };
    in_scope
        && query.kind.as_deref().is_none_or(|kind| item.kind == kind)
        && query
            .prime
            .is_none_or(|prime| item.prime.is_some() == prime)
        && query.owned.is_none_or(|owned| item.progress.owned == owned)
        && (query.source == ResourceSource::Craftable || holds_a_recipe(held, item))
}

fn holds_a_recipe(held: &HashSet<&str>, item: &FoundryItem) -> bool {
    held.contains(part_identity(&item.unique_name))
        || item
            .components
            .iter()
            .any(|component| held.contains(part_identity(&component.unique_name)))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{Catalog, fixtures};
    use crate::favourites::Favourites;
    use crate::listings::MarketListings;
    use crate::prices::FixedPrices;

    const NOW_MS: i64 = 1_788_807_069_000;
    const BRATON_PRIME: &str = "/Lotus/Weapons/Tenno/Rifle/BratonPrime";
    const BRATON_PRIME_BARREL: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel";
    const BRATON_PRIME_BLUEPRINT: &str = "/Lotus/Types/Recipes/Weapons/BratonPrimeBlueprint";
    const DUAL_KAMAS: &str = "/Lotus/Weapons/Tenno/Melee/DualKamas/DualKamas";
    const EXCALIBUR: &str = "/Lotus/Powersuits/Excalibur/Excalibur";
    const OROKIN_CELL: &str = "/Lotus/Types/Items/MiscItems/OrokinCell";

    fn resources(
        inventory: &Inventory,
        catalog: &Catalog,
        favourites: &Favourites,
        source: ResourceSource,
        scope: ResourceScope,
    ) -> ResourcesTab {
        let view = View {
            inventory,
            catalog,
            prices: &FixedPrices::default(),
            favourites,
            listings: &MarketListings::default(),
        };
        tab(
            &view,
            &ResourceQuery {
                source,
                scope,
                kind: None,
                prime: None,
                owned: None,
            },
            None,
            DateTime::from_timestamp_millis(NOW_MS).unwrap(),
        )
    }

    fn with_recipe(json: &str, recipe: &str, count: i64) -> String {
        json.replacen(
            r#""Recipes":[{"#,
            &format!(r#""Recipes":[{{"ItemType":"{recipe}","ItemCount":{count}}},{{"#),
            1,
        )
    }

    fn holding(recipe: &str, count: i64) -> Inventory {
        Inventory::parse(&with_recipe(fixtures::INVENTORY, recipe, count)).unwrap()
    }

    fn unbuilt_braton_prime() -> String {
        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        for key in ["LongGuns", "XPInfo"] {
            let entries = value
                .get_mut(key)
                .and_then(serde_json::Value::as_array_mut)
                .expect(key);
            entries.retain(|entry| {
                entry.get("ItemType").and_then(serde_json::Value::as_str) != Some(BRATON_PRIME)
            });
        }
        value.to_string()
    }

    fn names(tab: &ResourcesTab) -> Vec<&str> {
        tab.resources
            .iter()
            .flat_map(|row| &row.used_by)
            .map(|used| used.name.as_str())
            .collect::<BTreeSet<&str>>()
            .into_iter()
            .collect()
    }

    fn amounts<'a>(tab: &'a ResourcesTab, item: &str) -> Vec<(&'a str, i64)> {
        tab.resources
            .iter()
            .flat_map(|row| {
                row.used_by
                    .iter()
                    .filter(|used| used.unique_name == item)
                    .map(|used| (row.name.as_str(), used.amount))
            })
            .collect()
    }

    fn row<'a>(tab: &'a ResourcesTab, unique_name: &str) -> &'a ResourceRow {
        tab.resources
            .iter()
            .find(|row| row.unique_name == unique_name)
            .expect("resource row")
    }

    fn used_by(row: &ResourceRow) -> Vec<(&str, i64)> {
        row.used_by
            .iter()
            .map(|used| (used.name.as_str(), used.amount))
            .collect()
    }

    #[test]
    fn nothing_held_towards_mastery() {
        let tab = resources(
            &fixtures::inventory(),
            &fixtures::catalog(),
            &Favourites::default(),
            ResourceSource::Held,
            ResourceScope::Mastery,
        );
        assert!(tab.resources.is_empty());
        assert_eq!(tab.items, 0);
        assert_eq!(tab.credits, 0);
    }

    #[test]
    fn braton_prime_requirements() {
        let catalog = fixtures::catalog();
        let three = holding(BRATON_PRIME_BLUEPRINT, 3);
        let tab = resources(
            &three,
            &catalog,
            &Favourites::default(),
            ResourceSource::Held,
            ResourceScope::All,
        );

        assert_eq!(
            amounts(&tab, BRATON_PRIME),
            [("Orokin Cell", 10)],
            "the parts drop from relics and stay out of the resource list"
        );

        let cell = row(&tab, OROKIN_CELL);
        assert_eq!(cell.name, "Orokin Cell");
        assert_eq!(cell.image_name.as_deref(), Some("ComponentCell.png"));
        assert!(!cell.used_by[0].ready_to_build);
        assert_eq!(tab.items, 2);
        assert_eq!(cell.owned, 3319);
        assert_eq!(cell.required, 11);
        assert_eq!(cell.deficit, 0);
        assert_eq!(used_by(cell), [("Braton Prime", 10), ("Excalibur", 1)]);
        assert_eq!(tab.credits, 15_000 + 25_000);

        let stacked = resources(
            &holding(BRATON_PRIME_BLUEPRINT, 1000),
            &catalog,
            &Favourites::default(),
            ResourceSource::Held,
            ResourceScope::All,
        );
        assert_eq!(
            stacked, tab,
            "a stack of blueprints is still one item to build"
        );

        let with_skins = resources(
            &three,
            &fixtures::with_skins(fixtures::ITEMS),
            &Favourites::default(),
            ResourceSource::Held,
            ResourceScope::All,
        );
        assert_eq!(with_skins, tab);
    }

    #[test]
    fn deficit_without_stock() {
        let inventory = fixtures::inventory_stocked(&[], &[("Recipes", BRATON_PRIME_BLUEPRINT, 1)]);
        let tab = resources(
            &inventory,
            &fixtures::catalog(),
            &Favourites::default(),
            ResourceSource::Held,
            ResourceScope::All,
        );
        assert_eq!(names(&tab), ["Braton Prime"]);
        let cell = row(&tab, OROKIN_CELL);
        assert_eq!(cell.owned, 0);
        assert_eq!(cell.required, 10);
        assert_eq!(cell.deficit, 10);
    }

    #[test]
    fn scope_filters() {
        let inventory = Inventory::parse(&unbuilt_braton_prime()).unwrap();
        let catalog = fixtures::catalog();
        let scoped = |scope, favourites: &Favourites| {
            resources(
                &inventory,
                &catalog,
                favourites,
                ResourceSource::Craftable,
                scope,
            )
        };

        let mastery = scoped(ResourceScope::Mastery, &Favourites::default());
        assert_eq!(names(&mastery), ["Braton Prime"]);
        assert_eq!(mastery.credits, 15_000);

        let all = scoped(ResourceScope::All, &Favourites::default());
        assert_eq!(names(&all), ["Braton Prime", "Excalibur", "Trinity Prime"]);
        assert_eq!(row(&all, OROKIN_CELL).required, 12);

        let starred = scoped(
            ResourceScope::Starred,
            &[EXCALIBUR.to_owned()].into_iter().collect(),
        );
        assert_eq!(names(&starred), ["Excalibur"]);
        assert!(row(&starred, OROKIN_CELL).used_by[0].favourite);
        assert_eq!(row(&starred, OROKIN_CELL).required, 1);
    }

    #[test]
    fn kind_prime_and_owned_filters() {
        let inventory = Inventory::parse(&unbuilt_braton_prime()).unwrap();
        let catalog = fixtures::catalog();
        let view = View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &FixedPrices::default(),
            favourites: &Favourites::default(),
            listings: &MarketListings::default(),
        };
        let filtered = |kind: Option<&str>, prime, owned| {
            tab(
                &view,
                &ResourceQuery {
                    source: ResourceSource::Craftable,
                    scope: ResourceScope::All,
                    kind: kind.map(str::to_owned),
                    prime,
                    owned,
                },
                None,
                DateTime::from_timestamp_millis(NOW_MS).unwrap(),
            )
        };

        assert_eq!(
            names(&filtered(None, None, None)),
            ["Braton Prime", "Excalibur", "Trinity Prime"]
        );
        assert_eq!(
            names(&filtered(Some("warframe"), None, None)),
            ["Excalibur", "Trinity Prime"]
        );
        assert_eq!(
            names(&filtered(Some("primary"), None, None)),
            ["Braton Prime"]
        );
        assert_eq!(
            names(&filtered(None, Some(true), None)),
            ["Braton Prime", "Trinity Prime"]
        );
        assert_eq!(names(&filtered(None, Some(false), None)), ["Excalibur"]);
        assert_eq!(names(&filtered(None, None, Some(false))), ["Braton Prime"]);
        assert_eq!(
            names(&filtered(None, None, Some(true))),
            ["Excalibur", "Trinity Prime"]
        );
        assert!(names(&filtered(Some("warframe"), Some(true), Some(false))).is_empty());
    }

    #[test]
    fn source_filters() {
        let catalog = fixtures::catalog();
        let sourced = |inventory: &Inventory, source| {
            resources(
                inventory,
                &catalog,
                &Favourites::default(),
                source,
                ResourceScope::Mastery,
            )
        };

        let unbuilt = unbuilt_braton_prime();
        let bare = Inventory::parse(&unbuilt).unwrap();
        assert!(names(&sourced(&bare, ResourceSource::Held)).is_empty());
        assert_eq!(
            names(&sourced(&bare, ResourceSource::Craftable)),
            ["Braton Prime"]
        );

        let held = Inventory::parse(&with_recipe(&unbuilt, BRATON_PRIME_BLUEPRINT, 1)).unwrap();
        let tab = sourced(&held, ResourceSource::Held);
        assert_eq!(names(&tab), ["Braton Prime"]);
    }

    #[test]
    fn a_part_recipe_selects_the_item() {
        let tab = resources(
            &holding(BRATON_PRIME_BARREL, 2),
            &fixtures::catalog(),
            &Favourites::default(),
            ResourceSource::Held,
            ResourceScope::All,
        );
        assert_eq!(
            amounts(&tab, BRATON_PRIME),
            [("Orokin Cell", 10)],
            "a part recipe brings the item in without a blueprint of its own"
        );
    }

    #[test]
    fn pending_builds_are_left_out() {
        let json = fixtures::INVENTORY.replacen(
            r#""PendingRecipes":[{"#,
            &format!(
                r#""PendingRecipes":[{{"ItemType":"{BRATON_PRIME_BLUEPRINT}","CompletionDate":{{"$date":{{"$numberLong":"1789000000000"}}}},"ItemId":{{"$oid":"000000000000000000000001"}}}},{{"#
            ),
            1,
        );
        let inventory = Inventory::parse(&json).unwrap();
        let tab = resources(
            &inventory,
            &fixtures::catalog(),
            &Favourites::default(),
            ResourceSource::Craftable,
            ResourceScope::All,
        );
        assert_eq!(names(&tab), ["Excalibur", "Trinity Prime"]);
    }

    #[test]
    fn every_item_that_needs_a_resource_is_listed() {
        let tab = resources(
            &fixtures::inventory(),
            &fixtures::foundry_catalog(),
            &Favourites::default(),
            ResourceSource::Craftable,
            ResourceScope::All,
        );
        assert_eq!(
            names(&tab),
            ["Banshee Prime", "Boar", "Dual Kamas", "Kama", "Skana"]
        );
        assert_eq!(tab.items, 5);
        assert!(
            tab.resources
                .windows(2)
                .all(|pair| pair[0].name <= pair[1].name)
        );

        let cell = row(&tab, OROKIN_CELL);
        assert_eq!(used_by(cell), [("Banshee Prime", 5), ("Dual Kamas", 1)]);
        assert_eq!(cell.required, 6);

        let circuits = tab
            .resources
            .iter()
            .find(|row| row.name == "Circuits")
            .expect("Circuits");
        assert_eq!(circuits.owned, 0);
        assert_eq!(circuits.required, 2700);
        assert_eq!(circuits.deficit, 2700);
        assert_eq!(
            used_by(circuits),
            [("Dual Kamas", 1800), ("Kama", 900)],
            "both Kama slots of the Dual Kamas recipe add up"
        );
        assert!(
            tab.resources
                .iter()
                .all(|row| !is_blueprint(&row.unique_name))
        );
    }

    #[test]
    fn a_built_part_costs_nothing() {
        const SINGLE_KAMA: &str = "/Lotus/Weapons/Tenno/Melee/DualKamas/SingleKama";
        let inventory = fixtures::inventory_stocked(&[("Melee", SINGLE_KAMA)], &[]);
        let tab = resources(
            &inventory,
            &fixtures::foundry_catalog(),
            &Favourites::default(),
            ResourceSource::Craftable,
            ResourceScope::All,
        );
        assert!(amounts(&tab, DUAL_KAMAS).contains(&("Circuits", 900)));
    }

    #[test]
    fn the_whole_catalog_is_walked() {
        let tab = resources(
            &fixtures::inventory(),
            &fixtures::mastery_catalog(),
            &Favourites::default(),
            ResourceSource::Craftable,
            ResourceScope::All,
        );
        assert_eq!(names(&tab), ["Grimoire"]);
        assert_eq!(
            amounts(&tab, &tab.resources[0].used_by[0].unique_name),
            [
                ("Echo Voca", 15),
                ("Entrati Lanthorn", 10),
                ("Entrati Obols", 2000),
                ("Necracoil", 50),
            ]
        );
    }
}
