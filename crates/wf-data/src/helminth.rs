use crate::embedded::{HELMINTH_ABILITIES, lookup};
use crate::item::Item;

pub fn base_warframe_name(name: &str) -> &str {
    name.strip_suffix(" Prime")
        .or_else(|| name.strip_suffix(" Umbra"))
        .unwrap_or(name)
}

pub fn helminth_ability(item: &Item) -> Option<&'static str> {
    if !item.is_warframe() {
        return None;
    }
    lookup(HELMINTH_ABILITIES, base_warframe_name(&item.name)).copied()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const MASTERY_ITEMS: &str = include_str!("../../../fixtures/mastery_items.json");

    fn fixture_warframes() -> Vec<Item> {
        serde_json::from_str::<Vec<Item>>(MASTERY_ITEMS)
            .unwrap()
            .into_iter()
            .filter(Item::is_warframe)
            .collect()
    }

    #[test]
    fn every_warframe_has_ability() {
        let warframes = fixture_warframes();
        assert_eq!(warframes.len(), 117);
        for warframe in &warframes {
            assert!(
                helminth_ability(warframe).is_some(),
                "{} has no subsumable ability",
                warframe.name
            );
        }
    }

    #[test]
    fn table_covers_base_warframes() {
        let bases: HashSet<String> = fixture_warframes()
            .iter()
            .map(|warframe| base_warframe_name(&warframe.name).to_owned())
            .collect();
        assert_eq!(bases.len(), 65);
        assert!(HELMINTH_ABILITIES.len() >= bases.len());
    }

    #[test]
    fn variants_share_base_ability() {
        let warframes = fixture_warframes();
        let ability = |name: &str| {
            warframes
                .iter()
                .find(|warframe| warframe.name == name)
                .and_then(helminth_ability)
        };
        assert_eq!(ability("Excalibur"), Some("Radial Blind"));
        assert_eq!(ability("Excalibur Prime"), Some("Radial Blind"));
        assert_eq!(ability("Excalibur Umbra"), Some("Radial Blind"));
        assert_eq!(ability("Sevagoth Prime"), Some("Gloom"));
    }

    #[test]
    fn weapon_has_no_ability() {
        let items: Vec<Item> = serde_json::from_str(MASTERY_ITEMS).unwrap();
        let braton = items.iter().find(|item| item.name == "Braton").unwrap();
        assert_eq!(helminth_ability(braton), None);
    }
}
