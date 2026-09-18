use std::collections::HashMap;

use serde::Serialize;

use wf_data::{Component, GameData, Item, Refinement, Relic};

use crate::error::Result;

pub(crate) const RECIPE_PREFIX: &str = "/Lotus/Types/Recipes/";
pub(crate) const MOD_PREFIX: &str = "/Lotus/Upgrades/Mods/";
pub(crate) const ARCANE_PREFIX: &str = "/Lotus/Upgrades/CosmeticEnhancers/";
pub(crate) const RELIC_PREFIX: &str = wf_inventory::RELIC_PREFIX;
pub(crate) const DUCATS_ITEM: &str = "/Lotus/Types/Items/MiscItems/PrimeBucks";
pub(crate) const REQUIEM_MARKER: &str = "Requiem";
pub(crate) const FORMA_ITEM: &str = "/Lotus/Types/Items/MiscItems/Forma";
pub(crate) const FORMA_BLUEPRINT: &str = "/Lotus/Types/Recipes/Components/FormaBlueprint";

const REFINEMENT_SUFFIXES: [(&str, Refinement); 4] = [
    ("Bronze", Refinement::Intact),
    ("Silver", Refinement::Exceptional),
    ("Gold", Refinement::Flawless),
    ("Platinum", Refinement::Radiant),
];

pub(crate) const REFINEMENTS: [Refinement; 4] = [
    Refinement::Intact,
    Refinement::Exceptional,
    Refinement::Flawless,
    Refinement::Radiant,
];

pub struct Catalog {
    data: GameData,
}

impl Catalog {
    pub fn new(data: GameData) -> Self {
        Self { data }
    }

    pub fn from_json(items_json: &str, relics_json: &str) -> Result<Self> {
        Ok(Self::new(GameData::from_json(items_json, relics_json)?))
    }

    pub fn data(&self) -> &GameData {
        &self.data
    }

    pub fn item(&self, unique_name: &str) -> Option<&Item> {
        self.data.by_unique_name(unique_name)
    }

    pub fn component(&self, unique_name: &str) -> Option<(&Item, &Component)> {
        self.data.component_by_unique_name(unique_name)
    }

    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.data.items().iter().filter(|item| !item.is_skin())
    }

    pub fn skins(&self) -> impl Iterator<Item = &Item> {
        self.data.items().iter().filter(|item| item.is_skin())
    }

    pub fn relics(&self) -> impl Iterator<Item = &Relic> {
        self.data.relics().iter()
    }

    pub fn component_for_reward(&self, unique_name: &str) -> Option<(&Item, &Component)> {
        if self.item(unique_name).is_some() {
            return None;
        }
        if let Some(found) = self.component(unique_name) {
            return Some(found);
        }
        let base = unique_name.strip_suffix("Blueprint")?;
        self.component(&format!("{base}Component"))
    }

    pub fn component_for_stock(&self, unique_name: &str) -> Option<(&Item, &Component)> {
        if let Some(found) = self.component_for_reward(unique_name) {
            return Some(found);
        }
        let base = unique_name.strip_suffix("Blueprint")?;
        self.component(base)
    }

    pub fn relic_by_unique_name(&self, unique_name: &str) -> Option<(&Relic, Refinement)> {
        self.data.relic_by_unique_name(unique_name)
    }

    pub fn icon_for(&self, unique_name: &str) -> Option<String> {
        if let Some(item) = self.item(unique_name) {
            return item.image_name.clone();
        }
        let (item, component) = self.component_for_stock(unique_name)?;
        component_image(item, component)
    }

    pub fn prime_parts(&self) -> impl Iterator<Item = (&Item, &Component)> {
        self.items().filter(|item| is_prime(item)).flat_map(|item| {
            item.components
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter(|component| is_part(component))
                .map(move |component| (item, component))
        })
    }
}

pub(crate) fn owned_count(inventory: &wf_inventory::Inventory, unique_name: &str) -> i64 {
    resolve_count(unique_name, |item_type| inventory.counted(item_type))
}

pub(crate) struct Stock<'a> {
    counts: HashMap<&'a str, i64>,
}

