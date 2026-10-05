mod listing;

use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Index;

use wf_data::{Component, Item, Rarity, Refinement, catch_grade};
use wf_inventory::RIVEN_MARKER;

use crate::catalog::{
    Catalog, REFINEMENTS, VaultStatus, component_image, is_fish, is_prime, names_a_prime,
    part_name, refinement_name,
};

pub use listing::market_name;
pub(crate) use listing::traded_as;

pub(crate) struct UnlistedUpgrade {
    unique_name: &'static str,
    pub(crate) name: &'static str,
    rarity: Rarity,
    market_slug: &'static str,
    image_name: Option<&'static str>,
}

static UPGRADES_OUTSIDE_THE_EXPORT: [UnlistedUpgrade; 6] = [
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/AmmoEfficencyDuringUltimate",
        name: "Zid-An Haras",
        rarity: Rarity::Rare,
        market_slug: "zid-an-haras",
        image_name: None,
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/HeatStatusProcOnUltimateKill",
        name: "Zid-An Uskos",
        rarity: Rarity::Rare,
        market_slug: "zid-an-uskos",
        image_name: None,
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/StatusChanceOnUltimateHit",
        name: "Zid-An Asheir",
        rarity: Rarity::Rare,
        market_slug: "zid-an-asheir",
        image_name: None,
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/UltimateInvisibilty",
        name: "Zid-An Sek-Eel",
        rarity: Rarity::Rare,
        market_slug: "zid-an-sek-eel",
        image_name: None,
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/VoidSlingsOverguardStrip",
        name: "Zid-An Osbok",
        rarity: Rarity::Rare,
        market_slug: "zid-an-osbok",
        image_name: None,
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser",
        name: "Legendary Core",
        rarity: Rarity::Legendary,
        market_slug: "legendary_fusion_core",
        image_name: Some("game/legendary-core.png"),
    },
];

pub(crate) fn unlisted_upgrade(unique_name: &str) -> Option<&'static UnlistedUpgrade> {
    UPGRADES_OUTSIDE_THE_EXPORT
        .iter()
        .find(|upgrade| upgrade.unique_name == unique_name)
}

pub fn market_icon(catalog: &Catalog, item: &wf_market::Item) -> Option<String> {
    catalog.icon_for(&item.game_ref).or_else(|| {
        UPGRADES_OUTSIDE_THE_EXPORT
            .iter()
            .find(|upgrade| upgrade.market_slug == item.slug)?
            .image_name
            .map(str::to_owned)
    })
}

pub(crate) fn market_slug(display_name: &str) -> String {
    let mut slug = String::with_capacity(display_name.len());
    let mut pending_separator = false;
    for ch in display_name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('_');
            }
            pending_separator = false;
            slug.push(ch.to_ascii_lowercase());
        } else if ch != '\'' {
            pending_separator = true;
        }
    }
    slug
}

pub(crate) fn part_market_slug(item: &Item, component: &Component) -> String {
    let name = part_name(item, component);
    if component.unique_name.ends_with("Component") {
        market_slug(&format!("{name} Blueprint"))
    } else {
        market_slug(&name)
    }
}

pub(crate) fn is_riven(unique_name: &str) -> bool {
    unique_name.contains(RIVEN_MARKER)
}

pub(crate) fn ambassador_blueprint(unique_name: &str) -> Option<String> {
    if !unique_name.contains("CrpArSniper") || unique_name.contains("Blueprint") {
        return None;
    }
    Some(format!(
        "{}Blueprint",
        unique_name.replace("CrpArSniper", "Ambassador")
    ))
}

