use std::collections::{BTreeMap, HashMap, HashSet};

use wf_data::{Component, Item};

use super::misc::is_landing_craft_part;
use super::{ItemStatus, PartRow, PartSet, SetComponent, SetRow};
use crate::catalog::{
    Catalog, Stock, component_image, is_prime, part_market_slug, part_name, vault_status,
};
use crate::prices::{Prices, set_slug};
use crate::view::View;

fn is_mastered(affinity: &HashMap<&str, u64>, item: &Item) -> bool {
    affinity
        .get(item.unique_name.as_str())
        .copied()
        .unwrap_or_default()
        >= item.affinity_cap()
}

fn is_warframe_part(item_type: &str) -> bool {
    item_type.contains("/WarframeRecipes/")
        && !(item_type.contains("Beacon") && item_type.contains("Component"))
}

fn is_weapon_part(item_type: &str) -> bool {
    if item_type.contains("/Weapons/Ostron")
        || item_type.contains("/Weapons/Corpus")
        || item_type.contains("TeamAmmoBlueprint")
        || item_type.contains("/Weapons/Skins/")
    {
        return false;
    }
    if item_type.contains("/WeaponParts/") {
        return true;
    }
    item_type.contains("Blueprint")
        && (item_type.contains("/SentinelRecipes/") || item_type.contains("/Weapons/"))
}

pub(super) fn is_part_stock(item_type: &str) -> bool {
    let assembly = is_warframe_part(item_type)
        || item_type.contains("/Lotus/Types/Recipes/ArchwingRecipes/")
        || is_landing_craft_part(item_type);
    if assembly && item_type.to_lowercase().contains("blueprint") {
        return true;
    }
    is_weapon_part(item_type) || item_type.contains("/Archwing/Primary/")
}

