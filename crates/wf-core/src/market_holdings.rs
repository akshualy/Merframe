use std::borrow::Cow;
use std::collections::BTreeSet;

use wf_data::Refinement;
use wf_inventory::{CountedItem, EquipmentItem, RivenFingerprint};

use crate::catalog::{RECIPE_PREFIX, display_name_from_path, is_fish};
use crate::identity::{ItemKind, ItemRecord, is_riven, traded_as};
use crate::inventory_view::{UpgradeKind, complete_sets, held_parts, spare_weapons, upgrade_kind};
use crate::view::View;

fn counted(items: &[CountedItem], unique_name: &str) -> i64 {
    items
        .iter()
        .filter(|item| item.item_type == unique_name)
        .map(|item| item.item_count)
        .sum()
}

fn entries<T>(items: impl IntoIterator<Item = T>, matches: impl Fn(&T) -> bool) -> i64 {
    items.into_iter().filter(matches).map(|_| 1).sum()
}

fn unveiled(name: &str) -> &str {
    name.strip_suffix(" (Veiled)").unwrap_or(name)
}

impl<'a> View<'a> {
    pub(crate) fn market_owned(
        &self,
        record: &ItemRecord,
        listed_name: &str,
        rank: Option<u32>,
        subtype: Option<&str>,
    ) -> i64 {
        let unique_name = record.unique_name.as_str();
        match record.kind {
            ItemKind::Upgrade { .. } | ItemKind::Riven => self.upgrades(record, listed_name, rank),
            ItemKind::Relic { .. } => subtype
                .and_then(|subtype| {
                    Refinement::ALL
                        .into_iter()
                        .find(|refinement| refinement.name().eq_ignore_ascii_case(subtype))
                })
                .map_or(0, |refinement| self.relics(unique_name, refinement)),
            ItemKind::Set { .. } => self.sets(unique_name),
            ItemKind::Imprint => entries(&self.account.inventory.kubrow_pet_prints, |print| {
                print.dominant_traits.personality == unique_name
            }),
            ItemKind::Part { .. } => self.blueprints(unique_name),
            ItemKind::Item | ItemKind::Other if unique_name.starts_with(RECIPE_PREFIX) => {
                self.blueprints(unique_name)
            }
            ItemKind::Item | ItemKind::Other => self.misc(record, listed_name),
        }
    }

    fn blueprints(&self, unique_name: &str) -> i64 {
        let traded = traded_as(unique_name);
        let blueprint = format!("{traded}Blueprint");
        [
            &self.account.inventory.misc_items,
            &self.account.inventory.recipes,
        ]
        .into_iter()
        .map(|items| counted(items, &traded) + counted(items, &blueprint))
        .sum()
    }

    fn relics(&self, intact: &str, refinement: Refinement) -> i64 {
        let relics = &self.account.inventory.misc_items;
        let base = intact
            .strip_suffix(Refinement::Intact.projection_suffix())
            .unwrap_or(intact);
        let refined = counted(relics, &format!("{base}{}", refinement.projection_suffix()));
        if refinement == Refinement::Intact {
            refined + counted(relics, base)
        } else {
            refined
        }
    }

    fn sets(&self, unique_name: &str) -> i64 {
        let Some(item) = self.catalog.item(unique_name) else {
            return 0;
        };
        let parts = held_parts(self.items, item, &self.account.stock);
        if parts.len() <= 1 {
            return 0;
        }
        complete_sets(&parts)
    }