fn starter_original(unique_name: &str) -> Option<String> {
    let (parent, leaf) = unique_name.strip_suffix("Beginner")?.rsplit_once('/')?;
    Some(format!("{}/{leaf}", parent.strip_suffix("/Beginner")?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemRef(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variant {
    Set,
    Imprint,
    Veiled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Item,
    Upgrade {
        rarity: Option<Rarity>,
        max_rank: Option<u32>,
    },
    Riven,
    Set {
        parts: Vec<ItemRef>,
    },
    Part {
        set: ItemRef,
    },
    Relic {
        refinement: Refinement,
    },
    Imprint,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemRecord {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub market_slug: Option<String>,
    pub prime: bool,
    pub vault: Option<VaultStatus>,
    pub kind: ItemKind,
}

impl ItemRecord {
    pub(crate) fn unknown(unique_name: &str, name: String) -> Self {
        Self::new(unique_name, name, None, ItemKind::Other)
    }

    fn new(unique_name: &str, name: String, image_name: Option<String>, kind: ItemKind) -> Self {
        Self {
            unique_name: unique_name.to_owned(),
            market_slug: Some(market_slug(&name)),
            prime: name.contains("Prime"),
            vault: None,
            image_name,
            name,
            kind,
        }
    }

    fn slugged(self, slug: String) -> Self {
        Self {
            market_slug: Some(slug),
            ..self
        }
    }

    fn vaulted(self, vaulted: Option<bool>) -> Self {
        Self {
            vault: names_a_prime(&self.name).then(|| VaultStatus::from(vaulted)),
            ..self
        }
    }

    fn takes_market_slug(&self) -> bool {
        matches!(self.kind, ItemKind::Upgrade { .. })
            && unlisted_upgrade(&self.unique_name).is_none()
    }
}

#[derive(Debug, Default)]
pub struct ItemTable {
    records: Vec<ItemRecord>,
    by_unique_name: HashMap<String, ItemRef>,
    variants: HashMap<(ItemRef, Variant), ItemRef>,
    by_market_id: HashMap<String, ItemRef>,
    by_game_ref: HashMap<String, ItemRef>,
    listing_by_name: HashMap<String, String>,
    catalogued: usize,
}

impl Index<ItemRef> for ItemTable {
    type Output = ItemRecord;

    fn index(&self, item: ItemRef) -> &ItemRecord {
        &self.records[item.0]
    }
}

impl ItemTable {
    pub fn build(catalog: &Catalog) -> Self {
        let mut table = Self::default();
        let items = catalog.data().items();
        let bases: Vec<ItemRef> = items.iter().map(|item| table.add_item(item)).collect();
        for (item, base) in items.iter().zip(bases) {
            table.add_parts(catalog, item, base);
            table.add_grades(catalog, item, base);
            if item.category == "Pets" {
                let name = format!("{} Imprint", item.name);
                let imprint = table.push(ItemRecord::new(
                    &item.unique_name,
                    name,
                    item.image_name.clone(),
                    ItemKind::Imprint,
                ));
                table.variants.insert((base, Variant::Imprint), imprint);
            }
        }
        table.add_relics(catalog);
        for item in items {
            if let Some(original) = starter_original(&item.unique_name)
                && !table.by_unique_name.contains_key(&original)
            {
                let record = upgrade_record(&original, item, "");
                table.insert(record);
            }
        }
        for upgrade in &UPGRADES_OUTSIDE_THE_EXPORT {
            if !table.by_unique_name.contains_key(upgrade.unique_name) {
                let record = ItemRecord::new(
                    upgrade.unique_name,
                    upgrade.name.to_owned(),
                    upgrade.image_name.map(str::to_owned),
                    ItemKind::Upgrade {
                        rarity: Some(upgrade.rarity),
                        max_rank: None,
                    },
                )
                .slugged(upgrade.market_slug.to_owned());
                table.insert(record);
            }
        }
        table.catalogued = table.records.len();
        table
    }

    fn push(&mut self, record: ItemRecord) -> ItemRef {
        self.records.push(record);
        ItemRef(self.records.len() - 1)
    }

    fn insert(&mut self, record: ItemRecord) -> ItemRef {
        let unique_name = record.unique_name.clone();
        let item = self.push(record);
        self.by_unique_name.insert(unique_name, item);
        item
    }

    fn add_item(&mut self, item: &Item) -> ItemRef {
        if is_riven(&item.unique_name) {
            let veiled_slug = format!("{}_(veiled)", market_slug(&item.name));
            let base = self.insert(
                ItemRecord::new(
                    &item.unique_name,
                    item.name.clone(),
                    item.image_name.clone(),
                    ItemKind::Riven,
                )
                .slugged(veiled_slug),
            );
            let veiled = upgrade_record(&item.unique_name, item, " (Veiled)");
            let veiled = self.push(veiled);
            self.variants.insert((base, Variant::Veiled), veiled);
            return base;
        }
        if item.category == "Mods"
            || item.category == "Arcanes"
            || unlisted_upgrade(&item.unique_name).is_some()
        {
            return self.insert(upgrade_record(&item.unique_name, item, ""));
        }
        self.insert(
            ItemRecord::new(
                &item.unique_name,
                item.name.clone(),
                item.image_name.clone(),
                ItemKind::Item,
            )
            .vaulted(item.vaulted),
        )
    }

    fn add_parts(&mut self, catalog: &Catalog, item: &Item, base: ItemRef) {
        let Some(components) = item.components.as_deref().filter(|parts| !parts.is_empty()) else {
            return;
        };
        let set = self.push(
            ItemRecord::new(
                &item.unique_name,
                item.name.clone(),
                item.image_name.clone(),
                ItemKind::Set { parts: Vec::new() },
            )
            .slugged(format!("{}_set", market_slug(&item.name)))
            .vaulted(item.vaulted),
        );
        self.variants.insert((base, Variant::Set), set);
        let parts = components
            .iter()
            .map(|component| {
                let record = ItemRecord {
                    prime: is_prime(item),
                    ..ItemRecord::new(
                        &component.unique_name,
                        part_name(item, component),
                        component_image(item, component),
                        ItemKind::Part { set },
                    )
                    .slugged(part_market_slug(item, component))
                    .vaulted(item.vaulted)
                };
                let part = self.push(record);
                let first_owner = catalog
                    .component(&component.unique_name)
                    .is_some_and(|(_, found)| std::ptr::eq(found, component));
                if first_owner {
                    self.by_unique_name
                        .entry(component.unique_name.clone())
                        .or_insert(part);
                }
                part
            })
            .collect();
        self.records[set.0].kind = ItemKind::Set { parts };
    }

    fn add_grades(&mut self, catalog: &Catalog, item: &Item, base: ItemRef) {
        if !is_fish(&item.unique_name) {
            return;
        }
        let stem = item.unique_name.strip_suffix("Item");
        for marker in ["Large", "Medium"] {
            let graded = [
                Some(format!("{}{marker}", item.unique_name)),
                stem.map(|stem| format!("{stem}{marker}Item")),
            ];
            for unique_name in graded.into_iter().flatten() {
                let Some((_, grade)) =
                    catch_grade(&unique_name).filter(|(found, _)| *found == item.unique_name)
                else {
                    continue;
                };
                let image_name = catalog
                    .item(&unique_name)
                    .and_then(|own| own.image_name.clone());
                let slug = self[base].market_slug.clone();
                let record = ItemRecord {
                    market_slug: slug,
                    ..ItemRecord::new(
                        &unique_name,
                        format!("{} ({grade})", item.name),
                        image_name,
                        ItemKind::Item,
                    )
                };
                self.insert(record);
            }
        }
    }

    fn add_relics(&mut self, catalog: &Catalog) {
        for relic in catalog.relics() {
            for refinement in REFINEMENTS {
                let Some(unique_name) = relic.unique_names.get(&refinement) else {
                    continue;
                };
                let record = ItemRecord {
                    market_slug: relic.market_info.as_ref().map(|info| info.url_name.clone()),
                    vault: Some(VaultStatus::from(Some(relic.vaulted))),
                    ..ItemRecord::new(
                        unique_name,
                        format!("{} Relic ({})", relic.name, refinement_name(refinement)),
                        relic.image_names.get(&refinement).cloned(),
                        ItemKind::Relic { refinement },
                    )
                };
                self.insert(record);
            }
        }
    }

    pub fn get(&self, unique_name: &str) -> Option<&ItemRecord> {
        self.by_unique_name
            .get(unique_name)
            .map(|&item| &self[item])
    }

    pub fn variant(&self, unique_name: &str, variant: Variant) -> Option<&ItemRecord> {
        let base = self.by_unique_name.get(unique_name)?;
        self.variants
            .get(&(*base, variant))
            .map(|&item| &self[item])
    }

    pub(crate) fn part(&self, item: &Item, component: &Component) -> Option<&ItemRecord> {
        let ItemKind::Set { parts } = &self.variant(&item.unique_name, Variant::Set)?.kind else {
            return None;
        };
        parts
            .iter()
            .map(|&part| &self[part])
            .find(|part| part.unique_name == component.unique_name)
    }

    pub(crate) fn resolve(
        &self,
        unique_name: &str,
        name: impl FnOnce() -> String,
    ) -> Cow<'_, ItemRecord> {
        match self
            .get(unique_name)
            .or_else(|| self.by_game_ref(unique_name))
        {
            Some(record) => Cow::Borrowed(record),
            None => Cow::Owned(ItemRecord::unknown(unique_name, name())),
        }
    }

    pub fn by_market_id(&self, id: &str) -> Option<&ItemRecord> {
        self.by_market_id.get(id).map(|&item| &self[item])
    }

    pub fn by_game_ref(&self, game_ref: &str) -> Option<&ItemRecord> {
        self.by_game_ref.get(game_ref).map(|&item| &self[item])
    }
}

fn upgrade_record(unique_name: &str, item: &Item, suffix: &str) -> ItemRecord {
    let unlisted = unlisted_upgrade(unique_name);
    let record = ItemRecord::new(
        unique_name,
        format!("{}{suffix}", item.name),
        item.image_name
            .clone()
            .or_else(|| unlisted.and_then(|upgrade| upgrade.image_name.map(str::to_owned))),
        ItemKind::Upgrade {
            rarity: item.rarity.or(unlisted.map(|upgrade| upgrade.rarity)),
            max_rank: Some(item.max_upgrade_rank()),
        },
    );
    match unlisted {
        Some(upgrade) => record.slugged(upgrade.market_slug.to_owned()),
        None => record,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures::{self, market_item};

    const BARREL: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel";
    const BRATON: &str = "/Lotus/Weapons/Tenno/Rifle/BratonPrime";
    const ENERGIZE: &str =
        "/Lotus/Upgrades/CosmeticEnhancers/Utility/GolemArcaneRadialEnergyOnEnergyPickup";
    const HARAS: &str = "/Lotus/Upgrades/CosmeticEnhancers/Antiques/AmmoEfficencyDuringUltimate";

    fn slug_of(table: &ItemTable, unique_name: &str) -> Option<String> {
        table
            .get(unique_name)
            .and_then(|record| record.market_slug.clone())
    }

    #[test]
    fn slug_rules() {
        assert_eq!(market_slug("Braton Prime Barrel"), "braton_prime_barrel");
        assert_eq!(market_slug("Axi A1 Relic"), "axi_a1_relic");
        assert_eq!(market_slug("Gaia's Tragedy"), "gaias_tragedy");
        assert_eq!(
            market_slug("Zephyr Prime  Blueprint"),
            "zephyr_prime_blueprint"
        );
        assert_eq!(
            starter_original("/Lotus/Upgrades/Mods/Warframe/Beginner/AvatarSlideBoostModBeginner")
                .as_deref(),
            Some("/Lotus/Upgrades/Mods/Warframe/AvatarSlideBoostMod")
        );
        assert_eq!(
            starter_original("/Lotus/Upgrades/Mods/Warframe/AvatarSlideBoostModBeginner"),
            None
        );
    }

    #[test]
    fn parts_sets_and_relics() {
        let table = ItemTable::build(&fixtures::catalog());
        let barrel = table.get(BARREL).unwrap();
        assert_eq!(barrel.name, "Braton Prime Barrel");
        assert_eq!(barrel.market_slug.as_deref(), Some("braton_prime_barrel"));
        assert!(barrel.prime);
        assert!(barrel.vault.is_some());
        assert_eq!(
            slug_of(
                &table,
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsComponent"
            )
            .as_deref(),
            Some("trinity_prime_systems_blueprint")
        );
        let ItemKind::Part { set } = barrel.kind else {
            unreachable!();
        };
        assert_eq!(table[set].market_slug.as_deref(), Some("braton_prime_set"));
        assert_eq!(table.variant(BRATON, Variant::Set), Some(&table[set]));
        assert_eq!(slug_of(&table, BRATON).as_deref(), Some("braton_prime"));

        let relic = table
            .get("/Lotus/Types/Game/Projections/T4VoidProjectionESilver")
            .unwrap();
        assert_eq!(relic.name, "Axi A1 Relic (Exceptional)");
        assert_eq!(relic.market_slug.as_deref(), Some("axi_a1_relic"));
        assert_eq!(
            relic.kind,
            ItemKind::Relic {
                refinement: Refinement::Exceptional
            }
        );
        assert!(relic.vault.is_some());
        assert!(table.get("/Lotus/Nope").is_none());
    }

    #[test]
    fn upgrades() {
        let table = ItemTable::build(&fixtures::upgrade_catalog());
        let energize = table.get(ENERGIZE).unwrap();
        assert_eq!(energize.market_slug.as_deref(), Some("arcane_energize"));
        assert!(matches!(
            energize.kind,
            ItemKind::Upgrade {
                max_rank: Some(5),
                ..
            }
        ));
        assert_eq!(
            slug_of(
                &table,
                "/Lotus/Weapons/Tenno/Melee/MeleeTrees/FistCmbThreeMeleeTree"
            )
            .as_deref(),
            Some("gaias_tragedy")
        );
        let maglev = table
            .get("/Lotus/Upgrades/Mods/Warframe/AvatarSlideBoostMod")
            .unwrap();
        assert_eq!(maglev.name, "Maglev");
        assert_eq!(maglev.market_slug.as_deref(), Some("maglev"));
        let core = table
            .get("/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser")
            .unwrap();
        assert_eq!(core.name, "Legendary Core");
        assert_eq!(core.market_slug.as_deref(), Some("legendary_fusion_core"));
        assert_eq!(core.image_name.as_deref(), Some("game/legendary-core.png"));
        assert_eq!(slug_of(&table, HARAS).as_deref(), Some("zid-an-haras"));
    }

    #[test]
    fn rivens_fish_and_imprints() {
        let catalog = Catalog::from_json(
            r#"[
              {"uniqueName":"/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare",
               "name":"Rifle Riven Mod","category":"Mods","type":"Riven Mod","tradable":true},
              {"uniqueName":"/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAItem",
               "name":"Tink","category":"Fish","type":"Fish","tradable":true},
              {"uniqueName":"/Lotus/Types/Game/CatbrowPet/VampireCatbrowPetPowerSuit",
               "name":"Vasca Kavat","category":"Pets","type":"Pets","tradable":false}
            ]"#,
            fixtures::RELICS,
            "[]",
        )
        .unwrap();
        let table = ItemTable::build(&catalog);
        let riven = "/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare";
        assert_eq!(
            slug_of(&table, riven).as_deref(),
            Some("rifle_riven_mod_(veiled)")
        );
        let veiled = table.variant(riven, Variant::Veiled).unwrap();
        assert_eq!(veiled.name, "Rifle Riven Mod (Veiled)");
        assert_eq!(
            veiled.market_slug.as_deref(),
            Some("rifle_riven_mod_veiled")
        );

        let graded = table
            .get("/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAMediumItem")
            .unwrap();
        assert_eq!(graded.name, "Tink (Adorned)");
        assert_eq!(graded.market_slug.as_deref(), Some("tink"));

        let imprint = table
            .variant(
                "/Lotus/Types/Game/CatbrowPet/VampireCatbrowPetPowerSuit",
                Variant::Imprint,
            )
            .unwrap();
        assert_eq!(imprint.name, "Vasca Kavat Imprint");
        assert_eq!(imprint.market_slug.as_deref(), Some("vasca_kavat_imprint"));
    }

    #[test]
    fn market_index() {
        let mut table = ItemTable::build(&fixtures::upgrade_catalog());
        let unknown = "/Lotus/Weapons/Tenno/Melee/MeleeTrees/StaffCmbOneMeleeTree";
        let items = [
            market_item("arcane\u{2019}energize", "Arcane Energize", ENERGIZE, &[]),
            market_item("clashing_forest", "Clashing Forest", unknown, &[]),
            market_item("zid_an_haras", "Zid-An Haras", HARAS, &[]),
            market_item("braton_prime_set", "Braton Prime Set", "", &["set"]),
            market_item(
                "scan_aquatic_lifeforms",
                "Scan Aquatic Lifeforms",
                "",
                &["mod"],
            ),
        ];
        assert_eq!(table.index_market(&items), 4);
        let precept = table.resolve(
            "/Lotus/Types/Sentinels/SentinelPrecepts/LocateCreatures",
            || "Locate Creatures".to_owned(),
        );
        assert_eq!(precept.name, "Scan Aquatic Lifeforms");
        assert_eq!(
            precept.market_slug.as_deref(),
            Some("scan_aquatic_lifeforms")
        );
        assert_eq!(
            slug_of(&table, ENERGIZE).as_deref(),
            Some("arcane\u{2019}energize")
        );
        assert_eq!(
            table
                .by_market_id("arcane\u{2019}energize")
                .map(|record| record.unique_name.as_str()),
            Some(ENERGIZE)
        );
        let listed = table.by_game_ref(unknown).unwrap();
        assert_eq!(listed.name, "Clashing Forest");
        assert_eq!(listed.kind, ItemKind::Other);
        assert_eq!(table.by_market_id("clashing_forest"), Some(listed));
        assert!(table.get(unknown).is_none());
        let resolved = table.resolve(unknown, || "Staff Cmb One Melee Tree".to_owned());
        assert_eq!(resolved.name, "Clashing Forest");
        assert_eq!(resolved.market_slug.as_deref(), Some("clashing_forest"));
        assert_eq!(
            table
                .resolve("/Lotus/Nowhere", || "Nowhere".to_owned())
                .name,
            "Nowhere"
        );
        assert_eq!(slug_of(&table, HARAS).as_deref(), Some("zid-an-haras"));
        assert_eq!(
            table
                .by_market_id("braton_prime_set")
                .map(|record| &record.kind),
            Some(&ItemKind::Other)
        );

        assert_eq!(table.index_market(&[]), 0);
        assert_eq!(
            slug_of(&table, ENERGIZE).as_deref(),
            Some("arcane_energize")
        );
        assert!(table.by_game_ref(unknown).is_none());
        assert!(table.by_market_id("arcane\u{2019}energize").is_none());
    }

    #[test]
    fn set_listing() {
        let mut table = ItemTable::build(&fixtures::catalog());
        table.index_market(&[
            market_item(
                "braton_prime_set",
                "Braton Prime Set",
                BRATON,
                &["set", "weapon"],
            ),
            market_item("barrel_listing", "Braton Prime Barrel", BARREL, &[]),
        ]);
        let set = table.by_market_id("braton_prime_set");
        assert_eq!(set, table.variant(BRATON, Variant::Set));
        assert_eq!(slug_of(&table, BRATON).as_deref(), Some("braton_prime"));
        assert_eq!(
            slug_of(&table, BARREL).as_deref(),
            Some("braton_prime_barrel")
        );
        assert_eq!(table.by_market_id("barrel_listing"), table.get(BARREL));
    }
}
