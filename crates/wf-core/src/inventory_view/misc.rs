use std::collections::BTreeMap;

use wf_data::{Item, catch_grade};
use wf_inventory::{EquipmentItem, Inventory};

use super::{MiscRow, catalogued_name, display_name};
use crate::catalog::{Catalog, RELIC_PREFIX};
use crate::prices::market_slug;
use crate::view::View;

fn is_catchable_fish(item_type: &str) -> bool {
    item_type.contains("/Items/Fish/") && !item_type.contains("Boot")
}

fn is_arcane_helmet(item_type: &str) -> bool {
    item_type.contains("/Lotus/Upgrades/Skins/") && item_type.contains("Helmet")
}

pub(super) fn is_misc(item_type: &str) -> bool {
    if item_type.starts_with(RELIC_PREFIX) {
        return false;
    }
    item_type == "/Lotus/Upgrades/Skins/Kubrows/Collars/PrimeKubrowCollarA"
        || is_oubliette_key(item_type)
        || item_type.starts_with("/Lotus/Types/Items/FusionTreasures")
        || item_type.starts_with("/Lotus/Types/Items/MiscItems/PhotoboothTile")
        || item_type.contains("/Fusers/LegendaryModFuser")
        || item_type.contains("/Resources/Mechs/")
        || is_catchable_fish(item_type)
        || is_arcane_helmet(item_type)
}

fn is_emote(item_type: &str) -> bool {
    item_type.starts_with("/Lotus/Types/Items/Emotes")
}

fn is_scene(item_type: &str) -> bool {
    item_type.starts_with("/Lotus/Types/Items/MiscItems/PhotoboothTile")
        || item_type.starts_with("/Lotus/Types/Game/ShipScenes/")
}

pub(super) fn is_landing_craft_part(item_type: &str) -> bool {
    item_type.contains("/Lotus/Types/Recipes/LandingCraftRecipes/")
}

fn is_oubliette_key(item_type: &str) -> bool {
    item_type == "/Lotus/Types/Keys/Nightwave/GlassmakerBossFightKey"
}

fn misc_source_item<'a>(catalog: &'a Catalog, unique_name: &str) -> Option<&'a Item> {
    if let Some(item) = catalog.item(unique_name) {
        return Some(item);
    }
    let (base, _) = catch_grade(unique_name)?;
    catalog.item(&base)
}

fn is_tradable_misc(catalog: &Catalog, unique_name: &str) -> bool {
    if catalogued_name(catalog, unique_name).is_none() {
        return false;
    }
    if unique_name.contains("ModFuser")
        || is_emote(unique_name)
        || is_oubliette_key(unique_name)
        || is_landing_craft_part(unique_name)
    {
        return true;
    }
    if let Some(item) = misc_source_item(catalog, unique_name) {
        if item.is_skin() {
            return item.name.contains("Arcane");
        }
        return item.tradable || is_scene(unique_name);
    }
    if let Some((_, component)) = catalog.component(unique_name) {
        return component.tradable;
    }
    is_scene(unique_name)
}

pub(super) fn is_spare_weapon_stock(item_type: &str) -> bool {
    if item_type.contains("/Lotus/Weapons/Syndicates/CephalonSuda/Pistols/CSDroidArray") {
        return false;
    }
    item_type.contains("/Prisma")
        || item_type.contains("/Lotus/Weapons/Syndicates/")
        || item_type.contains("/VoidTrader")
        || item_type.contains("/Lotus/Weapons/Corpus/LongGuns/CrpBFG/Vandal/VandalCrpBFG")
        || item_type
            .contains("/Lotus/Weapons/Tenno/Pistols/ConclaveLeverPistol/ConclaveLeverPistol")
}

fn spare_weapons(inventory: &Inventory) -> impl Iterator<Item = &EquipmentItem> {
    [
        inventory.long_guns.as_slice(),
        &inventory.pistols,
        &inventory.melee,
        &inventory.space_guns,
        &inventory.space_melee,
        &inventory.space_suits,
        &inventory.sentinel_weapons,
    ]
    .into_iter()
    .flat_map(<[EquipmentItem]>::iter)
}

