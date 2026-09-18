use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use wf_data::{Component, Item, base_warframe_name, helminth_ability, store_item_to_type};
use wf_inventory::Inventory;
use wf_worldstate::WorldState;

use crate::catalog::{Catalog, VaultStatus, component_image, is_prime, item_name, part_name};
use crate::favourites::Favourites;
use crate::mastery::{includes_founders, kind_of, masterable};

use super::stock::fill_slots;
use super::{
    FoundryComponent, FoundryItem, FoundryTab, Helminth, MasteryGate, PendingBuild, Prime,
    Progress, WorldTimer,
};

const CRAFTABLE_INTO_KINDS: [&str; 4] = ["primary", "secondary", "melee", "arch"];

pub(crate) fn tab(
    inventory: &Inventory,
    catalog: &Catalog,
    world: Option<&WorldState>,
    include_founders: Option<bool>,
    now: DateTime<Utc>,
    favourites: &Favourites,
) -> FoundryTab {
    FoundryTab {
        pending: pending(inventory, catalog, now),
        items: items(inventory, catalog, world, include_founders, now, favourites),
        timers: world.map(|state| timers(state, now)).unwrap_or_default(),
    }
}

pub(crate) fn pending(
    inventory: &Inventory,
    catalog: &Catalog,
    now: DateTime<Utc>,
) -> Vec<PendingBuild> {
    let mut builds: Vec<PendingBuild> = inventory
        .pending_recipes
        .iter()
        .map(|recipe| {
            let completes_at = recipe.completion_date.datetime();
            let remaining_secs = completes_at.signed_duration_since(now).num_seconds();
            PendingBuild {
                name: recipe_name(catalog, &recipe.item_type),
                image_name: recipe_image(catalog, &recipe.item_type),
                item_type: recipe.item_type.clone(),
                completes_at,
                remaining_secs,
                ready: remaining_secs <= 0,
            }
        })
        .collect();
    builds.sort_by_key(|build| build.completes_at);
    builds
}

