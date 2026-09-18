use serde::Serialize;
use wf_data::Relic;
use wf_inventory::Inventory;

use crate::catalog::{Catalog, REFINEMENTS, Stock, refinement_name};
use crate::mastery;

mod expectation;
mod plan;
mod reward_screen;
mod rewards;
mod sources;

pub use expectation::{Best, PerTrace, RefinementValue};
pub(crate) use plan::plan;
pub use plan::{DropLocation, IntactToRadiant, Ownership, RelicMarket, RelicPlan};
pub(crate) use reward_screen::recommend;
pub use reward_screen::{Ranked, RankedComponent, RewardScreen};
pub use rewards::{RewardBreakdown, RewardOwnership};
pub use sources::RelicSource;
pub(crate) use sources::relics_for;

pub const DEFAULT_SQUAD_SIZE: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnedRefinement {
    pub refinement: &'static str,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissingPart {
    pub unique_name: String,
    pub mastery: bool,
}

pub(crate) fn missing_parts(inventory: &Inventory, catalog: &Catalog) -> Vec<MissingPart> {
    let unmastered = mastery::unmastered_types(inventory, catalog);
    let stock = Stock::new(inventory);
    catalog
        .prime_parts()
        .filter(|(_, component)| {
            stock.count(&component.unique_name) < i64::from(component.item_count)
        })
        .map(|(item, component)| MissingPart {
            unique_name: component.unique_name.clone(),
            mastery: unmastered.contains(item.unique_name.as_str()),
        })
        .collect()
}

fn owned_refinements(stock: &Stock, relic: &Relic) -> Vec<OwnedRefinement> {
    REFINEMENTS
        .into_iter()
        .filter_map(|refinement| {
            let unique_name = relic.unique_names.get(&refinement)?;
            let count = stock.count(unique_name);
            if count == 0 {
                return None;
            }
            Some(OwnedRefinement {
                refinement: refinement_name(refinement),
                count,
            })
        })
        .collect()
}

pub(crate) fn void_traces(inventory: &Inventory) -> i64 {
    inventory.counted("/Lotus/Types/Items/MiscItems/VoidTearDrop")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::prices::FixedPrices;

    pub(super) fn prices() -> FixedPrices {
        FixedPrices::new([
            ("styanax_prime_blueprint", 100.0),
            ("alternox_prime_stock", 10.0),
            ("voruna_prime_chassis_blueprint", 20.0),
            ("cedo_prime_barrel", 30.0),
            ("vadarya_prime_stock", 5.0),
            ("trinity_prime_systems_blueprint", 12.0),
            ("braton_prime_stock", 6.0),
            ("trinity_prime_set", 140.0),
        ])
    }

    fn inventory_without_affinity(item_type: &str) -> Inventory {
        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        let entries = value
            .get_mut("XPInfo")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap();
        entries.retain(|entry| {
            entry.get("ItemType").and_then(serde_json::Value::as_str) != Some(item_type)
        });
        Inventory::parse(&value.to_string()).unwrap()
    }

    #[test]
    fn fixture_missing_parts() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let missing = missing_parts(&inventory, &catalog);
        assert_eq!(missing.len(), 8);
        assert_eq!(
            missing_parts(&inventory, &fixtures::with_skins(fixtures::ITEMS)),
            missing
        );
        assert!(
            missing
                .iter()
                .any(|part| part.unique_name.ends_with("TrinityPrimeSystemsComponent"))
        );
    }

    #[test]
    fn mastery_flag() {
        let catalog = fixtures::catalog();
        let mastered = missing_parts(&fixtures::inventory(), &catalog);
        assert!(mastered.iter().all(|part| !part.mastery));

        let unranked = inventory_without_affinity("/Lotus/Weapons/Tenno/Rifle/BratonPrime");
        let missing = missing_parts(&unranked, &catalog);
        let braton: Vec<&MissingPart> = missing
            .iter()
            .filter(|part| part.unique_name.contains("BratonPrime"))
            .collect();
        assert_eq!(braton.len(), 4);
        assert!(braton.iter().all(|part| part.mastery));
        assert!(
            missing
                .iter()
                .filter(|part| part.unique_name.contains("TrinityPrime"))
                .all(|part| !part.mastery)
        );
    }

    #[test]
    fn void_traces_count() {
        assert_eq!(void_traces(&fixtures::inventory()), 6811);
    }
}