impl<'a> Stock<'a> {
    pub fn new(inventory: &'a wf_inventory::Inventory) -> Self {
        Self {
            counts: inventory.counted_index(),
        }
    }

    pub fn count(&self, unique_name: &str) -> i64 {
        resolve_count(unique_name, |item_type| {
            self.counts.get(item_type).copied().unwrap_or(0)
        })
    }
}

fn resolve_count(unique_name: &str, counted: impl Fn(&str) -> i64) -> i64 {
    if unique_name == FORMA_BLUEPRINT {
        return counted(FORMA_ITEM) + counted(FORMA_BLUEPRINT);
    }
    let direct = counted(unique_name);
    if builds_from_its_own_blueprint(unique_name) {
        return direct + counted(&format!("{unique_name}Blueprint"));
    }
    if direct > 0 {
        return direct;
    }
    if let Some(base) = unique_name.strip_suffix("Component") {
        return counted(&format!("{base}Blueprint"));
    }
    if let Some(base) = unique_name.strip_suffix("Blueprint") {
        return counted(&format!("{base}Component"));
    }
    counted(&format!("{unique_name}Blueprint"))
}

pub(crate) fn builds_from_its_own_blueprint(unique_name: &str) -> bool {
    unique_name.contains("/WeaponParts/")
        && !unique_name.contains("Prime")
        && !unique_name.ends_with("Blueprint")
        && !unique_name.ends_with("Component")
}

pub(crate) fn part_identity(unique_name: &str) -> &str {
    unique_name
        .strip_suffix("Blueprint")
        .or_else(|| unique_name.strip_suffix("Component"))
        .unwrap_or(unique_name)
}

pub(crate) fn component_image(item: &Item, component: &Component) -> Option<String> {
    match component.image_name.as_deref() {
        Some(image) if image != "blueprint.png" => Some(image.to_owned()),
        _ => item
            .image_name
            .clone()
            .or_else(|| component.image_name.clone()),
    }
}

pub(crate) fn is_prime(item: &Item) -> bool {
    item.name.contains("Prime")
}

pub(crate) fn names_a_prime(name: &str) -> bool {
    name.to_lowercase().contains("prime")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultStatus {
    Vaulted,
    Available,
    Unknown,
}

impl From<Option<bool>> for VaultStatus {
    fn from(vaulted: Option<bool>) -> Self {
        match vaulted {
            Some(true) => Self::Vaulted,
            Some(false) => Self::Available,
            None => Self::Unknown,
        }
    }
}

pub(crate) fn vault_status(name: &str, vaulted: Option<bool>) -> Option<VaultStatus> {
    names_a_prime(name).then(|| VaultStatus::from(vaulted))
}

pub(crate) fn is_part(component: &Component) -> bool {
    component.unique_name.starts_with(RECIPE_PREFIX)
}

pub(crate) fn is_resource(component: &Component) -> bool {
    component
        .unique_name
        .starts_with("/Lotus/Types/Items/MiscItems/")
}

pub(crate) fn part_market_slug(item: &Item, component: &Component) -> String {
    let name = part_name(item, component);
    if component.unique_name.ends_with("Component") {
        crate::prices::market_slug(&format!("{name} Blueprint"))
    } else {
        crate::prices::market_slug(&name)
    }
}

pub(crate) fn part_name(item: &Item, component: &Component) -> String {
    let set = item.name.split(' ').next().unwrap_or(&item.name);
    if component.name.starts_with(set) {
        component.name.clone()
    } else {
        format!("{} {}", item.name, component.name)
    }
}

pub(crate) fn refinement_from_unique_name(unique_name: &str) -> Option<Refinement> {
    REFINEMENT_SUFFIXES
        .into_iter()
        .find(|(suffix, _)| unique_name.ends_with(suffix))
        .map(|(_, refinement)| refinement)
}

pub(crate) fn refinement_name(refinement: Refinement) -> &'static str {
    match refinement {
        Refinement::Intact => "Intact",
        Refinement::Exceptional => "Exceptional",
        Refinement::Flawless => "Flawless",
        Refinement::Radiant => "Radiant",
    }
}

pub(crate) fn item_name(catalog: &Catalog, unique_name: &str) -> String {
    match catalog.item(unique_name) {
        Some(item) => item.name.clone(),
        None => display_name_from_path(unique_name),
    }
}

