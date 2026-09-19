use serde::Serialize;
use wf_data::Refinement;
use wf_inventory::Inventory;

use crate::catalog::{Catalog, Stock, part_identity};

use super::owned_relic_count;
use super::rewards::rarity_name;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RelicSource {
    pub relic: String,
    pub tier: String,
    pub image_name: Option<String>,
    pub vaulted: bool,
    pub owned: i64,
    pub rarity: &'static str,
    pub chance: f64,
}

pub(crate) fn relics_for(
    part_unique_name: &str,
    inventory: &Inventory,
    catalog: &Catalog,
) -> Vec<RelicSource> {
    let key = part_identity(part_unique_name);
    let stock = Stock::new(inventory);
    let mut sources: Vec<RelicSource> = catalog
        .relics()
        .filter_map(|relic| {
            let reward = relic
                .rewards_for(Refinement::Intact)
                .iter()
                .find(|reward| part_identity(&reward.item_unique_name) == key)?;
            let owned = owned_relic_count(&stock, relic);
            Some(RelicSource {
                relic: relic.name.clone(),
                tier: relic.tier.clone(),
                image_name: relic.image_name.clone(),
                vaulted: relic.vaulted,
                owned,
                rarity: rarity_name(reward.rarity),
                chance: reward.chance,
            })
        })
        .collect();
    sources.sort_by(|left, right| {
        right
            .owned
            .cmp(&left.owned)
            .then_with(|| right.chance.total_cmp(&left.chance))
            .then_with(|| left.relic.cmp(&right.relic))
    });
    sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;

    #[test]
    fn trinity_systems_source() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let sources = relics_for(
            "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsComponent",
            &inventory,
            &catalog,
        );
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].relic, "Axi A1");
        assert_eq!(sources[0].tier, "Axi");
        assert_eq!(sources[0].owned, 0);
        assert!(sources[0].chance > 0.0);
        assert!(sources[0].vaulted);
    }
}
