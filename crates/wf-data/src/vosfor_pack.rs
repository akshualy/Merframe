use std::collections::HashMap;

use serde::Deserialize;

use crate::dictionary::Dictionary;
use crate::error::{DataError, Result};
use crate::item::{Rarity, store_item_to_type};

const VOSFOR: &str = "/Lotus/Types/Items/MiscItems/DistillPoints";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawBoosterPackComponent {
    item: String,
    rarity: Rarity,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBoosterPack {
    name: String,
    icon: Option<String>,
    components: Vec<RawBoosterPackComponent>,
    rarity_weights_per_roll: Vec<HashMap<Rarity, f64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawVendorPrice {
    item_type: String,
    item_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVendorItem {
    store_item: String,
    #[serde(default)]
    item_prices: Vec<RawVendorPrice>,
}

#[derive(Debug, Deserialize)]
struct RawVendor {
    items: Vec<RawVendorItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VosforPackArcane {
    pub unique_name: String,
    pub draw_chance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VosforPack {
    pub name: String,
    pub icon: Option<String>,
    pub vosfor: u32,
    pub draws: u32,
    pub arcanes: Vec<VosforPackArcane>,
}

fn vosfor_pack(raw: &RawBoosterPack, vosfor: u32, dictionary: &Dictionary) -> Option<VosforPack> {
    let draws = u32::try_from(raw.rarity_weights_per_roll.len()).ok()?;
    let mut pool_sizes: HashMap<Rarity, f64> = HashMap::new();
    for component in &raw.components {
        *pool_sizes.entry(component.rarity).or_default() += 1.0;
    }

    let arcanes: Vec<VosforPackArcane> = raw
        .components
        .iter()
        .map(|component| {
            let pool_share: f64 = raw
                .rarity_weights_per_roll
                .iter()
                .filter_map(|weights| weights.get(&component.rarity))
                .sum();
            VosforPackArcane {
                unique_name: component.item.clone(),
                draw_chance: pool_share / f64::from(draws) / pool_sizes[&component.rarity],
            }
        })
        .filter(|arcane| arcane.draw_chance > 0.0)
        .collect();
    if arcanes.is_empty() {
        return None;
    }

    Some(VosforPack {
        name: dictionary.get(&raw.name)?.to_owned(),
        icon: raw.icon.clone(),
        vosfor,
        draws,
        arcanes,
    })
}

pub fn parse_vosfor_packs(
    booster_packs: &str,
    vendors: &str,
    dictionary: &Dictionary,
) -> Result<Vec<VosforPack>> {
    let mut booster_packs: HashMap<String, serde_json::Value> = serde_json::from_str(booster_packs)
        .map_err(|source| DataError::Parse("booster pack", source))?;
    let vendors: HashMap<String, RawVendor> =
        serde_json::from_str(vendors).map_err(|source| DataError::Parse("vendor", source))?;

    let mut packs = Vec::new();
    for item in vendors.values().flat_map(|vendor| &vendor.items) {
        let Some(price) = item
            .item_prices
            .iter()
            .find(|price| price.item_type == VOSFOR)
        else {
            continue;
        };
        let Some(raw) = booster_packs.remove(&store_item_to_type(&item.store_item)) else {
            continue;
        };

        let raw: RawBoosterPack = serde_json::from_value(raw)
            .map_err(|source| DataError::Parse("booster pack", source))?;
        packs.extend(vosfor_pack(&raw, price.item_count, dictionary));
    }
    Ok(packs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOSTER_PACKS: &str = r#"{
        "/Lotus/Types/BoosterPacks/Ostron": {
            "name": "/Lotus/Language/Ostron", "icon": "/Lotus/Icons/CetusPackIcon.png",
            "components": [
                {"Item": "/Lotus/Arcanes/CommonA", "Amount": 1, "Rarity": "COMMON"},
                {"Item": "/Lotus/Arcanes/CommonB", "Amount": 1, "Rarity": "COMMON"},
                {"Item": "/Lotus/Arcanes/Rare", "Amount": 1, "Rarity": "RARE"},
                {"Item": "/Lotus/Arcanes/NeverDrawn", "Amount": 1, "Rarity": "LEGENDARY"}
            ],
            "rarityWeightsPerRoll": [
                {"COMMON": 0.9, "UNCOMMON": 0, "RARE": 0.1, "LEGENDARY": 0},
                {"COMMON": 0.9, "UNCOMMON": 0, "RARE": 0.1, "LEGENDARY": 0},
                {"COMMON": 0.9, "UNCOMMON": 0, "RARE": 0.1, "LEGENDARY": 0}
            ]
        },
        "/Lotus/Types/BoosterPacks/Descendia": {
            "name": "/Lotus/Language/Descendia",
            "components": [{"Item": "/Lotus/Arcanes/Rare", "Amount": 1, "Rarity": "RARE"}],
            "rarityWeightsPerRoll": [{"COMMON": 0, "UNCOMMON": 0, "RARE": 1, "LEGENDARY": 0}]
        }
    }"#;
    const VENDORS: &str = r#"{
        "/Lotus/Vendors/Loid": {"items": [{"storeItem": "/Lotus/StoreItems/Types/Gift"}, {
            "storeItem": "/Lotus/StoreItems/Types/BoosterPacks/Ostron",
            "itemPrices": [{"ItemCount": 200, "ItemType": "/Lotus/Types/Items/MiscItems/DistillPoints"}]
        }]},
        "/Lotus/Vendors/Roathe": {"items": [{
            "storeItem": "/Lotus/StoreItems/Types/BoosterPacks/Descendia",
            "itemPrices": [{"ItemCount": 200, "ItemType": "/Lotus/Types/Items/MiscItems/Ignia"}]
        }]}
    }"#;
    const DICTIONARY: &str = r#"{
        "/Lotus/Language/Ostron": "Ostron Arcane Collection",
        "/Lotus/Language/Descendia": "Descendia Arcane Collection"
    }"#;

    #[test]
    fn packs_sold_for_vosfor() {
        let dictionary = Dictionary::from_json(DICTIONARY).unwrap();
        let packs = parse_vosfor_packs(BOOSTER_PACKS, VENDORS, &dictionary).unwrap();

        let [ostron] = packs.as_slice() else {
            panic!("{packs:?}");
        };
        assert_eq!(ostron.name, "Ostron Arcane Collection");
        assert_eq!((ostron.vosfor, ostron.draws), (200, 3));
        let chances: Vec<(&str, f64)> = ostron
            .arcanes
            .iter()
            .map(|arcane| (arcane.unique_name.as_str(), arcane.draw_chance))
            .collect();
        let expected = [
            ("/Lotus/Arcanes/CommonA", 0.45),
            ("/Lotus/Arcanes/CommonB", 0.45),
            ("/Lotus/Arcanes/Rare", 0.1),
        ];
        assert_eq!(chances.len(), expected.len(), "{chances:?}");
        for ((arcane, chance), (expected_arcane, expected_chance)) in chances.iter().zip(expected) {
            assert_eq!(*arcane, expected_arcane);
            assert!(
                (chance - expected_chance).abs() < 1e-9,
                "{arcane}: {chance}"
            );
        }
    }
}