    fn upgrade_name(&self, unique_name: &str) -> Cow<'a, str> {
        self.items.get(unique_name).map_or_else(
            || Cow::Owned(display_name_from_path(unique_name)),
            |record| Cow::Borrowed(record.name.as_str()),
        )
    }

    fn upgrades(&self, record: &ItemRecord, listed_name: &str, rank: Option<u32>) -> i64 {
        let listed = unveiled(listed_name);
        let matches = |unique_name: &str| {
            upgrade_kind(unique_name) != UpgradeKind::Neither
                && (unique_name == record.unique_name
                    || self.upgrade_name(unique_name).eq_ignore_ascii_case(listed))
        };
        let at_rank = |held: Option<u32>| rank.is_none_or(|rank| held.unwrap_or(0) == rank);
        let unranked: i64 = self
            .account
            .inventory
            .raw_upgrades
            .iter()
            .filter(|item| item.item_count > 0 && matches(&item.item_type))
            .filter(|item| at_rank(is_riven(&item.item_type).then_some(0)))
            .map(|item| item.item_count)
            .sum();
        let ranked = entries(&self.account.inventory.upgrades, |upgrade| {
            if !matches(&upgrade.item_type) {
                return false;
            }
            let fingerprint = upgrade.fingerprint();
            if is_riven(&upgrade.item_type) {
                return !fingerprint
                    .as_ref()
                    .is_some_and(RivenFingerprint::is_unveiled)
                    && at_rank(Some(0));
            }
            at_rank(fingerprint.map(|fingerprint| fingerprint.lvl))
        });
        unranked + ranked
    }

    fn misc(&self, record: &ItemRecord, listed_name: &str) -> i64 {
        let mut unique_names: BTreeSet<&str> = self
            .catalog
            .items()
            .chain(self.catalog.skins())
            .filter(|known| known.name.eq_ignore_ascii_case(listed_name))
            .map(|known| known.unique_name.as_str())
            .collect();
        if let Some(species) = unique_names
            .iter()
            .copied()
            .filter(|unique_name| is_fish(unique_name))
            .min_by_key(|unique_name| unique_name.len())
        {
            unique_names.retain(|unique_name| !is_fish(unique_name) || *unique_name == species);
        }
        if !record.unique_name.is_empty() {
            unique_names.insert(&record.unique_name);
        }
        unique_names
            .into_iter()
            .map(|unique_name| self.held(unique_name))
            .sum()
    }

    fn held(&self, unique_name: &str) -> i64 {
        let inventory = &self.account.inventory;
        if is_fish(unique_name) {
            let species = unique_name.strip_suffix("Item").unwrap_or(unique_name);
            return inventory
                .misc_items
                .iter()
                .filter(|item| item.item_type.contains(species))
                .map(|item| item.item_count)
                .sum();
        }
        let unranked = |owned: &&EquipmentItem| owned.item_type == unique_name && owned.xp == 0;
        let spare_equipment = entries(spare_weapons(inventory), unranked);
        let stacked: i64 = [
            &inventory.misc_items,
            &inventory.level_keys,
            &inventory.fusion_treasures,
            &inventory.raw_upgrades,
        ]
        .into_iter()
        .map(|items| counted(items, unique_name))
        .sum();
        stacked
            + spare_equipment
            + entries(&inventory.weapon_skins, |skin| {
                skin.item_type == unique_name
            })
            + entries(&inventory.flavour_items, |flavour| {
                flavour.item_type == unique_name
            })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::catalog::Catalog;
    use crate::catalog::fixtures::{self, BRATON_PRIME_BARREL, catalog_of, market_item};
    use crate::identity::market_name;
    use crate::view::Fixture;

    const PRIMED_CONTINUITY: &str =
        "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarAbilityDurationModExpert";
    const FEAR_SENSE: &str =
        "/Lotus/Types/Friendly/Pets/CatbrowPetPrecepts/CatbrowTremorSensePrecept";
    const CLASHING_FOREST: &str = "/Lotus/Weapons/Tenno/Melee/MeleeTrees/StaffCmbOneMeleeTree";
    const VEILED_RIFLE: &str = "/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare";
    const ENERGIZE: &str =
        "/Lotus/Upgrades/CosmeticEnhancers/Utility/GolemArcaneRadialEnergyOnEnergyPickup";

    fn holdings(
        catalog: Catalog,
        inventory: wf_inventory::Inventory,
        listings: &[wf_market::Item],
    ) -> impl Fn(&str, Option<u32>, Option<&str>) -> i64 + use<> {
        let mut fixture = Fixture::new(catalog, inventory);
        fixture.items.index_market(listings);
        let listings = listings.to_vec();
        move |id, rank, subtype| {
            let listing = listings.iter().find(|item| item.id == id).unwrap();
            fixture.view().market_owned(
                fixture.items.by_market_id(id).unwrap(),
                market_name(listing),
                rank,
                subtype,
            )
        }
    }

    #[test]
    fn part_counts_blueprint_stock() {
        let inventory = fixtures::inventory_stocked(
            &[],
            &[
                (
                    "Recipes",
                    "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrelBlueprint",
                    3,
                ),
                (
                    "MiscItems",
                    "/Lotus/Types/Recipes/Weapons/WeaponParts/TeshinGlaiveDisc",
                    63,
                ),
                (
                    "Recipes",
                    "/Lotus/Types/Recipes/WarframeRecipes/RhinoPrimeChassisBlueprint",
                    2,
                ),
                (
                    "MiscItems",
                    "/Lotus/Types/Recipes/Kubrow/Collars/PrimeKubrowCollarABandComponent",
                    1,
                ),
                (
                    "Recipes",
                    "/Lotus/Types/Recipes/Weapons/WeaponParts/AmbassadorBarrelBlueprint",
                    4,
                ),
                (
                    "Recipes",
                    "/Lotus/Types/Recipes/Weapons/WeaponParts/StrunWraithReceiverBlueprint",
                    3,
                ),
            ],
        );
        let owned = holdings(
            fixtures::catalog(),
            inventory,
            &[
                market_item(
                    "braton_prime_barrel",
                    "Braton Prime Barrel",
                    BRATON_PRIME_BARREL,
                    &["component"],
                ),
                market_item(
                    "orvius_disc",
                    "Orvius Disc",
                    "/Lotus/Types/Recipes/Weapons/WeaponParts/TeshinGlaiveDisc",
                    &["component"],
                ),
                market_item(
                    "rhino_prime_chassis_blueprint",
                    "Rhino Prime Chassis Blueprint",
                    "/Lotus/Types/Recipes/WarframeRecipes/RhinoPrimeChassisComponent",
                    &["component", "blueprint"],
                ),
                market_item(
                    "kavasa_prime_band",
                    "Kavasa Prime Band",
                    "/Lotus/Types/Recipes/Kubrow/Collars/PrimeKubrowCollarABandComponent",
                    &["prime"],
                ),
                market_item(
                    "ambassador_barrel",
                    "Ambassador Barrel",
                    "/Lotus/Types/Recipes/Weapons/WeaponParts/CrpArSniperBarrel",
                    &["component"],
                ),
                market_item(
                    "strun_wraith_receiver",
                    "Strun Wraith Receiver",
                    "/Lotus/Types/Recipes/Weapons/WeaponParts/StrunWraithReceiver",
                    &["component", "weapon"],
                ),
            ],
        );
        assert_eq!(owned("braton_prime_barrel", None, None), 3);
        assert_eq!(owned("orvius_disc", None, None), 63);
        assert_eq!(owned("rhino_prime_chassis_blueprint", None, None), 2);
        assert_eq!(owned("kavasa_prime_band", None, None), 1);
        assert_eq!(owned("ambassador_barrel", None, None), 4);
        assert_eq!(owned("strun_wraith_receiver", None, None), 3);
    }

    #[test]
    fn relic_counts_by_refinement() {
        let inventory = fixtures::inventory_owning(&[(
            "/Lotus/Types/Game/Projections/T1VoidProjectionSevagothPrimeDPlatinum",
            2,
        )]);
        let owned = holdings(
            fixtures::catalog(),
            inventory,
            &[
                market_item(
                    "lith_g12_relic",
                    "Lith G12 Relic",
                    "/Lotus/Types/Game/Projections/T1VoidProjectionSevagothPrimeD",
                    &["relic"],
                ),
                market_item(
                    "requiem_eterna_relic",
                    "Requiem Eterna Relic",
                    "/Lotus/Types/Game/Projections/T5VoidProjectionImmortalOmniA",
                    &["relic"],
                ),
            ],
        );
        assert_eq!(owned("lith_g12_relic", None, Some("intact")), 25);
        assert_eq!(owned("lith_g12_relic", None, Some("radiant")), 2);
        assert_eq!(owned("lith_g12_relic", None, Some("flawless")), 0);
        assert_eq!(owned("lith_g12_relic", None, None), 0);
        assert_eq!(owned("requiem_eterna_relic", None, Some("intact")), 21);
    }

    #[test]
    fn misc_counts_every_container() {
        let tink = |catalog: Catalog| {
            holdings(
                catalog,
                fixtures::inventory(),
                &[market_item(
                    "tink",
                    "Tink",
                    "/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAItem",
                    &["fish"],
                )],
            )("tink", None, None)
        };
        let both_sizes = catalog_of(
            r#"[
              {"uniqueName":"/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAItem",
               "name":"Tink","category":"Fish","type":"Fish","tradable":true},
              {"uniqueName":"/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAMediumItem",
               "name":"Tink","category":"Fish","type":"Fish","tradable":true}
            ]"#,
        );
        assert_eq!(tink(both_sizes), 119);

        assert_eq!(tink(fixtures::catalog()), 119);
        let owned = holdings(
            fixtures::catalog(),
            fixtures::inventory(),
            &[
                market_item(
                    "ayatan_vaya_sculpture",
                    "Ayatan Vaya Sculpture",
                    "/Lotus/Types/Items/FusionTreasures/OroFusexD",
                    &["sculpture"],
                ),
                market_item(
                    "nihils_oubliette_(key)",
                    "Nihil's Oubliette (Key)",
                    "/Lotus/Types/Items/ShipDecos/Nightwave/GlassmakerShipDeco",
                    &["key"],
                ),
                market_item(
                    "prisma_angstrum",
                    "Prisma Angstrum",
                    "/Lotus/Weapons/Corpus/Pistols/CrpHandRL/PrismaAngstrum",
                    &["weapon"],
                ),
                market_item(
                    "vasca_kavat_imprint",
                    "Vasca Kavat Imprint",
                    "/Lotus/Types/Game/CatbrowPet/VampireCatbrowPetPowerSuit",
                    &["imprint"],
                ),
                market_item(
                    "panzer_vulpaphyla_imprint",
                    "Panzer Vulpaphyla Imprint",
                    "/Lotus/Types/Friendly/Pets/CreaturePets/ArmoredInfestedCatbrowPetPowerSuit",
                    &["imprint"],
                ),
                market_item(
                    "legendary_fusion_core",
                    "Legendary Fusion Core",
                    "",
                    &["fusion core"],
                ),
            ],
        );
        assert_eq!(owned("ayatan_vaya_sculpture", None, None), 35);
        assert_eq!(owned("nihils_oubliette_(key)", None, None), 2);
        assert_eq!(owned("prisma_angstrum", None, None), 1, "an unranked spare");
        assert_eq!(owned("vasca_kavat_imprint", None, None), 1);
        assert_eq!(owned("panzer_vulpaphyla_imprint", None, None), 4);
        assert_eq!(owned("legendary_fusion_core", None, None), 6);
    }

    fn upgrade_inventory() -> wf_inventory::Inventory {
        fixtures::inventory_with(|inventory| {
            inventory["RawUpgrades"] = [
                (PRIMED_CONTINUITY, 3),
                (FEAR_SENSE, 25),
                (CLASHING_FOREST, 158),
                (VEILED_RIFLE, 2),
                ("/Lotus/Upgrades/Mods/Randomized/RawRifleRandomMod", 5),
                (ENERGIZE, 4),
            ]
            .iter()
            .map(|(item_type, count)| json!({ "ItemType": item_type, "ItemCount": count }))
            .collect();
            inventory["Upgrades"] = [(PRIMED_CONTINUITY, 10), (CLASHING_FOREST, 3), (ENERGIZE, 5)]
                .iter()
                .enumerate()
                .map(|(index, (item_type, rank))| {
                    json!({
                        "ItemType": item_type,
                        "UpgradeFingerprint": format!("{{\"lvl\":{rank}}}"),
                        "ItemId": { "$oid": format!("{index:024x}") },
                    })
                })
                .collect();
        })
    }

    #[test]
    fn upgrades_count_by_rank_and_name() {
        let mut upgrades: Vec<Value> = serde_json::from_str(fixtures::UPGRADE_ITEMS).unwrap();
        upgrades.push(json!({
            "uniqueName": FEAR_SENSE,
            "name": "Fear Sense",
            "category": "Mods",
            "type": "Mod",
            "tradable": true,
        }));
        let catalog = catalog_of(&Value::from(upgrades).to_string());
        let owned = holdings(
            catalog,
            upgrade_inventory(),
            &[
                market_item("sense_danger", "Fear Sense", "", &["mod"]),
                market_item(
                    "clashing_forest",
                    "Clashing Forest",
                    CLASHING_FOREST,
                    &["mod"],
                ),
                market_item(
                    "rifle_riven_mod_(veiled)",
                    "Rifle Riven Mod (Veiled)",
                    VEILED_RIFLE,
                    &["mod", "riven_mod"],
                ),
                market_item(
                    "arcane_energize",
                    "Arcane Energize",
                    ENERGIZE,
                    &["arcane_enhancement"],
                ),
                market_item(
                    "arcane_barrier",
                    "Arcane Barrier",
                    "/Lotus/Upgrades/CosmeticEnhancers/Defensive/InstantShieldOnDamage",
                    &["arcane_enhancement"],
                ),
            ],
        );
        assert_eq!(
            owned("sense_danger", Some(0), None),
            25,
            "a renamed mod counts by its name"
        );
        assert_eq!(owned("clashing_forest", Some(0), None), 158);
        assert_eq!(owned("clashing_forest", Some(3), None), 1);
        assert_eq!(owned("rifle_riven_mod_(veiled)", Some(0), None), 7);
        assert_eq!(owned("arcane_energize", Some(5), None), 1);
        assert_eq!(owned("arcane_energize", Some(0), None), 4);
        assert_eq!(owned("arcane_barrier", Some(0), None), 0);
    }
}