pub(crate) fn items(
    inventory: &Inventory,
    catalog: &Catalog,
    world: Option<&WorldState>,
    include_founders: Option<bool>,
    now: DateTime<Utc>,
    favourites: &Favourites,
) -> Vec<FoundryItem> {
    let resurgence = prime_resurgence(world, now);
    let adapted = inventory.adapted_incarnons();
    let crafts_into = crafting_parents(catalog);
    let subsumed = subsumed_warframes(inventory, catalog);
    let shards = inventory.archon_shard_index();
    let owned_types = inventory.owned_item_types();
    let affinity = inventory.affinity_index();
    let pending: HashSet<&str> = inventory
        .pending_recipes
        .iter()
        .map(|recipe| recipe.item_type.as_str())
        .collect();
    let mut rows: Vec<FoundryItem> =
        masterable(catalog, includes_founders(inventory, include_founders))
            .map(|item| {
                let recipe = item.components.as_deref().unwrap_or_default();
                let components: Vec<FoundryComponent> = fill_slots(inventory, recipe)
                    .into_iter()
                    .zip(recipe)
                    .map(|(slot, component)| FoundryComponent {
                        favourite: favourites.contains(&component.unique_name),
                        unique_name: component.unique_name.clone(),
                        name: component_name(catalog, item, component),
                        image_name: component_image(item, component),
                        owned: slot.filled,
                        required: i64::from(component.item_count),
                        enough: slot.satisfied,
                    })
                    .collect();
                let pending_here = components
                    .iter()
                    .any(|component| pending.contains(component.unique_name.as_str()));
                let owned = owned_types.contains(item.unique_name.as_str()) || pending_here;
                let xp = affinity
                    .get(item.unique_name.as_str())
                    .copied()
                    .unwrap_or_default();
                let kind = kind_of(item);
                FoundryItem {
                    unique_name: item.unique_name.clone(),
                    name: item.name.clone(),
                    kind,
                    type_name: item.item_type.clone(),
                    image_name: item.image_name.clone(),
                    prime: is_prime(item).then(|| Prime {
                        vault: vault_status(item),
                        resurgence: resurgence.contains(item.unique_name.as_str()),
                    }),
                    mastered: xp >= item.affinity_cap(),
                    progress: Progress {
                        owned,
                        pending: pending_here,
                        ready_to_build: !components.is_empty()
                            && components.iter().all(|component| component.enough),
                    },
                    crafts_into: if CRAFTABLE_INTO_KINDS.contains(&kind) {
                        crafts_into
                            .get(item.unique_name.as_str())
                            .cloned()
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    },
                    mastery: MasteryGate {
                        required: item.mastery_req,
                        met: inventory.player_level >= item.mastery_req.unwrap_or_default(),
                    },
                    incarnon: adapted.contains(item.unique_name.as_str()),
                    helminth: helminth_ability(item).map(|ability| Helminth {
                        ability: ability.to_owned(),
                        subsumed: subsumed.contains(base_warframe_name(&item.name)),
                    }),
                    archon_shards: shards
                        .get(item.unique_name.as_str())
                        .copied()
                        .unwrap_or_default(),
                    favourite: favourites.contains(&item.unique_name),
                    components,
                }
            })
            .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

fn prime_resurgence(world: Option<&WorldState>, now: DateTime<Utc>) -> HashSet<String> {
    let Some(varzia) = world.and_then(|world| world.varzia(now)) else {
        return HashSet::new();
    };
    varzia
        .items
        .iter()
        .map(|offer| store_item_to_type(&offer.item_type))
        .collect()
}

fn subsumed_warframes<'a>(inventory: &Inventory, catalog: &'a Catalog) -> HashSet<&'a str> {
    inventory
        .subsumed_suits()
        .into_iter()
        .filter_map(|item_type| catalog.item(item_type))
        .map(|item| base_warframe_name(&item.name))
        .collect()
}

fn vault_status(item: &Item) -> VaultStatus {
    if ["Excalibur Prime", "Lato Prime", "Skana Prime"].contains(&item.name.as_str()) {
        return VaultStatus::Vaulted;
    }
    VaultStatus::from(item.vaulted)
}

fn crafting_parents(catalog: &Catalog) -> HashMap<&str, Vec<String>> {
    let mut parents: HashMap<&str, Vec<String>> = HashMap::new();
    for item in catalog.items() {
        for component in item.components.iter().flatten() {
            let names = parents.entry(component.unique_name.as_str()).or_default();
            if !names.contains(&item.name) {
                names.push(item.name.clone());
            }
        }
    }
    for names in parents.values_mut() {
        names.sort_unstable();
    }
    parents
}

fn is_farmed_stock(unique_name: &str, name: &str) -> bool {
    ["/Types/Items/", "/Resources/", "/Resource/"]
        .iter()
        .any(|marker| unique_name.contains(marker))
        || name == "Orokin Cell"
        || name.contains("Kavasa Prime")
}

pub(super) fn component_name(catalog: &Catalog, parent: &Item, component: &Component) -> String {
    if component.name == "Forma" {
        return "Forma Blueprint".to_owned();
    }
    if is_farmed_stock(&component.unique_name, &component.name) {
        return component.name.clone();
    }
    if catalog
        .item(&component.unique_name)
        .is_some_and(|item| CRAFTABLE_INTO_KINDS.contains(&kind_of(item)))
    {
        return component.name.clone();
    }
    part_name(parent, component)
}

pub(crate) fn timers(world: &WorldState, now: DateTime<Utc>) -> Vec<WorldTimer> {
    world
        .timers(now)
        .into_iter()
        .map(|timer| WorldTimer {
            name: timer.name.to_owned(),
            state: timer.state.to_owned(),
            ends_at: timer.ends,
            remaining_secs: timer.ends.signed_duration_since(now).num_seconds(),
        })
        .collect()
}