pub(crate) fn misc(view: &View) -> Vec<MiscRow> {
    let mut rows = counted_rows(view);
    rows.extend(cosmetic_rows(view));
    rows.extend(pet_print_rows(view));
    rows.extend(spare_equipment_rows(view));
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

fn counted_rows(view: &View) -> Vec<MiscRow> {
    let View {
        inventory,
        catalog,
        prices,
        favourites,
        listings,
    } = *view;
    let mut counted: BTreeMap<&str, i64> = BTreeMap::new();
    for item in inventory
        .misc_items
        .iter()
        .chain(&inventory.fusion_treasures)
        .chain(&inventory.raw_upgrades)
        .chain(&inventory.level_keys)
    {
        if !is_misc(&item.item_type) || !is_tradable_misc(catalog, &item.item_type) {
            continue;
        }
        *counted.entry(item.item_type.as_str()).or_insert(0) += item.item_count;
    }
    counted
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(unique_name, count)| {
            let name = display_name(catalog, unique_name);
            let slug = market_slug(&name);
            MiscRow {
                image_name: catalog
                    .item(unique_name)
                    .and_then(|item| item.image_name.clone()),
                ducats: catalog
                    .component(unique_name)
                    .and_then(|(_, component)| component.ducats),
                plat: prices.plat(&slug),
                favourite: favourites.contains(unique_name),
                order_placed: listings.has_order(&slug),
                unique_name: unique_name.to_owned(),
                count,
                market_slug: slug,
                name,
            }
        })
        .collect()
}

fn cosmetic_rows(view: &View) -> Vec<MiscRow> {
    let View {
        inventory,
        catalog,
        favourites,
        ..
    } = *view;
    let mut counted: BTreeMap<&str, i64> = BTreeMap::new();
    for skin in &inventory.weapon_skins {
        if is_misc(&skin.item_type) && is_tradable_misc(catalog, &skin.item_type) {
            *counted.entry(skin.item_type.as_str()).or_insert(0) += 1;
        }
    }
    for flavour in &inventory.flavour_items {
        if is_tradable_misc(catalog, &flavour.item_type) {
            *counted.entry(flavour.item_type.as_str()).or_insert(0) += 1;
        }
    }
    counted
        .into_iter()
        .map(|(unique_name, count)| MiscRow {
            name: display_name(catalog, unique_name),
            image_name: catalog
                .item(unique_name)
                .and_then(|item| item.image_name.clone()),
            count,
            ducats: None,
            plat: None,
            market_slug: String::new(),
            favourite: favourites.contains(unique_name),
            order_placed: false,
            unique_name: unique_name.to_owned(),
        })
        .collect()
}

fn pet_print_rows(view: &View) -> Vec<MiscRow> {
    let View {
        inventory,
        catalog,
        prices,
        favourites,
        listings,
    } = *view;
    let mut counted: BTreeMap<&str, i64> = BTreeMap::new();
    for print in &inventory.kubrow_pet_prints {
        let personality = print.dominant_traits.personality.as_str();
        if catalog.item(personality).is_some() {
            *counted.entry(personality).or_insert(0) += 1;
        }
    }
    counted
        .into_iter()
        .map(|(unique_name, count)| {
            let name = format!("{} Imprint", display_name(catalog, unique_name));
            let slug = market_slug(&name);
            MiscRow {
                image_name: catalog
                    .item(unique_name)
                    .and_then(|item| item.image_name.clone()),
                count,
                ducats: None,
                plat: prices.plat(&slug),
                favourite: favourites.contains(unique_name),
                order_placed: listings.has_order(&slug),
                unique_name: unique_name.to_owned(),
                market_slug: slug,
                name,
            }
        })
        .collect()
}

