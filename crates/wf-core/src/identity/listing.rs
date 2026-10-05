use std::borrow::Cow;

use wf_data::Refinement;

use super::{ItemKind, ItemRecord, ItemRef, ItemTable, Variant, ambassador_blueprint, market_slug};
use crate::catalog::{RECIPE_PREFIX, RELIC_PREFIX, projection_suffix};
use crate::trade::name_key;

static LISTINGS_OF_ANOTHER_ITEM: [(&str, &str); 3] = [
    (
        "Nihil's Oubliette (Key)",
        "/Lotus/Types/Keys/Nightwave/GlassmakerBossFightKey",
    ),
    (
        "Legendary Fusion Core",
        "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser",
    ),
    (
        "Scan Aquatic Lifeforms",
        "/Lotus/Types/Sentinels/SentinelPrecepts/LocateCreatures",
    ),
];

static DIALOG_NAMES: [(&str, &str); 3] = [
    ("Enter Nihil's Oubliette", "Nihil's Oubliette (Key)"),
    ("Legendary Core", "Legendary Fusion Core"),
    ("Ancient Core", "Ancient Fusion Core"),
];

pub fn market_name(item: &wf_market::Item) -> &str {
    item.i18n
        .get("en")
        .map_or(item.slug.as_str(), |english| english.name.as_str())
}

fn listed_game_ref(item: &wf_market::Item) -> Cow<'_, str> {
    let name = market_name(item);
    if let Some((_, unique_name)) = LISTINGS_OF_ANOTHER_ITEM
        .iter()
        .find(|(listed, _)| *listed == name)
    {
        return Cow::Borrowed(unique_name);
    }
    let game_ref = item.game_ref.as_str();
    if game_ref.starts_with(RECIPE_PREFIX)
        && let Some(blueprint) = ambassador_blueprint(game_ref)
    {
        return Cow::Owned(blueprint);
    }
    Cow::Borrowed(game_ref)
}

fn listed_kind(item: &wf_market::Item) -> ItemKind {
    let tagged = |tag: &str| item.tags.iter().any(|owned| owned == tag);
    if tagged("component") || tagged("blueprint") {
        return ItemKind::Other;
    }
    if tagged("imprint") {
        return ItemKind::Imprint;
    }
    if tagged("relic") {
        return ItemKind::Relic {
            refinement: Refinement::Intact,
        };
    }
    if tagged("mod") || tagged("arcane") || tagged("arcane_enhancement") {
        return ItemKind::Upgrade {
            rarity: None,
            max_rank: item.max_rank,
        };
    }
    ItemKind::Other
}

pub(crate) fn traded_as(unique_name: &str) -> Cow<'_, str> {
    if unique_name.contains("/Kubrow/Collars/") {
        Cow::Borrowed(unique_name)
    } else {
        Cow::Owned(unique_name.replace("Component", "Blueprint"))
    }
}

impl ItemTable {
    pub fn index_market(&mut self, items: &[wf_market::Item]) -> usize {
        self.records.truncate(self.catalogued);
        self.by_market_id.clear();
        self.by_game_ref.clear();
        self.listing_by_name.clear();
        for record in &mut self.records {
            if record.takes_market_slug() {
                record.market_slug = Some(market_slug(&record.name));
            }
        }
        for item in items {
            let game_ref = listed_game_ref(item);
            let target = match self.catalog_match(&game_ref, item) {
                Some(target) => target,
                None => self.push(
                    ItemRecord::new(
                        &game_ref,
                        market_name(item).to_owned(),
                        None,
                        listed_kind(item),
                    )
                    .slugged(item.slug.clone()),
                ),
            };
            let record = &mut self.records[target.0];
            if record.takes_market_slug() {
                record.market_slug = Some(item.slug.clone());
            }
            self.by_market_id.insert(item.id.clone(), target);
            if !game_ref.is_empty() {
                self.by_game_ref.insert(game_ref.into_owned(), target);
            }
            if let Some(english) = item.i18n.get("en") {
                self.listing_by_name
                    .entry(name_key(&english.name))
                    .or_insert_with(|| item.id.clone());
            }
        }
        for (dialog, listed) in DIALOG_NAMES {
            match self.listing_by_name.get(&name_key(listed)).cloned() {
                Some(id) => self.listing_by_name.insert(name_key(dialog), id),
                None => self.listing_by_name.remove(&name_key(dialog)),
            };
        }
        self.by_game_ref.len()
    }

    pub fn listing_id(&self, dialog_name: &str) -> Option<&str> {
        let named = |name: &str| {
            self.listing_by_name
                .get(&name_key(name))
                .map(String::as_str)
        };
        named(dialog_name)
            .or_else(|| named(dialog_name.strip_suffix(" Set")?))
            .or_else(|| {
                dialog_name
                    .ends_with(" Riven Mod")
                    .then(|| named(&format!("{dialog_name} (Veiled)")))
                    .flatten()
            })
    }