fn recipe_image(catalog: &Catalog, unique_name: &str) -> Option<String> {
    if let Some((item, component)) = catalog.component(unique_name) {
        return component_image(item, component);
    }
    catalog
        .item(unique_name)
        .and_then(|item| item.image_name.clone())
}

fn recipe_name(catalog: &Catalog, unique_name: &str) -> String {
    match catalog.component(unique_name) {
        Some((item, component)) => part_name(item, component),
        None => item_name(catalog, unique_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::foundry::details;

    const DUAL_KAMAS: &str = "/Lotus/Weapons/Tenno/Melee/DualKamas/DualKamas";
    const SINGLE_KAMA: &str = "/Lotus/Weapons/Tenno/Melee/DualKamas/SingleKama";

    fn at(millis: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(millis).unwrap()
    }

    fn without_excalibur() -> serde_json::Value {
        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        if let Some(entries) = value
            .get_mut("Suits")
            .and_then(serde_json::Value::as_array_mut)
        {
            entries.retain(|entry| {
                entry.get("ItemType").and_then(serde_json::Value::as_str)
                    != Some("/Lotus/Powersuits/Excalibur/Excalibur")
            });
        }
        value
    }

    #[test]
    fn pending_builds() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let builds = pending(&inventory, &catalog, at(1_787_000_000_000));
        assert_eq!(builds.len(), 4);
        assert_eq!(builds[0].name, "Bio Component Blueprint");
        assert_eq!(builds[0].remaining_secs, 0);
        assert!(builds[0].ready);
        assert_eq!(builds[1].remaining_secs, 6);
        assert!(!builds[1].ready);

        let forma = builds
            .iter()
            .find(|build| build.item_type.ends_with("FormaBlueprint"))
            .unwrap();
        assert_eq!(forma.completes_at, at(1_793_557_501_000));
        assert!(!forma.ready);
    }

    #[test]
    fn favourite_flags() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let now = at(1_788_807_069_000);
        let plain = items(
            &inventory,
            &catalog,
            None,
            None,
            now,
            &Favourites::default(),
        );
        let braton = plain
            .iter()
            .find(|row| row.name == "Braton Prime")
            .expect("Braton Prime");
        assert!(!braton.favourite);
        assert!(
            braton
                .components
                .iter()
                .all(|component| !component.favourite)
        );

        let barrel = braton.components[0].unique_name.clone();
        let starred: Favourites = [braton.unique_name.clone(), barrel.clone()]
            .into_iter()
            .collect();
        let rows = items(&inventory, &catalog, None, None, now, &starred);
        let braton = rows
            .iter()
            .find(|row| row.name == "Braton Prime")
            .expect("Braton Prime");
        assert!(braton.favourite);
        assert_eq!(
            braton
                .components
                .iter()
                .filter(|component| component.favourite)
                .map(|component| component.unique_name.as_str())
                .collect::<Vec<_>>(),
            vec![barrel.as_str()]
        );
        assert!(
            rows.iter()
                .filter(|row| row.name != "Braton Prime")
                .all(|row| !row.favourite)
        );
    }

    #[test]
    fn whole_catalog_listed() {
        let inventory =
            fixtures::inventory_without_equipment("/Lotus/Powersuits/Excalibur/Excalibur");
        let catalog = fixtures::catalog();
        let rows = items(
            &inventory,
            &catalog,
            None,
            None,
            at(1_788_807_069_000),
            &Favourites::default(),
        );
        let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, vec!["Braton Prime", "Excalibur", "Trinity Prime"]);

        let excalibur = rows.iter().find(|row| row.name == "Excalibur").unwrap();
        assert_eq!(excalibur.kind, "warframe");
        assert!(!excalibur.progress.owned);
        assert!(excalibur.prime.is_none());
        assert!(!excalibur.progress.ready_to_build);
        let blueprint = excalibur
            .components
            .iter()
            .find(|component| component.name == "Excalibur Blueprint")
            .unwrap();
        assert_eq!(blueprint.owned, 0);
        assert!(!blueprint.enough);
        let neuroptics = excalibur
            .components
            .iter()
            .find(|component| component.name == "Excalibur Neuroptics")
            .unwrap();
        assert_eq!(neuroptics.owned, 7);
        assert!(neuroptics.enough);

        let trinity = rows.iter().find(|row| row.name == "Trinity Prime").unwrap();
        assert!(trinity.progress.owned);
        assert!(trinity.mastered);
        assert!(trinity.prime.is_some());

        let tree = details(&inventory, &catalog, &excalibur.unique_name)
            .unwrap()
            .tree;
        let cell = tree
            .iter()
            .find(|node| node.name == "Orokin Cell")
            .expect("Orokin Cell");
        assert_eq!(cell.owned, 3319);
        assert!(cell.children.is_empty());
        assert!(details(&inventory, &catalog, "/Lotus/Nope").is_none());

        let with_skins = fixtures::with_skins(fixtures::ITEMS);
        assert_eq!(
            items(
                &inventory,
                &with_skins,
                None,
                None,
                at(1_788_807_069_000),
                &Favourites::default()
            ),
            rows
        );
    }

    #[test]
    fn helminth_and_archon_shards() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::mastery_catalog();
        let rows = items(
            &inventory,
            &catalog,
            None,
            None,
            at(1_788_807_069_000),
            &Favourites::default(),
        );
        let row = |name: &str| rows.iter().find(|row| row.name == name).unwrap();

        let excalibur = row("Excalibur");
        assert_eq!(
            excalibur.helminth,
            Some(Helminth {
                ability: "Radial Blind".to_owned(),
                subsumed: true,
            })
        );
        let umbra = row("Excalibur Umbra");
        assert_eq!(
            umbra.helminth,
            Some(Helminth {
                ability: "Radial Blind".to_owned(),
                subsumed: true,
            })
        );

        let styanax = row("Styanax Prime");
        assert_eq!(
            styanax.helminth,
            Some(Helminth {
                ability: "Tharros Strike".to_owned(),
                subsumed: false,
            })
        );

        assert_eq!(row("Equinox Prime").archon_shards, 5);
        assert_eq!(row("Gara Prime").archon_shards, 2);
        assert_eq!(row("Equinox").archon_shards, 0);

        let braton = row("Braton Prime");
        assert_eq!(braton.helminth, None);
        assert_eq!(braton.archon_shards, 0);
    }

    #[test]
    fn necramech_rows() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::mastery_catalog();
        let rows = items(
            &inventory,
            &catalog,
            None,
            None,
            at(1_788_807_069_000),
            &Favourites::default(),
        );
        let row = |name: &str| rows.iter().find(|row| row.name == name).unwrap();

        for name in ["Voidrig", "Bonewidow"] {
            let mech = row(name);
            assert_eq!(mech.kind, "necramech");
            assert_eq!(mech.helminth, None);
            assert_eq!(mech.archon_shards, 0);
        }

        assert_eq!(row("Excalibur").kind, "warframe");
        assert_eq!(row("Odonata").kind, "arch");
        assert!(
            !rows
                .iter()
                .filter(|row| row.kind == "warframe")
                .any(|row| row.name == "Voidrig")
        );
    }

    #[test]
    fn ready_to_build() {
        let inventory_json = serde_json::to_string(&without_excalibur()).unwrap().replacen(
            r#""Recipes":[{"#,
            r#""Recipes":[{"ItemType":"/Lotus/Types/Recipes/WarframeRecipes/ExcaliburBlueprint","ItemCount":1},{"ItemType":"/Lotus/Types/Recipes/WarframeRecipes/ExcaliburChassisComponent","ItemCount":1},{"ItemType":"/Lotus/Types/Recipes/WarframeRecipes/ExcaliburHelmetComponent","ItemCount":1},{"ItemType":"/Lotus/Types/Recipes/WarframeRecipes/ExcaliburSystemsComponent","ItemCount":1},{"#,
            1,
        );
        let inventory = wf_inventory::Inventory::parse(&inventory_json).unwrap();
        let catalog = fixtures::catalog();
        let rows = items(
            &inventory,
            &catalog,
            None,
            None,
            at(1_788_807_069_000),
            &Favourites::default(),
        );
        let excalibur = rows.iter().find(|row| row.name == "Excalibur").unwrap();
        assert!(excalibur.progress.ready_to_build);
        assert!(
            details(&inventory, &catalog, &excalibur.unique_name)
                .unwrap()
                .missing
                .is_empty()
        );
    }

    #[test]
    fn world_state_timers() {
        let world = world_state();
        let now = at(1_788_807_069_000);
        let timers = timers(&world, now);
        assert!(!timers.is_empty());
        let cetus = timers.iter().find(|timer| timer.name == "Cetus").unwrap();
        assert!(cetus.state == "day" || cetus.state == "night");
        assert_eq!(
            cetus.remaining_secs,
            cetus.ends_at.signed_duration_since(now).num_seconds()
        );
    }

    const BANSHEE_PRIME: &str = "/Lotus/Powersuits/Banshee/BansheePrime";
    const BOAR: &str = "/Lotus/Weapons/Tenno/Shotgun/FullAutoShotgun";
    const SKANA: &str = "/Lotus/Weapons/Tenno/Melee/LongSword/LongSword";
    const VARZIA_TRADING_MS: i64 = 1_788_807_069_000;

    fn world_state() -> WorldState {
        WorldState::parse(include_str!("../../../../fixtures/worldState.json"))
            .expect("world state")
    }

    fn inventory_at_rank(rank: u32) -> Inventory {
        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        value["PlayerLevel"] = serde_json::json!(rank);
        Inventory::parse(&value.to_string()).unwrap()
    }

    fn row<'a>(rows: &'a [FoundryItem], unique_name: &str) -> &'a FoundryItem {
        rows.iter()
            .find(|row| row.unique_name == unique_name)
            .unwrap()
    }

    #[test]
    fn mastery_requirement() {
        let catalog = fixtures::foundry_catalog();
        let veteran = items(
            &inventory_at_rank(36),
            &catalog,
            None,
            None,
            at(VARZIA_TRADING_MS),
            &Favourites::default(),
        );
        assert!(veteran.iter().all(|row| row.mastery.met));
        assert_eq!(row(&veteran, BOAR).mastery.required, Some(2));

        let fresh = items(
            &inventory_at_rank(1),
            &catalog,
            None,
            None,
            at(VARZIA_TRADING_MS),
            &Favourites::default(),
        );
        assert!(row(&fresh, SKANA).mastery.met);
        assert!(row(&fresh, DUAL_KAMAS).mastery.met);
        assert!(!row(&fresh, BOAR).mastery.met);
    }

    #[test]
    fn varzia_resurgence() {
        let catalog = fixtures::foundry_catalog();
        let inventory = fixtures::inventory();
        let world = world_state();

        let trading = items(
            &inventory,
            &catalog,
            Some(&world),
            None,
            at(VARZIA_TRADING_MS),
            &Favourites::default(),
        );
        assert!(
            row(&trading, BANSHEE_PRIME)
                .prime
                .is_some_and(|prime| prime.resurgence)
        );
        assert!(
            !row(&trading, DUAL_KAMAS)
                .prime
                .is_some_and(|prime| prime.resurgence)
        );

        let gone = items(
            &inventory,
            &catalog,
            Some(&world),
            None,
            at(1_790_877_600_000),
            &Favourites::default(),
        );
        assert!(
            !row(&gone, BANSHEE_PRIME)
                .prime
                .is_some_and(|prime| prime.resurgence)
        );

        let offline = items(
            &inventory,
            &catalog,
            None,
            None,
            at(VARZIA_TRADING_MS),
            &Favourites::default(),
        );
        assert!(
            !row(&offline, BANSHEE_PRIME)
                .prime
                .is_some_and(|prime| prime.resurgence)
        );
    }

    #[test]
    fn incarnon_adapted_copy() {
        let catalog = fixtures::foundry_catalog();
        let untouched = items(
            &fixtures::inventory(),
            &catalog,
            None,
            None,
            at(VARZIA_TRADING_MS),
            &Favourites::default(),
        );
        assert!(
            !row(&untouched, SKANA).incarnon,
            "the account owns no adapted Skana"
        );
        assert!(!row(&untouched, BOAR).incarnon);

        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        value
            .get_mut("Melee")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap()
            .push(serde_json::json!({
                "ItemType": SKANA,
                "ItemId": { "$oid": "00000000000000000000beef" },
                "XP": 0,
                "SkillTree": "0121",
            }));
        let adapted = wf_inventory::Inventory::parse(&value.to_string()).unwrap();
        let rows = items(
            &adapted,
            &catalog,
            None,
            None,
            at(VARZIA_TRADING_MS),
            &Favourites::default(),
        );
        assert!(row(&rows, SKANA).incarnon);
        assert!(!row(&rows, BOAR).incarnon);
        assert!(!row(&rows, DUAL_KAMAS).incarnon);
        assert!(!row(&rows, BANSHEE_PRIME).incarnon);
    }

    #[test]
    fn founders_items() {
        let catalog = fixtures::mastery_catalog();
        let founders = [
            "/Lotus/Powersuits/Excalibur/ExcaliburPrime",
            "/Lotus/Weapons/Tenno/Pistol/LatoPrime",
            "/Lotus/Weapons/Tenno/Melee/LongSword/SkanaPrime",
        ];
        let listed = |inventory: &Inventory, chosen: Option<bool>| -> usize {
            let rows = items(
                inventory,
                &catalog,
                None,
                chosen,
                at(VARZIA_TRADING_MS),
                &Favourites::default(),
            );
            rows.iter()
                .filter(|row| founders.contains(&row.unique_name.as_str()))
                .count()
        };

        let ordinary = fixtures::inventory();
        assert!(!ordinary.is_founder());
        assert_eq!(listed(&ordinary, None), 0);
        assert_eq!(listed(&ordinary, Some(true)), 3);

        let founder = Inventory::parse(&fixtures::INVENTORY.replacen(
            '{',
            r#"{"Accolades":{"Founder":4},"#,
            1,
        ))
        .unwrap();
        assert!(founder.is_founder());
        assert_eq!(listed(&founder, None), 3);
        assert_eq!(listed(&founder, Some(false)), 0);
    }

    #[test]
    fn single_kama_crafts_into_dual() {
        let catalog = fixtures::foundry_catalog();
        let rows = items(
            &fixtures::inventory(),
            &catalog,
            None,
            None,
            at(VARZIA_TRADING_MS),
            &Favourites::default(),
        );
        let kama = row(&rows, SINGLE_KAMA);
        assert_eq!(kama.crafts_into, ["Dual Kamas"]);
        assert!(row(&rows, DUAL_KAMAS).crafts_into.is_empty());
        assert!(row(&rows, BANSHEE_PRIME).crafts_into.is_empty());
    }

    #[test]
    fn prime_vault_status() {
        let catalog = fixtures::foundry_catalog();
        let rows = items(
            &fixtures::inventory(),
            &catalog,
            None,
            None,
            at(VARZIA_TRADING_MS),
            &Favourites::default(),
        );
        assert!(row(&rows, BANSHEE_PRIME).prime.is_some());
        assert_eq!(row(&rows, SKANA).prime, None);
    }
}
