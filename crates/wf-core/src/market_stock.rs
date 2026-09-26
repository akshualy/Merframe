use std::collections::BTreeSet;

use wf_inventory::{CountedItem, EquipmentItem, Inventory};

use crate::catalog::Catalog;

#[derive(Clone, Copy)]
pub struct MarketStock<'a> {
    pub(crate) inventory: &'a Inventory,
    pub(crate) catalog: &'a Catalog,
}

fn counted(items: &[CountedItem], unique_name: &str) -> i64 {
    items
        .iter()
        .filter(|item| item.item_type == unique_name)
        .map(|item| item.item_count)
        .sum()
}

fn is_fish(unique_name: &str) -> bool {
    unique_name.contains("/Items/Fish/")
}

fn refinement_suffix(listed: &str) -> Option<&'static str> {
    match listed.to_lowercase().as_str() {
        "intact" => Some("Bronze"),
        "exceptional" => Some("Silver"),
        "flawless" => Some("Gold"),
        "radiant" => Some("Platinum"),
        _ => None,
    }
}

fn entries<T>(items: &[T], matches: impl Fn(&T) -> bool) -> i64 {
    items.iter().filter(|item| matches(item)).map(|_| 1).sum()
}

impl MarketStock<'_> {
    pub fn part(&self, item: &wf_market::Item) -> i64 {
        let mut unique_name = item.game_ref.clone();
        if unique_name.contains("CrpArSniper") && !unique_name.contains("Blueprint") {
            unique_name = unique_name.replace("CrpArSniper", "Ambassador") + "Blueprint";
        }
        if !item.slug.contains("kavasa") {
            unique_name = unique_name.replace("Component", "Blueprint");
        }
        let blueprint = format!("{unique_name}Blueprint");
        [&self.inventory.misc_items, &self.inventory.recipes]
            .into_iter()
            .map(|items| counted(items, &unique_name) + counted(items, &blueprint))
            .sum()
    }

    pub fn relic(&self, item: &wf_market::Item, refinement: &str) -> i64 {
        let Some(suffix) = refinement_suffix(refinement) else {
            return 0;
        };
        let relics = &self.inventory.misc_items;
        let refined = counted(relics, &format!("{}{suffix}", item.game_ref));
        if suffix == "Bronze" {
            refined + counted(relics, &item.game_ref)
        } else {
            refined
        }
    }

    pub fn misc(&self, item: &wf_market::Item, listed_name: &str) -> i64 {
        let wanted = listed_name.to_lowercase();
        if let Some(pet) = wanted.strip_suffix(" imprint") {
            return self.imprints(pet, &item.game_ref);
        }
        self.unique_names(item, listed_name)
            .into_iter()
            .map(|unique_name| self.held(unique_name))
            .sum()
    }

    fn unique_names<'a>(
        &'a self,
        item: &'a wf_market::Item,
        listed_name: &str,
    ) -> BTreeSet<&'a str> {
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
        let listed = match listed_name {
            "Nihil's Oubliette (Key)" => "/Lotus/Types/Keys/Nightwave/GlassmakerBossFightKey",
            "Legendary Fusion Core" => "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser",
            _ => item.game_ref.as_str(),
        };
        if !listed.is_empty() {
            unique_names.insert(listed);
        }
        unique_names
    }

    fn held(&self, unique_name: &str) -> i64 {
        let inventory = self.inventory;
        if is_fish(unique_name) {
            let species = unique_name.strip_suffix("Item").unwrap_or(unique_name);
            return inventory
                .misc_items
                .iter()
                .filter(|item| item.item_type.contains(species))
                .map(|item| item.item_count)
                .sum();
        }
        let unranked = |owned: &EquipmentItem| owned.item_type == unique_name && owned.xp == 0;
        let spare_equipment: i64 = [
            &inventory.long_guns,
            &inventory.pistols,
            &inventory.melee,
            &inventory.space_guns,
            &inventory.space_melee,
            &inventory.space_suits,
            &inventory.sentinel_weapons,
        ]
        .into_iter()
        .map(|owned| entries(owned, unranked))
        .sum();
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

    fn imprints(&self, pet: &str, game_ref: &str) -> i64 {
        let named = self
            .catalog
            .items()
            .find(|known| known.name.to_lowercase() == pet.trim())
            .map(|known| known.unique_name.as_str());
        entries(&self.inventory.kubrow_pet_prints, |print| {
            let personality = print.dominant_traits.personality.as_str();
            Some(personality) == named || personality == game_ref
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::catalog::fixtures;

    fn listed(slug: &str, game_ref: &str) -> wf_market::Item {
        wf_market::Item {
            id: slug.to_owned(),
            slug: slug.to_owned(),
            game_ref: game_ref.to_owned(),
            tags: Vec::new(),
            max_rank: None,
            subtypes: None,
            max_amber_stars: None,
            max_cyan_stars: None,
            tradable: Some(true),
            bulk_tradable: None,
            vaulted: None,
            ducats: None,
            rarity: None,
            i18n: HashMap::new(),
        }
    }

    #[test]
    fn part_counts_blueprint_stock() {
        let catalog = fixtures::catalog();
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
            ],
        );
        let stock = MarketStock {
            inventory: &inventory,
            catalog: &catalog,
        };
        let owned = |slug: &str, game_ref: &str| stock.part(&listed(slug, game_ref));
        assert_eq!(
            owned(
                "braton_prime_barrel",
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel"
            ),
            3
        );
        assert_eq!(
            owned(
                "orvius_disc",
                "/Lotus/Types/Recipes/Weapons/WeaponParts/TeshinGlaiveDisc"
            ),
            63
        );
        assert_eq!(
            owned(
                "rhino_prime_chassis_blueprint",
                "/Lotus/Types/Recipes/WarframeRecipes/RhinoPrimeChassisComponent"
            ),
            2
        );
        assert_eq!(
            owned(
                "kavasa_prime_band",
                "/Lotus/Types/Recipes/Kubrow/Collars/PrimeKubrowCollarABandComponent"
            ),
            1
        );
        assert_eq!(
            owned(
                "ambassador_barrel",
                "/Lotus/Types/Recipes/Weapons/WeaponParts/CrpArSniperBarrel"
            ),
            4
        );
    }

    #[test]
    fn relic_counts_by_refinement() {
        let catalog = fixtures::catalog();
        let inventory = fixtures::inventory_owning(&[(
            "/Lotus/Types/Game/Projections/T1VoidProjectionSevagothPrimeDPlatinum",
            2,
        )]);
        let stock = MarketStock {
            inventory: &inventory,
            catalog: &catalog,
        };
        let lith = listed(
            "lith_g12_relic",
            "/Lotus/Types/Game/Projections/T1VoidProjectionSevagothPrimeD",
        );
        assert_eq!(stock.relic(&lith, "intact"), 25);
        assert_eq!(stock.relic(&lith, "radiant"), 2);
        assert_eq!(stock.relic(&lith, "flawless"), 0);
        let eterna = listed(
            "requiem_eterna_relic",
            "/Lotus/Types/Game/Projections/T5VoidProjectionImmortalOmniA",
        );
        assert_eq!(
            stock.relic(&eterna, "intact"),
            21,
            "a relic the export does not know, held without a refinement suffix"
        );
    }

    #[test]
    fn fish_sizes_counted_once() {
        let catalog = Catalog::from_json(
            r#"[
              {"uniqueName":"/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAItem",
               "name":"Tink","category":"Fish","type":"Fish","tradable":true},
              {"uniqueName":"/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAMediumItem",
               "name":"Tink","category":"Fish","type":"Fish","tradable":true}
            ]"#,
            fixtures::RELICS,
            "[]",
        )
        .unwrap();
        let inventory = fixtures::inventory();
        let stock = MarketStock {
            inventory: &inventory,
            catalog: &catalog,
        };
        let tink = listed(
            "tink",
            "/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAItem",
        );
        assert_eq!(stock.misc(&tink, "Tink"), 119);
    }

    #[test]
    fn misc_counts_every_container() {
        let catalog = fixtures::catalog();
        let inventory = fixtures::inventory();
        let stock = MarketStock {
            inventory: &inventory,
            catalog: &catalog,
        };
        let owned =
            |slug: &str, name: &str, game_ref: &str| stock.misc(&listed(slug, game_ref), name);
        assert_eq!(
            owned(
                "tink",
                "Tink",
                "/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAItem"
            ),
            119,
            "every size of the fish"
        );
        assert_eq!(
            owned(
                "ayatan_vaya_sculpture",
                "Ayatan Vaya Sculpture",
                "/Lotus/Types/Items/FusionTreasures/OroFusexD"
            ),
            35
        );
        assert_eq!(
            owned(
                "nihils_oubliette_(key)",
                "Nihil's Oubliette (Key)",
                "/Lotus/Types/Items/ShipDecos/Nightwave/GlassmakerShipDeco"
            ),
            2
        );
        assert_eq!(
            owned(
                "prisma_angstrum",
                "Prisma Angstrum",
                "/Lotus/Weapons/Corpus/Pistols/CrpHandRL/PrismaAngstrum"
            ),
            1,
            "an unranked spare"
        );
        assert_eq!(
            owned(
                "vasca_kavat_imprint",
                "Vasca Kavat Imprint",
                "/Lotus/Types/Game/CatbrowPet/VampireCatbrowPetPowerSuit"
            ),
            1
        );
        assert_eq!(
            owned("legendary_fusion_core", "Legendary Fusion Core", ""),
            6
        );
    }
}