fn spare_equipment_rows(view: &View) -> Vec<MiscRow> {
    let View {
        inventory,
        catalog,
        prices,
        favourites,
        listings,
    } = *view;
    let mut counted: BTreeMap<&str, i64> = BTreeMap::new();
    for owned in spare_weapons(inventory) {
        if owned.xp == 0
            && is_spare_weapon_stock(&owned.item_type)
            && catalog.item(&owned.item_type).is_some()
        {
            *counted.entry(owned.item_type.as_str()).or_insert(0) += 1;
        }
    }
    counted
        .into_iter()
        .map(|(unique_name, count)| {
            let name = display_name(catalog, unique_name);
            let slug = market_slug(&name);
            MiscRow {
                image_name: catalog
                    .item(unique_name)
                    .and_then(|item| item.image_name.clone()),
                count,
                ducats: None,
                plat: prices.plat(&slug),
                favourite: favourites.contains(unique_name),
                order_placed: listings.has_order(&slug),
                unique_name: unique_name.to_owned(),
                market_slug: slug,
                name,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::tests::{no_listings, prices};
    use super::*;
    use crate::catalog::display_name_from_path;
    use crate::catalog::fixtures;
    use crate::favourites::Favourites;
    use crate::prices::FixedPrices;

    const MISC_ITEMS: &str = r#"[
      {"uniqueName":"/Lotus/Weapons/Corpus/Pistols/CrpHandRL/PrismaAngstrum",
       "name":"Prisma Angstrum","category":"Secondary","type":"Pistols","tradable":true,
       "imageName":"prisma-angstrum.png"},
      {"uniqueName":"/Lotus/Weapons/Syndicates/SteelMeridian/LongGuns/SMHek",
       "name":"Vaykor Hek","category":"Primary","type":"Shotgun","tradable":true,
       "imageName":"vaykor-hek.png"},
      {"uniqueName":"/Lotus/Types/Game/CatbrowPet/VampireCatbrowPetPowerSuit",
       "name":"Vasca Kavat","category":"Pets","type":"Pets","tradable":false,
       "imageName":"vasca-kavat.png"},
      {"uniqueName":"/Lotus/Types/Friendly/Pets/CreaturePets/ArmoredInfestedCatbrowPetPowerSuit",
       "name":"Panzer Vulpaphyla","category":"Pets","type":"Pets","tradable":false,
       "imageName":"panzer-vulpaphyla.png"},
      {"uniqueName":"/Lotus/Types/Friendly/Pets/CreaturePets/MedjayPredatorKubrowPetPowerSuit",
       "name":"Medjay Predasite","category":"Pets","type":"Pets","tradable":false,
       "imageName":"medjay-predasite.png"}
    ]"#;

    const GATE_ITEMS: &str = r#"[
      {"uniqueName":"/Lotus/Upgrades/Skins/Rhino/RhinoHelmetAltB",
       "name":"Arcane Vanguard Helmet","category":"Skins","type":"Skin","tradable":false,
       "imageName":"RhinoSeries3Helmet.png"},
      {"uniqueName":"/Lotus/Upgrades/Skins/Rhino/RhinoHelmetAltBStatless",
       "name":"Rhino Vanguard Helmet","category":"Skins","type":"Skin","tradable":false,
       "imageName":"RhinoSeries3Helmet.png"},
      {"uniqueName":"/Lotus/Types/StoreItems/AvatarImages/AvatarImageItem1",
       "name":"Excalibur Glyph","category":"Glyphs","type":"Glyph","tradable":false,
       "imageName":"Icon01.png"},
      {"uniqueName":"/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItem",
       "name":"Mortus Lungfish","category":"Fish","type":"Fish","tradable":true,
       "imageName":"FishItemDayUncommonB.png"},
      {"uniqueName":"/Lotus/Types/Items/Fish/Eidolon/FishParts/DayUncommonFishBPartItem",
       "name":"Mortus Horn","category":"Misc","type":"Fish Part","tradable":false,
       "imageName":"FishResourceHorn.png"},
      {"uniqueName":"/Lotus/Types/Items/Emotes/ShawzinEmote",
       "name":"Shawzin","category":"Skins","type":"Emotes","tradable":false,
       "imageName":"ShawzinEmote.png"},
      {"uniqueName":"/Lotus/Types/Items/MiscItems/PhotoboothTileDrifterCamp",
       "name":"The Drifter Camp Scene","category":"Misc","type":"Captura","tradable":true,
       "imageName":"DrifterCamp.png"}
    ]"#;

    const GAMMACOR: &str = "/Lotus/Weapons/Syndicates/CephalonSuda/Pistols/CSDroidArray";

    const OPTICOR_VANDAL: &str = "/Lotus/Weapons/Corpus/LongGuns/CrpBFG/Vandal/VandalCrpBFG";

    const ZYLOK: &str = "/Lotus/Weapons/Tenno/Pistols/ConclaveLeverPistol/ConclaveLeverPistol";

    const SPARE_WEAPONS: &str = r#"[
      {"uniqueName":"/Lotus/Weapons/Corpus/LongGuns/CrpBFG/Vandal/VandalCrpBFG",
       "name":"Opticor Vandal","category":"Primary","type":"Rifle","tradable":true,
       "imageName":"OpticorVandal.png"},
      {"uniqueName":"/Lotus/Weapons/ClanTech/Chemical/FlameThrowerWraith",
       "name":"Ignis Wraith","category":"Primary","type":"Rifle","tradable":true,
       "imageName":"IgnisWraith.png"},
      {"uniqueName":"/Lotus/Weapons/Tenno/Pistols/ConclaveLeverPistol/ConclaveLeverPistol",
       "name":"Zylok","category":"Secondary","type":"Pistol","tradable":false,
       "imageName":"ConclaveLeverPistol.png"},
      {"uniqueName":"/Lotus/Weapons/Syndicates/CephalonSuda/Pistols/CSDroidArray",
       "name":"Gammacor","category":"Secondary","type":"Pistol","tradable":false,
       "imageName":"Gammacor.png"},
      {"uniqueName":"/Lotus/Types/Sentinels/SentinelPowersuits/PrismaShadePowerSuit",
       "name":"Prisma Shade","category":"Sentinels","type":"Sentinel","tradable":true,
       "imageName":"PrismaShade.png"}
    ]"#;

    #[test]
    fn tradable_oddments() {
        let inventory = fixtures::inventory();
        let catalog = misc_catalog();
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|row| row.count > 0));
        assert!(
            !rows
                .iter()
                .any(|row| row.unique_name.starts_with(RELIC_PREFIX))
        );
        assert!(
            !rows
                .iter()
                .any(|row| row.unique_name.ends_with("MiscItems/Ferrite")),
            "plain resources belong to the resources tab, not misc"
        );
        assert!(
            rows.iter().any(|row| row
                .unique_name
                .starts_with("/Lotus/Types/Items/FusionTreasures")),
            "ayatan sculptures are misc"
        );
        let core = rows
            .iter()
            .find(|row| row.unique_name == "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser")
            .expect("Legendary Core");
        assert_eq!(core.name, "Legendary Core");
    }

    #[test]
    fn cosmetics_unpriced() {
        let inventory = fixtures::inventory();
        let catalog = misc_catalog();
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });

        let skins: HashSet<&str> = inventory
            .weapon_skins
            .iter()
            .map(|skin| skin.item_type.as_str())
            .filter(|item_type| is_misc(item_type))
            .collect();
        let flavour: HashSet<&str> = inventory
            .flavour_items
            .iter()
            .map(|flavour| flavour.item_type.as_str())
            .collect();
        assert_eq!(skins.len(), 5);
        assert_eq!(flavour.len(), 14);

        let cosmetics: Vec<&MiscRow> = rows
            .iter()
            .filter(|row| {
                skins.contains(row.unique_name.as_str())
                    || flavour.contains(row.unique_name.as_str())
            })
            .collect();
        assert_eq!(cosmetics.len(), 8);
        assert!(
            cosmetics
                .iter()
                .all(|row| row.plat.is_none() && row.market_slug.is_empty()),
            "cosmetics are not traded on warframe.market"
        );

        let listed = |unique_name: &str| rows.iter().any(|row| row.unique_name == unique_name);
        assert!(
            !listed("/Lotus/Upgrades/Skins/Excalibur/ExcaliburHelmet"),
            "an ordinary helmet skin cannot be traded"
        );
        assert!(
            !listed("/Lotus/Types/StoreItems/AvatarImages/AvatarImageItem1"),
            "glyphs cannot be traded"
        );
        assert!(
            !listed("/Lotus/Upgrades/Skins/Kubrows/Collars/PrimeKubrowCollarA"),
            "the collar is a skin without Arcane in its name"
        );
        assert!(
            !listed("/Lotus/Upgrades/Skins/Armor/ArbiterOfHexisArmor/ArbiterOfHexisArmorA"),
            "armour pieces are not part of the misc selection"
        );
    }

    #[test]
    fn cosmetic_names_from_export() {
        let inventory = fixtures::inventory();
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &misc_catalog(),
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let row = |unique_name: &str| {
            rows.iter()
                .find(|row| row.unique_name == unique_name)
                .cloned()
                .unwrap()
        };

        let emote = row("/Lotus/Types/Items/Emotes/ShawzinEmote");
        assert_eq!(emote.name, "Shawzin");
        assert_eq!(emote.image_name.as_deref(), Some("ShawzinEmote.png"));
        assert!(emote.plat.is_none());
        assert!(emote.market_slug.is_empty());

        let scene = row("/Lotus/Types/Items/MiscItems/PhotoboothTileDrifterCamp");
        assert_eq!(scene.name, "The Drifter Camp Scene");
        assert!(scene.image_name.is_some());

        let owned_emotes: HashSet<&str> = inventory
            .flavour_items
            .iter()
            .map(|flavour| flavour.item_type.as_str())
            .filter(|item_type| is_emote(item_type))
            .collect();
        let listed_emotes = rows.iter().filter(|row| is_emote(&row.unique_name)).count();
        assert_eq!(owned_emotes.len(), 4);
        assert_eq!(
            listed_emotes,
            owned_emotes.len(),
            "an emote is tradable whatever the export says about it"
        );
    }

    #[test]
    fn tradable_gate() {
        let catalog = Catalog::from_json(GATE_ITEMS, fixtures::RELICS, "[]").unwrap();
        let tradable = |unique_name: &str| is_tradable_misc(&catalog, unique_name);

        assert!(tradable("/Lotus/Upgrades/Skins/Rhino/RhinoHelmetAltB"));
        assert!(!tradable(
            "/Lotus/Upgrades/Skins/Rhino/RhinoHelmetAltBStatless"
        ));
        assert!(!tradable(
            "/Lotus/Types/StoreItems/AvatarImages/AvatarImageItem1"
        ));
        assert!(tradable(
            "/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItem"
        ));
        assert!(tradable(
            "/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItemLarge"
        ));
        assert!(!tradable(
            "/Lotus/Types/Items/Fish/Eidolon/FishParts/DayUncommonFishBPartItem"
        ));
        assert!(tradable("/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser"));
        assert!(tradable(
            "/Lotus/Types/Keys/Nightwave/GlassmakerBossFightKey"
        ));
        assert!(tradable("/Lotus/Types/Items/Emotes/ShawzinEmote"));
        assert!(tradable("/Lotus/Types/Game/ShipScenes/CorpusShipScene"));
        assert!(tradable(
            "/Lotus/Types/Items/MiscItems/PhotoboothTileDrifterCamp"
        ));
        assert!(
            !tradable("/Lotus/Upgrades/Skins/Horse/HorseHelmetDrapery"),
            "a curated skin without Arcane in its name is not tradable"
        );
        assert!(
            !tradable("/Lotus/Types/Items/MiscItems/NothingTheExportsKnow"),
            "a candidate no data source can name is dropped"
        );
    }

    #[test]
    fn spare_weapon_stock() {
        assert!(is_spare_weapon_stock(
            "/Lotus/Weapons/Corpus/Pistols/CrpHandRL/PrismaAngstrum"
        ));
        assert!(is_spare_weapon_stock(
            "/Lotus/Weapons/Syndicates/SteelMeridian/LongGuns/SMHek"
        ));
        assert!(is_spare_weapon_stock("/Lotus/Weapons/VoidTrader/VTDetron"));
        assert!(is_spare_weapon_stock(OPTICOR_VANDAL));
        assert!(
            is_spare_weapon_stock(ZYLOK),
            "the conclave pistol trades even though the export calls it untradable"
        );
        assert!(
            !is_spare_weapon_stock(GAMMACOR),
            "the plain Gammacor shares the Cephalon Suda path with its syndicate twin"
        );
        assert!(
            !is_spare_weapon_stock("/Lotus/Weapons/ClanTech/Chemical/FlameThrowerWraith"),
            "a wraith weapon the export calls tradable is not a syndicate weapon"
        );
        assert!(!is_spare_weapon_stock("/Lotus/Weapons/Tenno/Rifle/Rifle"));
    }

    #[test]
    fn spare_weapons_from_slots() {
        let inventory = fixtures::inventory_stocked(
            &[
                ("LongGuns", OPTICOR_VANDAL),
                (
                    "LongGuns",
                    "/Lotus/Weapons/ClanTech/Chemical/FlameThrowerWraith",
                ),
                ("Pistols", ZYLOK),
                ("Pistols", GAMMACOR),
                (
                    "Sentinels",
                    "/Lotus/Types/Sentinels/SentinelPowersuits/PrismaShadePowerSuit",
                ),
            ],
            &[],
        );
        let catalog = Catalog::from_json(SPARE_WEAPONS, fixtures::RELICS, "[]").unwrap();
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let listed = |name: &str| rows.iter().any(|row| row.name == name);
        assert!(listed("Opticor Vandal"));
        assert!(listed("Zylok"));
        assert!(
            !listed("Ignis Wraith"),
            "a tradable wraith weapon is not a spare faction weapon"
        );
        assert!(!listed("Gammacor"));
        assert!(
            !listed("Prisma Shade"),
            "a sentinel is not one of the weapon slots the tab reads"
        );
    }

    #[test]
    fn faction_weapons_and_imprints() {
        let inventory = fixtures::inventory();
        let catalog = Catalog::from_json(MISC_ITEMS, fixtures::RELICS, "[]").unwrap();
        let prices = FixedPrices::new([("prisma_angstrum", 32.0), ("vasca_kavat_imprint", 15.0)]);
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices,
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });

        let angstrum: Vec<&MiscRow> = rows
            .iter()
            .filter(|row| row.name == "Prisma Angstrum")
            .collect();
        assert_eq!(angstrum.len(), 1);
        assert_eq!(
            angstrum[0].count, 1,
            "the account owns two, one of them ranked"
        );
        assert_eq!(angstrum[0].market_slug, "prisma_angstrum");
        assert_eq!(angstrum[0].plat, Some(32.0));
        assert!(
            !rows.iter().any(|row| row.name == "Vaykor Hek"),
            "a ranked weapon can no longer be traded"
        );

        let imprints: Vec<&MiscRow> = rows
            .iter()
            .filter(|row| row.name.ends_with(" Imprint"))
            .collect();
        assert_eq!(imprints.len(), 3);
        assert_eq!(imprints.iter().map(|row| row.count).sum::<i64>(), 6);
        let vasca = imprints
            .iter()
            .find(|row| row.name == "Vasca Kavat Imprint")
            .expect("Vasca Kavat");
        assert_eq!(vasca.count, 1);
        assert_eq!(vasca.market_slug, "vasca_kavat_imprint");
        assert_eq!(vasca.plat, Some(15.0));
        assert_eq!(vasca.image_name.as_deref(), Some("vasca-kavat.png"));
        assert!(
            imprints
                .iter()
                .any(|row| row.name == "Panzer Vulpaphyla Imprint")
        );
    }

    const MISC_ITEMS_EXPORT: &str = include_str!("../../../../fixtures/misc_items.json");

    fn misc_catalog() -> Catalog {
        Catalog::from_json(MISC_ITEMS_EXPORT, fixtures::RELICS, fixtures::COMPONENTS).unwrap()
    }

    fn misc_catalog_without(categories: &[&str]) -> Catalog {
        let entries: Vec<serde_json::Value> = serde_json::from_str(MISC_ITEMS_EXPORT).unwrap();
        let kept: Vec<serde_json::Value> = entries
            .into_iter()
            .filter(|entry| !categories.contains(&entry["category"].as_str().unwrap_or_default()))
            .collect();
        let json = serde_json::to_string(&kept).unwrap();
        Catalog::from_json(&json, fixtures::RELICS, fixtures::COMPONENTS).unwrap()
    }

    fn path_derived(catalog: &Catalog, row: &MiscRow) -> bool {
        catalog.item(&row.unique_name).is_none()
            && catalog.component(&row.unique_name).is_none()
            && row.name == display_name_from_path(&row.unique_name)
    }

    #[test]
    fn no_path_derived_names() {
        let inventory = fixtures::inventory();
        let catalog = misc_catalog();
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let unresolved: Vec<&str> = rows
            .iter()
            .filter(|row| path_derived(&catalog, row))
            .map(|row| row.unique_name.as_str())
            .collect();
        assert!(
            unresolved.is_empty(),
            "path-derived rows left: {unresolved:?}"
        );

        let without_catches = misc_catalog_without(&["Fish"]);
        let narrowed = misc(&View {
            inventory: &inventory,
            catalog: &without_catches,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert!(
            !narrowed
                .iter()
                .any(|row| is_catchable_fish(&row.unique_name)),
            "a catch no export names is dropped rather than listed by its path"
        );
        assert_eq!(rows.len() - narrowed.len(), 17);
    }

    #[test]
    fn fish_names() {
        let inventory = fixtures::inventory();
        let catalog = misc_catalog();
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let name = |unique_name: &str| {
            rows.iter()
                .find(|row| row.unique_name == unique_name)
                .map(|row| row.name.as_str())
        };
        assert_eq!(
            name("/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItem"),
            Some("Mortus Lungfish")
        );
        assert_eq!(
            name("/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItemLarge"),
            Some("Mortus Lungfish (Large)")
        );
        assert_eq!(
            name("/Lotus/Types/Items/Fish/Deimos/InfestedCommonDFishItemMedium"),
            Some("Amniophysi (Medium)")
        );
        assert_eq!(
            name("/Lotus/Types/Items/Fish/Deimos/HybridRareAFishItemLarge"),
            Some("Aquapulmo (Magnificent)")
        );
        assert_eq!(
            name("/Lotus/Types/Items/Fish/Solaris/OrokinCoolRareFishAMediumItem"),
            Some("Tromyzon (Adorned)")
        );
        assert_eq!(
            name("/Lotus/Types/Items/Fish/Duviri/DuviriFishAItem"),
            Some("Inaak")
        );
    }

    #[test]
    fn curated_rows_without_image() {
        let inventory = fixtures::inventory();
        let catalog = misc_catalog();
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let without_image: Vec<(&str, &str)> = rows
            .iter()
            .filter(|row| row.image_name.is_none())
            .map(|row| (row.unique_name.as_str(), row.name.as_str()))
            .collect();
        assert_eq!(
            without_image,
            [
                (
                    "/Lotus/Types/Game/ShipScenes/CorpusShipScene",
                    "Corpus Interior Decorations"
                ),
                (
                    "/Lotus/Types/Keys/Nightwave/GlassmakerBossFightKey",
                    "Enter Nihil's Oubliette"
                ),
                (
                    "/Lotus/Types/Game/ShipScenes/PrimeLisetFiligreeScene",
                    "Filigree Prime Decoration"
                ),
                (
                    "/Lotus/Types/Game/ShipScenes/HalloweenScene",
                    "Haunted Interior Decorations"
                ),
                (
                    "/Lotus/Types/Game/ShipScenes/NidusPrimeScene",
                    "Infested Orbiter Decorations"
                ),
                (
                    "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser",
                    "Legendary Core"
                ),
                (
                    "/Lotus/Types/Items/Emotes/GeminiEmote",
                    "Universal Gemini Emote"
                ),
            ]
        );
    }

    #[test]
    fn is_misc_categories() {
        assert!(is_misc(
            "/Lotus/Types/Items/FusionTreasures/Ayatan/AyatanAnasaSculpture"
        ));
        assert!(is_misc(
            "/Lotus/Types/Items/MiscItems/PhotoboothTileSomething"
        ));
        assert!(is_misc(
            "/Lotus/Upgrades/Skins/Kubrows/Collars/PrimeKubrowCollarA"
        ));
        assert!(is_misc("/Lotus/Types/Items/Fish/EidolonFishA"));
        assert!(!is_misc("/Lotus/Types/Items/Fish/BootItem"));
        assert!(!is_misc("/Lotus/Types/Items/MiscItems/Ferrite"));
        assert!(!is_misc(
            "/Lotus/Types/Game/Projections/T1VoidProjectionABronze"
        ));
    }

    fn misc_family(inventory: &Inventory, unique_name: &str) -> &'static str {
        if unique_name.starts_with("/Lotus/Types/Items/FusionTreasures") {
            return "sculptures";
        }
        if unique_name.contains("ModFuser") {
            return "fusers";
        }
        if is_oubliette_key(unique_name) {
            return "keys";
        }
        if is_scene(unique_name) {
            return "scenes";
        }
        if is_catchable_fish(unique_name) {
            return "fish";
        }
        if unique_name.contains("/Resources/Mechs/") {
            return "necramech resources";
        }
        if inventory
            .kubrow_pet_prints
            .iter()
            .any(|print| print.dominant_traits.personality == unique_name)
        {
            return "imprints";
        }
        if inventory
            .equipment()
            .any(|owned| owned.item_type == unique_name)
        {
            return "faction weapons";
        }
        if is_arcane_helmet(unique_name) {
            return "helmet skins";
        }
        if inventory
            .weapon_skins
            .iter()
            .any(|skin| skin.item_type == unique_name)
        {
            return "other skins";
        }
        "flavour items"
    }

    fn misc_families(inventory: &Inventory, rows: &[MiscRow]) -> BTreeMap<&'static str, usize> {
        let mut families: BTreeMap<&'static str, usize> = BTreeMap::new();
        for row in rows {
            *families
                .entry(misc_family(inventory, &row.unique_name))
                .or_insert(0) += 1;
        }
        families
    }

    #[test]
    fn fixture_misc_families() {
        let inventory = fixtures::inventory();
        let catalog = misc_catalog();
        let rows = misc(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        assert_eq!(rows.len(), 50);
        let expected: BTreeMap<&str, usize> = [
            ("faction weapons", 1),
            ("fish", 17),
            ("flavour items", 4),
            ("fusers", 1),
            ("imprints", 3),
            ("keys", 1),
            ("necramech resources", 5),
            ("scenes", 7),
            ("sculptures", 11),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            misc_families(&inventory, &rows),
            expected,
            "helmet skins and the flavour items outside the export stay out of the list"
        );
    }
}