    fn catalog_match(&self, game_ref: &str, item: &wf_market::Item) -> Option<ItemRef> {
        let intact_relic = || {
            game_ref
                .starts_with(RELIC_PREFIX)
                .then(|| {
                    let intact = projection_suffix(Refinement::Intact);
                    self.by_unique_name.get(&format!("{game_ref}{intact}"))
                })
                .flatten()
        };
        let component = || {
            let base = game_ref.strip_suffix("Blueprint")?;
            self.by_unique_name.get(&format!("{base}Component"))
        };
        let base = self
            .by_unique_name
            .get(game_ref)
            .or_else(intact_relic)
            .or_else(component)?;
        Some(self.variant_for(*base, item))
    }

    fn variant_for(&self, base: ItemRef, item: &wf_market::Item) -> ItemRef {
        let tagged = |wanted: &str| item.tags.iter().any(|tag| tag == wanted);
        let variant = if tagged("set") {
            Variant::Set
        } else if tagged("imprint") {
            Variant::Imprint
        } else if matches!(self[base].kind, ItemKind::Riven) {
            Variant::Veiled
        } else {
            return base;
        };
        self.variants.get(&(base, variant)).copied().unwrap_or(base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures::{self, market_item};

    #[test]
    fn listing_records() {
        let mut table = ItemTable::build(&fixtures::catalog());
        let items = [
            market_item(
                "trinity_prime_systems_blueprint",
                "Trinity Prime Systems Blueprint",
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint",
                &["component", "blueprint"],
            ),
            market_item(
                "axi_a1_relic",
                "Axi A1 Relic",
                "/Lotus/Types/Game/Projections/T4VoidProjectionE",
                &["relic"],
            ),
            market_item(
                "meso_c1_relic",
                "Meso C1 Relic",
                "/Lotus/Types/Game/MissionDecks/VoidMissionDecks/MesoC1VoidProjection",
                &["relic"],
            ),
            market_item(
                "ambassador_barrel",
                "Ambassador Barrel",
                "/Lotus/Types/Recipes/Weapons/WeaponParts/CrpArSniperBarrel",
                &["component"],
            ),
            market_item(
                "legendary_fusion_core",
                "Legendary Fusion Core",
                "",
                &["fusion core"],
            ),
            market_item("melee_influence", "Melee Influence", "", &["arcane"]),
            market_item(
                "panzer_vulpaphyla_imprint",
                "Panzer Vulpaphyla Imprint",
                "/Lotus/Types/Friendly/Pets/CreaturePets/ArmoredInfestedCatbrowPetPowerSuit",
                &["imprint"],
            ),
        ];
        assert_eq!(table.index_market(&items), 6);
        let record = |id: &str| table.by_market_id(id).unwrap();

        let systems = record("trinity_prime_systems_blueprint");
        assert_eq!(
            systems.unique_name,
            "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsComponent"
        );
        assert!(matches!(systems.kind, ItemKind::Part { .. }));
        assert_eq!(
            record("axi_a1_relic").kind,
            ItemKind::Relic {
                refinement: Refinement::Intact
            }
        );
        assert_eq!(
            record("axi_a1_relic").unique_name,
            "/Lotus/Types/Game/Projections/T4VoidProjectionEBronze"
        );
        assert_eq!(
            record("meso_c1_relic").kind,
            ItemKind::Relic {
                refinement: Refinement::Intact
            }
        );
        assert_eq!(
            record("ambassador_barrel").unique_name,
            "/Lotus/Types/Recipes/Weapons/WeaponParts/AmbassadorBarrelBlueprint"
        );
        assert_eq!(
            record("legendary_fusion_core").unique_name,
            "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser"
        );
        assert!(matches!(
            record("melee_influence").kind,
            ItemKind::Upgrade { .. }
        ));
        assert_eq!(record("melee_influence").name, "Melee Influence");
        assert_eq!(record("panzer_vulpaphyla_imprint").kind, ItemKind::Imprint);
    }

    #[test]
    fn names_and_traded_types() {
        let forma = market_item("forma_blueprint", "Forma Blueprint", "", &[]);
        let mut bare = market_item("no_english", "", "", &[]);
        bare.i18n.clear();
        assert_eq!(market_name(&forma), "Forma Blueprint");
        assert_eq!(market_name(&bare), "no_english");
        assert_eq!(
            traded_as("/Lotus/Types/Recipes/WarframeRecipes/RhinoPrimeChassisComponent"),
            "/Lotus/Types/Recipes/WarframeRecipes/RhinoPrimeChassisBlueprint"
        );
        assert_eq!(
            traded_as("/Lotus/Types/Recipes/Kubrow/Collars/PrimeKubrowCollarABandComponent"),
            "/Lotus/Types/Recipes/Kubrow/Collars/PrimeKubrowCollarABandComponent",
            "kubrow collar parts trade built"
        );
    }
}