pub(crate) fn parts(view: &View) -> Vec<PartRow> {
    let View {
        inventory,
        catalog,
        prices,
        favourites,
        listings,
    } = *view;
    let owned_equipment = inventory.owned_item_types();
    let affinity = inventory.affinity_index();
    let stock = Stock::new(inventory);
    let building: HashSet<&str> = inventory
        .pending_recipes
        .iter()
        .map(|recipe| recipe.item_type.as_str())
        .collect();
    let mut counted: BTreeMap<&str, i64> = BTreeMap::new();
    for entry in inventory.misc_items.iter().chain(&inventory.recipes) {
        if is_part_stock(&entry.item_type) {
            *counted.entry(entry.item_type.as_str()).or_insert(0) += entry.item_count;
        }
    }
    let mut rows: Vec<PartRow> = counted
        .into_iter()
        .filter_map(|(unique_name, held)| {
            let (item, component) = catalog.component_for_stock(unique_name)?;
            if !component.tradable {
                return None;
            }
            let mut count = held;
            if building.contains(unique_name) {
                count -= 1;
            }
            if count <= 0 {
                return None;
            }
            let name = part_name(item, component);
            let slug = part_market_slug(item, component);
            Some(PartRow {
                count,
                prices: Prices {
                    sell: prices.plat(&slug),
                    buy: prices.buy_plat(&slug),
                    ducats: component.ducats,
                },
                set: PartSet {
                    name: item.name.clone(),
                    complete: set_is_complete(&stock, item),
                },
                vault: vault_status(&name, item.vaulted),
                item: ItemStatus {
                    built: owned_equipment.contains(item.unique_name.as_str()),
                    mastered: is_mastered(&affinity, item),
                },
                prime: is_prime(item),
                favourite: favourites.any([
                    unique_name,
                    component.unique_name.as_str(),
                    item.unique_name.as_str(),
                ]),
                order_placed: listings.has_order(&slug),
                unique_name: unique_name.to_owned(),
                image_name: component_image(item, component),
                market_slug: slug,
                name,
            })
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

fn set_parts(item: &Item) -> impl Iterator<Item = &Component> {
    item.components
        .iter()
        .flatten()
        .filter(|component| crate::catalog::is_part(component) && component.tradable)
}

fn set_is_complete(stock: &Stock<'_>, item: &Item) -> bool {
    let mut parts = set_parts(item).peekable();
    parts.peek().is_some()
        && parts
            .all(|component| stock.count(&component.unique_name) >= i64::from(component.item_count))
}

fn set_components(item: &Item, stock: &Stock<'_>) -> Vec<SetComponent> {
    set_parts(item)
        .map(|component| {
            let owned = stock.count(&component.unique_name);
            let required = i64::from(component.item_count);
            SetComponent {
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

fn set_ducats(catalog: &Catalog, components: &[SetComponent]) -> u32 {
    components
        .iter()
        .filter_map(|part| {
            let ducats = catalog
                .component(&part.unique_name)
                .and_then(|(_, component)| component.ducats)?;
            u32::try_from(part.required)
                .ok()
                .map(|required| ducats * required)
        })
        .sum()
}

pub(crate) fn sets(view: &View) -> Vec<SetRow> {
    let View {
        inventory,
        catalog,
        prices,
        favourites,
        listings,
    } = *view;
    let owned_equipment = inventory.owned_item_types();
    let affinity = inventory.affinity_index();
    let stock = Stock::new(inventory);
    let mut rows: Vec<SetRow> = catalog
        .items()
        .filter_map(|item| {
            let components = set_components(item, &stock);
            if components.len() <= 1 {
                return None;
            }
            let owned_parts = components.iter().filter(|part| part.enough).count();
            if owned_parts == 0 {
                return None;
            }
            let complete = owned_parts == components.len();
            let count = if complete {
                components
                    .iter()
                    .map(|part| part.owned / part.required.max(1))
                    .min()
                    .unwrap_or(0)
            } else {
                0
            };
            let slug = set_slug(&item.name);
            Some(SetRow {
                set_name: item.name.clone(),
                unique_name: item.unique_name.clone(),
                image_name: item.image_name.clone(),
                owned_parts,
                total_parts: components.len(),
                count,
                complete,
                item: ItemStatus {
                    built: owned_equipment.contains(item.unique_name.as_str()),
                    mastered: is_mastered(&affinity, item),
                },
                vault: vault_status(&item.name, item.vaulted),
                prices: Prices {
                    sell: prices.plat(&slug),
                    buy: prices.buy_plat(&slug),
                    ducats: Some(set_ducats(catalog, &components)),
                },
                order_placed: listings.has_order(&slug),
                market_slug: slug,
                favourite: favourites.contains(&item.unique_name),
                components,
            })
        })
        .collect();
    rows.sort_by(|a, b| a.set_name.cmp(&b.set_name));
    rows
}

#[cfg(test)]
mod tests {
    use super::super::tests::{no_listings, prices};
    use super::*;
    use crate::catalog::fixtures;
    use crate::favourites::Favourites;
    use crate::listings::MarketListings;

    #[test]
    fn owned_parts_only() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let rows = parts(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert!(
            rows.is_empty(),
            "the fixture account holds no tradable prime parts"
        );

        let stocked = fixtures::inventory_owning(&[
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                3,
            ),
            (
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint",
                1,
            ),
        ]);
        let rows = parts(&View {
            inventory: &stocked,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.count > 0));
        assert!(!rows.iter().any(|row| row.name.contains("Orokin Cell")));
        assert!(!rows.iter().any(|row| row.set.name == "Excalibur"));
        assert_eq!(rows[0].name, "Braton Prime Barrel");
        assert_eq!(rows[1].name, "Trinity Prime Systems");

        let barrel = &rows[0];
        assert_eq!(barrel.count, 3);
        assert_eq!(barrel.prices.sell, Some(8.0));
        assert_eq!(barrel.set.name, "Braton Prime");
        assert!(barrel.prime);
        assert!(barrel.image_name.is_some());
    }

    #[test]
    fn pending_part_not_stock() {
        let catalog = fixtures::catalog();
        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        value
            .get_mut("MiscItems")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap()
            .push(serde_json::json!({
                "ItemType": "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                "ItemCount": 1
            }));
        value
            .get_mut("PendingRecipes")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap()
            .push(serde_json::json!({
                "ItemType": "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                "CompletionDate": { "$date": { "$numberLong": "1700000000000" } },
                "ItemId": { "$oid": "000000000000000000000000" }
            }));
        let inventory = wf_inventory::Inventory::parse(&value.to_string()).unwrap();
        assert!(
            parts(&View {
                inventory: &inventory,
                catalog: &catalog,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &no_listings()
            })
            .is_empty()
        );
    }

    #[test]
    fn mastered_flag() {
        let catalog = fixtures::catalog();
        let held = [
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
        ];
        let stocked = fixtures::inventory_owning(&held);
        let rows = parts(&View {
            inventory: &stocked,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert!(
            rows.iter().all(|row| row.item.mastered),
            "the fixture account has Braton Prime and Trinity Prime at max rank"
        );
        let braton_set = sets(&View {
            inventory: &stocked,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        })
        .into_iter()
        .find(|row| row.set_name == "Braton Prime")
        .expect("Braton Prime");
        assert!(braton_set.item.mastered);

        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        for entry in value
            .get_mut("XPInfo")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap()
        {
            if entry["ItemType"] == "/Lotus/Weapons/Tenno/Rifle/BratonPrime" {
                entry["XP"] = serde_json::json!(1_000);
            }
        }
        let misc = value
            .get_mut("MiscItems")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap();
        for (item_type, count) in held {
            misc.push(serde_json::json!({ "ItemType": item_type, "ItemCount": count }));
        }
        let unmastered = wf_inventory::Inventory::parse(&value.to_string()).unwrap();
        let rows = parts(&View {
            inventory: &unmastered,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        for row in &rows {
            assert_eq!(
                row.item.mastered,
                row.set.name == "Trinity Prime",
                "{} tracks the rank of {}",
                row.name,
                row.set.name
            );
        }
        let braton_set = sets(&View {
            inventory: &unmastered,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        })
        .into_iter()
        .find(|row| row.set_name == "Braton Prime")
        .expect("Braton Prime");
        assert!(!braton_set.item.mastered);
        assert!(
            braton_set.item.built,
            "the rifle is still owned, just not maxed"
        );
    }

    #[test]
    fn partial_braton_set() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        assert!(
            sets(&View {
                inventory: &inventory,
                catalog: &catalog,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &no_listings()
            })
            .is_empty()
        );

        let stocked = fixtures::inventory_owning(&[
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                3,
            ),
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock",
                1,
            ),
        ]);
        let rows = sets(&View {
            inventory: &stocked,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert_eq!(rows.len(), 1);
        let braton = &rows[0];
        assert_eq!(braton.set_name, "Braton Prime");
        assert_eq!(braton.total_parts, 4);
        assert_eq!(braton.owned_parts, 2);
        assert!(!braton.complete);
        assert_eq!(braton.count, 0);
        assert_eq!(braton.prices.sell, Some(45.0));
        assert!(braton.item.built);
        assert_eq!(braton.components.len(), 4);
        assert_eq!(
            braton.components.iter().filter(|part| part.enough).count(),
            2
        );
        assert!(braton.image_name.is_some());

        let with_skins = fixtures::with_skins(fixtures::ITEMS);
        assert!(
            sets(&View {
                inventory: &inventory,
                catalog: &with_skins,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &no_listings()
            })
            .is_empty()
        );
        assert_eq!(
            sets(&View {
                inventory: &stocked,
                catalog: &with_skins,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &no_listings()
            }),
            rows
        );
    }

    #[test]
    fn set_ducats() {
        let catalog = fixtures::catalog();
        let stocked = fixtures::inventory_owning(&[
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                3,
            ),
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock",
                1,
            ),
        ]);
        let rows = sets(&View {
            inventory: &stocked,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert_eq!(rows[0].prices.ducats, Some(110));
    }

    #[test]
    fn is_part_stock_paths() {
        assert!(is_part_stock(
            "/Lotus/Types/Recipes/WarframeRecipes/MagChassisBlueprint"
        ));
        assert!(
            !is_part_stock("/Lotus/Types/Recipes/WarframeRecipes/IronframeChassisComponent"),
            "a built warframe part is not a blueprint"
        );
        assert!(!is_part_stock(
            "/Lotus/Types/Recipes/WarframeRecipes/ChromaBeaconCComponent"
        ));
        assert!(is_part_stock(
            "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel"
        ));
        assert!(is_part_stock(
            "/Lotus/Types/Recipes/LandingCraftRecipes/Mantys/MantysStarChartBlueprint"
        ));
        assert!(is_part_stock(
            "/Lotus/Types/Recipes/ArchwingRecipes/OdonataHarnessBlueprint"
        ));
        assert!(is_part_stock(
            "/Lotus/Types/Recipes/Weapons/Archwing/Primary/ArchGunBarrel"
        ));
        assert!(is_part_stock(
            "/Lotus/Types/Recipes/SentinelRecipes/HelperCarrierBlueprint"
        ));
        assert!(!is_part_stock(
            "/Lotus/Types/Recipes/Weapons/Ostron/OstronMeleeBlueprint"
        ));
        assert!(!is_part_stock(
            "/Lotus/Types/Recipes/Weapons/Corpus/CorpusRifleBlueprint"
        ));
        assert!(!is_part_stock(
            "/Lotus/Types/Recipes/Weapons/TeamAmmoBlueprintLarge"
        ));
        assert!(!is_part_stock(
            "/Lotus/Types/Recipes/Weapons/Skins/SkinBlueprint"
        ));
        assert!(!is_part_stock("/Lotus/Types/Items/MiscItems/Ferrite"));

        let candidates = fixtures::inventory();
        let listed = candidates
            .misc_items
            .iter()
            .chain(&candidates.recipes)
            .filter(|item| is_part_stock(&item.item_type))
            .count();
        assert_eq!(listed, 32);
    }

    #[test]
    fn built_and_mastered_flags() {
        let catalog = fixtures::catalog();
        let stocked = fixtures::inventory_owning(&[(
            "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
            3,
        )]);
        let row = parts(&View {
            inventory: &stocked,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        })
        .into_iter()
        .find(|row| row.name == "Braton Prime Barrel")
        .unwrap();
        assert!(row.item.built);
        assert!(row.item.mastered);
        assert!(row.item.built || row.item.mastered);
        assert!(row.vault.is_some(), "the part names a prime");

        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        for key in ["LongGuns", "XPInfo"] {
            let entries = value
                .get_mut(key)
                .and_then(serde_json::Value::as_array_mut)
                .unwrap();
            entries.retain(|entry| entry["ItemType"] != "/Lotus/Weapons/Tenno/Rifle/BratonPrime");
        }
        value
            .get_mut("MiscItems")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap()
            .push(serde_json::json!({
                "ItemType": "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                "ItemCount": 3
            }));
        let never_built = wf_inventory::Inventory::parse(&value.to_string()).unwrap();
        let row = parts(&View {
            inventory: &never_built,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        })
        .into_iter()
        .find(|row| row.name == "Braton Prime Barrel")
        .unwrap();
        assert!(!row.item.built);
        assert!(!row.item.mastered);
        assert!(!row.item.built && !row.item.mastered);
    }

    #[test]
    fn order_placed() {
        let inventory = fixtures::inventory_owning(&[
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                3,
            ),
            (
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint",
                1,
            ),
        ]);
        let catalog = fixtures::catalog();
        let listings = MarketListings::new(["braton_prime_barrel", "trinity_prime_systems"], &[]);
        let rows = parts(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &listings,
        });
        let ordered: Vec<&str> = rows
            .iter()
            .filter(|row| row.order_placed)
            .map(|row| row.market_slug.as_str())
            .collect();
        assert_eq!(
            ordered,
            ["braton_prime_barrel", "trinity_prime_systems_blueprint"],
            "an order on the part covers the row whose slug only differs by the blueprint suffix"
        );
        assert!(
            parts(&View {
                inventory: &inventory,
                catalog: &catalog,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &no_listings()
            })
            .iter()
            .all(|row| !row.order_placed),
            "without a market session no row claims an order"
        );
    }
}