pub(crate) fn display_name_from_path(path: &str) -> String {
    let tail = path.rsplit_once('/').map_or(path, |(_, tail)| tail);
    let mut out = String::with_capacity(tail.len() + 8);
    for (index, character) in tail.char_indices() {
        if index > 0 && character.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(character);
    }
    out
}

#[cfg(test)]
pub mod fixtures {
    use super::Catalog;

    pub const ITEMS: &str = include_str!("../../wf-data/tests/fixtures/items.json");
    pub const RELICS: &str = include_str!("../../wf-data/tests/fixtures/relics.json");
    pub const INVENTORY: &str = include_str!("../../../fixtures/inventory.json");
    pub const MASTERY_ITEMS: &str = include_str!("../../../fixtures/mastery_items.json");
    pub const FOUNDRY_ITEMS: &str = include_str!("../../wf-data/tests/fixtures/foundry_items.json");
    pub const RIVEN_ITEMS: &str = include_str!("../../wf-data/tests/fixtures/riven_items.json");
    pub const SKINS: &str = include_str!("../../wf-data/tests/fixtures/skins.json");

    pub fn catalog() -> Catalog {
        Catalog::from_json(ITEMS, RELICS).unwrap()
    }

    pub fn with_skins(items_json: &str) -> Catalog {
        let mut merged: Vec<serde_json::Value> = serde_json::from_str(items_json).unwrap();
        let mut skins: Vec<serde_json::Value> = serde_json::from_str(SKINS).unwrap();
        merged.append(&mut skins);
        let merged = serde_json::to_string(&merged).unwrap();
        Catalog::from_json(&merged, RELICS).unwrap()
    }

    pub fn foundry_catalog() -> Catalog {
        Catalog::from_json(FOUNDRY_ITEMS, RELICS).unwrap()
    }

    pub fn inventory_stocked(
        equipment: &[(&str, &str)],
        counted: &[(&str, &str, i64)],
    ) -> wf_inventory::Inventory {
        let mut value: serde_json::Value = serde_json::from_str(INVENTORY).unwrap();
        let object = value.as_object_mut().unwrap();
        for key in [
            "Suits",
            "LongGuns",
            "Pistols",
            "Melee",
            "MiscItems",
            "Recipes",
        ] {
            object.insert(key.to_owned(), serde_json::json!([]));
        }
        for (index, (category, item_type)) in equipment.iter().enumerate() {
            let entries = object
                .entry((*category).to_owned())
                .or_insert_with(|| serde_json::json!([]))
                .as_array_mut()
                .unwrap();
            entries.push(serde_json::json!({
                "ItemType": item_type,
                "ItemId": { "$oid": format!("{index:024x}") },
                "XP": 0,
            }));
        }
        for (category, item_type, count) in counted {
            let entries = object
                .entry((*category).to_owned())
                .or_insert_with(|| serde_json::json!([]))
                .as_array_mut()
                .unwrap();
            entries.push(serde_json::json!({ "ItemType": item_type, "ItemCount": count }));
        }
        wf_inventory::Inventory::parse(&value.to_string()).unwrap()
    }

    pub fn mastery_catalog() -> Catalog {
        Catalog::from_json(MASTERY_ITEMS, RELICS).unwrap()
    }

    pub fn inventory() -> wf_inventory::Inventory {
        wf_inventory::Inventory::parse(INVENTORY).unwrap()
    }

    pub fn inventory_owning(counted: &[(&str, i64)]) -> wf_inventory::Inventory {
        let mut value: serde_json::Value = serde_json::from_str(INVENTORY).unwrap();
        let entries = value
            .get_mut("MiscItems")
            .and_then(serde_json::Value::as_array_mut)
            .expect("misc items");
        for (item_type, count) in counted {
            entries.push(serde_json::json!({ "ItemType": item_type, "ItemCount": count }));
        }
        wf_inventory::Inventory::parse(&value.to_string()).unwrap()
    }

