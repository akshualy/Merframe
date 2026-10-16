use serde::Serialize;
use wf_data::{Refinement, Relic};
use wf_inventory::Inventory;

use crate::account::Account;
use crate::catalog::{Catalog, Stock};
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
pub use reward_screen::{AccountBalance, RankedComponent, RankedReward, RewardScreen};
pub use rewards::{RewardBreakdown, RewardOwnership};
pub use sources::RelicSource;
pub(crate) use sources::relics_for;

pub const DEFAULT_SQUAD_SIZE: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct OwnedRefinement {
    pub refinement: Refinement,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct MissingPart {
    pub unique_name: String,
    pub mastery: bool,
}

pub(crate) fn missing_parts(account: &Account, catalog: &Catalog) -> Vec<MissingPart> {
    let unmastered = mastery::unmastered_types(account, catalog);
    catalog
        .prime_parts()
        .filter(|(_, component)| {
            account.stock.count(&component.unique_name) < i64::from(component.item_count)
        })
        .map(|(item, component)| MissingPart {
            unique_name: component.unique_name.clone(),
            mastery: unmastered.contains(item.unique_name.as_str()),
        })
        .collect()
}

pub(crate) fn wanted_names(parts: &[MissingPart]) -> Vec<String> {
    parts.iter().map(|part| part.unique_name.clone()).collect()
}

fn owned_refinements(stock: &Stock, relic: &Relic) -> Vec<OwnedRefinement> {
    Refinement::ALL
        .into_iter()
        .filter_map(|refinement| {
            let unique_name = relic.unique_names.get(&refinement)?;
            let count = stock.count(unique_name);
            if count == 0 {
                return None;
            }
            Some(OwnedRefinement { refinement, count })
        })
        .collect()
}

fn owned_total(refinements: &[OwnedRefinement]) -> i64 {
    refinements.iter().map(|entry| entry.count).sum()
}

pub(crate) fn owned_relic_count(stock: &Stock, relic: &Relic) -> i64 {
    owned_total(&owned_refinements(stock, relic))
}

pub(crate) fn void_traces(inventory: &Inventory) -> i64 {
    inventory.counted("/Lotus/Types/Items/MiscItems/VoidTearDrop")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::Account;
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

    #[test]
    fn fixture_missing_parts() {
        let account = Account::new(fixtures::inventory());
        let catalog = fixtures::catalog();
        let parts = missing_parts(&account, &catalog);

        assert_eq!(parts.len(), 8);
        assert_eq!(
            missing_parts(&account, &fixtures::with_skins(fixtures::ITEMS)),
            parts
        );
        assert!(
            parts
                .iter()
                .any(|part| part.unique_name == fixtures::TRINITY_PRIME_SYSTEMS)
        );
    }

    #[test]
    fn mastery_flag() {
        let catalog = fixtures::catalog();
        let mastered = missing_parts(&Account::new(fixtures::inventory()), &catalog);
        assert!(mastered.iter().all(|part| !part.mastery));

        let unranked = fixtures::inventory_with(|value| {
            fixtures::drop_entries(value, &["XPInfo"], fixtures::BRATON_PRIME);
        });
        let unmastered = missing_parts(&Account::new(unranked), &catalog);
        let braton: Vec<&MissingPart> = unmastered
            .iter()
            .filter(|part| part.unique_name.contains("BratonPrime"))
            .collect();
        assert_eq!(braton.len(), 4);
        assert!(braton.iter().all(|part| part.mastery));
        assert!(
            unmastered
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