    pub fn inventory_without_equipment(item_type: &str) -> wf_inventory::Inventory {
        let mut value: serde_json::Value = serde_json::from_str(INVENTORY).unwrap();
        for key in ["Suits", "LongGuns", "Pistols", "Melee"] {
            if let Some(entries) = value.get_mut(key).and_then(serde_json::Value::as_array_mut) {
                entries.retain(|entry| {
                    entry.get("ItemType").and_then(serde_json::Value::as_str) != Some(item_type)
                });
            }
        }
        wf_inventory::Inventory::parse(&value.to_string()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wf_data::Refinement;

    #[test]
    fn fixture_counts() {
        let catalog = fixtures::catalog();
        assert_eq!(catalog.items().count(), 3);
        assert_eq!(catalog.relics().count(), 2);
        assert_eq!(catalog.skins().count(), 0);
    }

    #[test]
    fn skins_kept_apart() {
        let catalog = fixtures::with_skins(fixtures::ITEMS);
        assert_eq!(catalog.items().count(), 3);
        assert_eq!(catalog.skins().count(), 13);
        assert!(catalog.items().all(|item| !item.is_skin()));
        assert!(
            catalog
                .skins()
                .any(|skin| skin.name == "Kavasa Prime Kubrow Collar")
        );

        let helmet = catalog
            .item("/Lotus/Upgrades/Skins/Brawler/BrawlerAltHelmet")
            .expect("helmet skin");
        assert_eq!(helmet.name, "Atlas Tartarus Helmet");
        assert_eq!(
            catalog.icon_for("/Lotus/Upgrades/Skins/Brawler/BrawlerAltHelmet"),
            Some("BrawlerAltHelmet.png".to_owned())
        );
        assert!(helmet.components.is_some());
        assert!(!helmet.masterable());
        assert!(helmet.warframe_market.is_none());
    }

    #[test]
    fn relic_by_unique_name() {
        let catalog = fixtures::catalog();
        let (relic, refinement) = catalog
            .relic_by_unique_name(
                "/Lotus/Types/Game/Projections/T4VoidProjectionStyanaxPrimeABronze",
            )
            .unwrap();
        assert_eq!(relic.name, "Axi A21");
        assert_eq!(refinement, Refinement::Intact);
        assert!(!relic.vaulted);
    }

    #[test]
    fn refinement_suffixes() {
        assert_eq!(
            refinement_from_unique_name("T1VoidProjectionWispPrimeABronze"),
            Some(Refinement::Intact)
        );
        assert_eq!(
            refinement_from_unique_name("T1VoidProjectionWispPrimeASilver"),
            Some(Refinement::Exceptional)
        );
        assert_eq!(
            refinement_from_unique_name("T1VoidProjectionWispPrimeAGold"),
            Some(Refinement::Flawless)
        );
        assert_eq!(
            refinement_from_unique_name("T1VoidProjectionWispPrimeAPlatinum"),
            Some(Refinement::Radiant)
        );
        assert_eq!(refinement_from_unique_name("SomethingElse"), None);
    }

    #[test]
    fn prime_parts_skip_resources() {
        let catalog = fixtures::catalog();
        let parts: Vec<String> = catalog
            .prime_parts()
            .map(|(item, component)| part_name(item, component))
            .collect();
        assert!(parts.contains(&"Trinity Prime Systems".to_owned()));
        assert!(parts.contains(&"Braton Prime Barrel".to_owned()));
        assert!(!parts.iter().any(|n| n.contains("Orokin Cell")));
        assert!(!parts.iter().any(|n| n.starts_with("Excalibur")));

        let with_skins = fixtures::with_skins(fixtures::ITEMS);
        let skinned: Vec<String> = with_skins
            .prime_parts()
            .map(|(item, component)| part_name(item, component))
            .collect();
        assert_eq!(skinned, parts);
    }

    #[test]
    fn stock_matches_owned_count() {
        let inventory = fixtures::inventory_owning(&[
            (
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint",
                2,
            ),
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                3,
            ),
        ]);
        let stock = Stock::new(&inventory);
        for unique_name in [
            "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsComponent",
            "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint",
            "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
            "/Lotus/Types/Items/MiscItems/Ferrite",
            "/Lotus/Types/Items/MiscItems/OrokinCell",
            "/Lotus/Types/Items/MiscItems/DoesNotExist",
        ] {
            assert_eq!(
                stock.count(unique_name),
                owned_count(&inventory, unique_name),
                "{unique_name}"
            );
        }
        assert_eq!(
            stock.count("/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsComponent"),
            2
        );
    }

    #[test]
    fn item_doubling_as_ingredient() {
        let items = r#"[
            {"uniqueName": "/Lotus/Types/Items/MiscItems/Forma", "name": "Forma",
             "category": "Misc", "type": "Misc", "tradable": false, "imageName": "forma.png"},
            {"uniqueName": "/Lotus/Types/Items/MiscItems/FormaUmbra", "name": "Umbra Forma",
             "category": "Misc", "type": "Misc", "tradable": false,
             "components": [
                {"uniqueName": "/Lotus/Types/Items/MiscItems/Forma", "name": "Forma",
                 "itemCount": 1, "tradable": false}
             ]}
        ]"#;
        let catalog = Catalog::from_json(items, fixtures::RELICS).unwrap();
        let forma = "/Lotus/Types/Items/MiscItems/Forma";
        assert!(catalog.component(forma).is_some());
        assert!(catalog.component_for_reward(forma).is_none());
        assert_eq!(item_name(&catalog, forma), "Forma");
        assert_eq!(catalog.icon_for(forma).as_deref(), Some("forma.png"));
    }

    #[test]
    fn forma_stock() {
        let inventory = fixtures::inventory();
        let stock = Stock::new(&inventory);
        assert_eq!(inventory.counted(FORMA_ITEM), 65);
        assert_eq!(inventory.counted(FORMA_BLUEPRINT), 25);
        assert_eq!(stock.count(FORMA_BLUEPRINT), 90);
        assert_eq!(owned_count(&inventory, FORMA_BLUEPRINT), 90);
        assert_eq!(stock.count(FORMA_ITEM), 65);
        assert_eq!(
            stock.count("/Lotus/Types/Recipes/Components/FormaAuraBlueprint"),
            53
        );
    }

    #[test]
    fn non_prime_part_adds_blueprints() {
        let inventory = fixtures::inventory_owning(&[
            ("/Lotus/Types/Recipes/Weapons/WeaponParts/BoltorBarrel", 2),
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BoltorBarrelBlueprint",
                3,
            ),
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                4,
            ),
            (
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrelBlueprint",
                5,
            ),
        ]);
        let stock = Stock::new(&inventory);
        assert_eq!(
            stock.count("/Lotus/Types/Recipes/Weapons/WeaponParts/BoltorBarrel"),
            5
        );
        assert_eq!(
            stock.count("/Lotus/Types/Recipes/Weapons/WeaponParts/BoltorBarrelBlueprint"),
            3
        );
        assert_eq!(
            stock.count("/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel"),
            4
        );
    }

    #[test]
    fn generic_blueprint_image() {
        let catalog = fixtures::catalog();
        let (item, blueprint) = catalog
            .component("/Lotus/Types/Recipes/Weapons/BratonPrimeBlueprint")
            .unwrap();
        assert_eq!(blueprint.image_name.as_deref(), Some("blueprint.png"));
        assert_eq!(component_image(item, blueprint), item.image_name);
        let (item, barrel) = catalog
            .component("/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel")
            .unwrap();
        assert_eq!(component_image(item, barrel), barrel.image_name);
    }

    #[test]
    fn icon_lookup_order() {
        let catalog = fixtures::catalog();
        assert_eq!(
            catalog.icon_for("/Lotus/Weapons/Tenno/Rifle/BratonPrime"),
            catalog
                .item("/Lotus/Weapons/Tenno/Rifle/BratonPrime")
                .and_then(|item| item.image_name.clone())
        );
        assert_eq!(
            catalog.icon_for("/Lotus/Types/Recipes/Weapons/BratonPrimeBlueprint"),
            catalog
                .item("/Lotus/Weapons/Tenno/Rifle/BratonPrime")
                .and_then(|item| item.image_name.clone())
        );
        assert_eq!(catalog.icon_for("/Lotus/Nope"), None);
    }

    #[test]
    fn camel_case_display_name() {
        assert_eq!(
            display_name_from_path("/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod"),
            "Avatar Shield Max Mod"
        );
    }
}
