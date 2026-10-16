use std::collections::HashSet;

use serde::Serialize;
use wf_data::{Refinement, Relic};

use crate::catalog::REQUIEM_MARKER;
use crate::identity::part_identity;
use crate::view::View;

use super::expectation::{Best, RefinementValue, best_refinement, refinement_values};
use super::rewards::{RewardBreakdown, reward_breakdown};
use super::{OwnedRefinement, owned_refinements, owned_total};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct DropLocation {
    pub location: String,
    pub chance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RelicMarket {
    pub slug: String,
    pub sell: Option<f64>,
    pub buy: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct Ownership {
    pub owned: i64,
    pub by_refinement: Vec<OwnedRefinement>,
    pub all_sets_owned: bool,
    pub all_items_owned: bool,
    pub missing_items: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct IntactToRadiant {
    pub plat: f64,
    pub ducats: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RelicPlan {
    pub relic: String,
    pub unique_name: String,
    pub tier: String,
    pub image_name: Option<String>,
    pub market: Option<RelicMarket>,
    pub vaulted: bool,
    pub ownership: Ownership,
    pub rewards: Vec<RewardBreakdown>,
    pub values: Vec<RefinementValue>,
    pub best: Best,
    pub intact_to_radiant: IntactToRadiant,
    pub favourite: bool,
    pub favourite_rewards: usize,
    pub drops: Vec<DropLocation>,
    pub drop_locations: usize,
}

pub(crate) fn plan(
    view: &View,
    wanted: &[String],
    squad_size: u32,
    only_owned: bool,
    tier: Option<&str>,
) -> Vec<RelicPlan> {
    let View {
        account, catalog, ..
    } = *view;
    let squad_size = squad_size.clamp(1, 4);
    let wanted_keys: HashSet<&str> = wanted.iter().map(|name| part_identity(name)).collect();
    let mut plans: Vec<RelicPlan> = catalog
        .relics()
        .filter(|relic| relic.tradable && !relic.name.contains(REQUIEM_MARKER))
        .filter(|relic| tier.is_none_or(|tier| relic.tier == tier))
        .filter_map(|relic| {
            let owned_by_refinement = owned_refinements(&account.stock, relic);
            if only_owned && owned_total(&owned_by_refinement) == 0 {
                return None;
            }
            let rewards: Vec<RewardBreakdown> = relic
                .rewards_for(Refinement::Intact)
                .iter()
                .map(|reward| reward_breakdown(view, reward))
                .collect();
            Some(relic_plan(
                view,
                relic,
                owned_by_refinement,
                rewards,
                &wanted_keys,
                squad_size,
            ))
        })
        .collect();
    plans.sort_by(|left, right| {
        right
            .best
            .wanted_chance
            .total_cmp(&left.best.wanted_chance)
            .then_with(|| right.best.plat.total_cmp(&left.best.plat))
            .then_with(|| left.relic.cmp(&right.relic))
            .then_with(|| left.unique_name.cmp(&right.unique_name))
    });
    plans
}

fn relic_plan(
    view: &View,
    relic: &Relic,
    owned_by_refinement: Vec<OwnedRefinement>,
    rewards: Vec<RewardBreakdown>,
    wanted_keys: &HashSet<&str>,
    squad_size: u32,
) -> RelicPlan {
    let View {
        prices, favourites, ..
    } = *view;
    let owned = owned_total(&owned_by_refinement);
    let wanted_rewards: Vec<bool> = rewards
        .iter()
        .map(|reward| wanted_keys.contains(part_identity(&reward.unique_name)))
        .collect();
    let values = refinement_values(relic, &rewards, &wanted_rewards, squad_size).to_vec();
    RelicPlan {
        relic: relic.name.clone(),
        unique_name: relic
            .unique_names
            .get(&Refinement::Intact)
            .cloned()
            .unwrap_or_default(),
        tier: relic.tier.clone(),
        image_name: relic.image_name.clone(),
        market: relic.market_info.as_ref().map(|info| RelicMarket {
            sell: prices.plat(&info.url_name),
            buy: prices.buy_plat(&info.url_name),
            slug: info.url_name.clone(),
        }),
        vaulted: relic.vaulted,
        ownership: Ownership {
            owned,
            by_refinement: owned_by_refinement,
            all_sets_owned: rewards
                .iter()
                .all(|reward| !reward.ownership.needed_for_set),
            all_items_owned: rewards
                .iter()
                .all(|reward| reward.ownership.parent_owned || reward.forma),
            missing_items: rewards
                .iter()
                .filter(|reward| {
                    reward.ownership.needed_for_set
                        && !reward.ownership.parent_owned
                        && !reward.forma
                })
                .count(),
        },
        best: best_refinement(&values),
        intact_to_radiant: IntactToRadiant {
            plat: values[3].expected_plat - values[0].expected_plat,
            ducats: values[3].expected_ducats - values[0].expected_ducats,
        },
        favourite: favourites.any(relic.unique_names.values().map(String::as_str)),
        favourite_rewards: rewards.iter().filter(|reward| reward.favourite).count(),
        drops: relic
            .drops
            .iter()
            .take(6)
            .map(|drop| DropLocation {
                location: mission_node_label(&drop.location),
                chance: drop.chance,
            })
            .collect(),
        drop_locations: relic.drops.len(),
        rewards,
        values,
    }
}

fn mission_node_label(location: &str) -> String {
    match location.split_once('/') {
        Some((planet, node)) => format!("{planet}, {node}"),
        None => location.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::rewards::rarity_name;
    use super::super::tests::prices;
    use super::super::{DEFAULT_SQUAD_SIZE, missing_parts, wanted_names};
    use super::*;
    use crate::catalog::fixtures;
    use crate::prices::FixedPrices;
    use crate::view::Fixture;
    use wf_inventory::Inventory;

    const AXI_A1_INTACT: &str = "/Lotus/Types/Game/Projections/T4VoidProjectionEBronze";

    fn fixture(inventory: Inventory) -> Fixture {
        Fixture::new(fixtures::catalog(), inventory).with_prices(prices())
    }

    fn relic<'a>(plans: &'a [RelicPlan], name: &str) -> &'a RelicPlan {
        plans
            .iter()
            .find(|plan| plan.relic == name)
            .expect("relic in the plan")
    }

    fn inventory_owning_axi_a1() -> Inventory {
        fixtures::inventory_owning(&[(AXI_A1_INTACT, 3)])
    }

    fn inventory_owning_axi_a1_without(item_type: &str) -> Inventory {
        fixtures::inventory_with(|value| {
            fixtures::drop_entries(
                value,
                &["Suits", "LongGuns", "Pistols", "Melee", "XPInfo"],
                item_type,
            );
            fixtures::push_counted(value, "MiscItems", AXI_A1_INTACT, 3);
        })
    }

    #[test]
    fn missing_items_and_parent_ownership() {
        let mastered = plan(
            &fixture(inventory_owning_axi_a1()).view(),
            &[],
            1,
            true,
            None,
        );
        let axi = relic(&mastered, "Axi A1");
        let short = axi
            .rewards
            .iter()
            .filter(|reward| reward.ownership.needed_for_set)
            .count();
        assert_eq!(short, 2);
        assert!(
            axi.rewards
                .iter()
                .filter(|reward| reward.ownership.needed_for_set)
                .all(|reward| reward.ownership.parent_owned)
        );
        assert_eq!(axi.ownership.missing_items, 0);

        let unmastered = plan(
            &fixture(inventory_owning_axi_a1_without(fixtures::BRATON_PRIME)).view(),
            &[],
            1,
            true,
            None,
        );
        let axi = relic(&unmastered, "Axi A1");
        let braton = axi
            .rewards
            .iter()
            .find(|reward| reward.name == "Braton Prime Stock")
            .expect("Braton Prime Stock");
        assert_eq!(
            braton.component.as_deref(),
            Some(fixtures::BRATON_PRIME_STOCK)
        );
        assert!(braton.ownership.needed_for_set);
        assert!(!braton.ownership.parent_owned);
        assert_eq!(axi.ownership.missing_items, 1);
    }

    #[test]
    fn forma_never_blocks_all_owned() {
        let plans = plan(
            &fixture(inventory_owning_axi_a1()).view(),
            &[],
            1,
            true,
            None,
        );
        for relic in &plans {
            assert!(!relic.ownership.all_items_owned);
        }

        let a21 = relic(&plans, "Axi A21");
        let forma: Vec<&RewardBreakdown> =
            a21.rewards.iter().filter(|reward| reward.forma).collect();
        assert_eq!(forma.len(), 1);
        assert_eq!(forma[0].name, "Forma Blueprint");
        assert_eq!(a21.ownership.missing_items, 0);
    }

    #[test]
    fn ducats_per_trace() {
        let plans = plan(
            &fixture(inventory_owning_axi_a1()).view(),
            &[],
            1,
            true,
            None,
        );
        let axi = relic(&plans, "Axi A1");
        assert_eq!(axi.values[0].ducats_per_trace, None);
        assert!((axi.values[0].expected_ducats - 10.132).abs() < 1e-9);
        for (value, want) in axi.values[1..].iter().zip([-0.032, -0.04264, -0.03464]) {
            assert!((value.ducats_per_trace.unwrap() - want).abs() < 1e-9);
        }
        let best = axi.best.ducats_per_trace.unwrap();
        assert!(
            axi.values
                .iter()
                .filter_map(|value| value.ducats_per_trace)
                .all(|per_trace| per_trace <= best.value)
        );
    }

    #[test]
    fn favourite_relic_and_rewards() {
        let mut fixture = fixture(fixtures::inventory());
        let plain = plan(&fixture.view(), &[], 1, false, None);
        assert!(
            plain
                .iter()
                .all(|entry| !entry.favourite && entry.favourite_rewards == 0)
        );
        assert_eq!(
            relic(&plain, "Axi A1").unique_name,
            "/Lotus/Types/Game/Projections/T4VoidProjectionEBronze"
        );

        fixture.favourites = [
            "/Lotus/Types/Game/Projections/T4VoidProjectionEPlatinum",
            "/Lotus/Types/Recipes/Weapons/WeaponParts/AkstilettoPrimeBarrel",
            "/Lotus/Types/Recipes/Weapons/WeaponParts/AlternoxPrimeStock",
            "/Lotus/Types/Recipes/Weapons/WeaponParts/CedoPrimeBarrel",
        ]
        .into_iter()
        .collect();
        let marked = plan(&fixture.view(), &[], 1, false, None);
        let a1 = relic(&marked, "Axi A1");
        let a21 = relic(&marked, "Axi A21");
        assert!(
            a1.favourite,
            "a refined stack of the family marks the whole relic"
        );
        assert_eq!(a1.favourite_rewards, 1);
        assert!(!a21.favourite);
        assert_eq!(a21.favourite_rewards, 2);
        assert!(
            a1.rewards
                .iter()
                .any(|reward| reward.favourite && reward.name.contains("Akstiletto"))
        );
    }

    #[test]
    fn every_relic_in_a_fixed_order() {
        let fixture = fixture(fixtures::inventory());
        let order = || -> Vec<String> {
            plan(&fixture.view(), &[], DEFAULT_SQUAD_SIZE, false, None)
                .into_iter()
                .map(|entry| entry.unique_name)
                .collect()
        };
        let first = order();
        assert!(first.len() > 1);
        assert_eq!(first, order());
    }

    #[test]
    fn sort_by_favourite_rewards() {
        let fixture = Fixture {
            favourites: [
                "/Lotus/Types/Game/Projections/T4VoidProjectionEBronze",
                "/Lotus/Types/Recipes/Weapons/WeaponParts/AlternoxPrimeStock",
                "/Lotus/Types/Recipes/Weapons/WeaponParts/CedoPrimeBarrel",
            ]
            .into_iter()
            .collect(),
            ..fixture(fixtures::inventory())
        };
        let mut plans = plan(&fixture.view(), &[], 1, false, None);
        plans.sort_by_key(|plan| std::cmp::Reverse(plan.favourite_rewards));
        assert_eq!(plans[0].relic, "Axi A21");
        assert!(
            !plans[0].favourite,
            "the ordering counts favourite rewards, not the starred relic"
        );
        assert_eq!(plans.last().unwrap().favourite_rewards, 0);
    }

    #[test]
    fn relic_market_quote() {
        let fixture = Fixture::new(fixtures::catalog(), fixtures::inventory()).with_prices(
            FixedPrices::new([("axi_a21_relic", 12.0)]).with_buy([("axi_a21_relic", 7.0)]),
        );
        let plans = plan(&fixture.view(), &[], 1, true, None);
        let axi = &plans[0];
        assert_eq!(
            axi.market,
            Some(RelicMarket {
                slug: "axi_a21_relic".to_owned(),
                sell: Some(12.0),
                buy: Some(7.0),
            })
        );
    }

    #[test]
    fn drop_locations() {
        let fixture = fixture(fixtures::inventory());
        let plans = plan(&fixture.view(), &[], 1, true, None);
        let axi = &plans[0];
        assert_eq!(axi.relic, "Axi A21");
        assert!(!axi.vaulted);
        assert_eq!(axi.drops.len(), 6);
        assert!(axi.drop_locations > 6);
        assert!(
            axi.drops
                .windows(2)
                .all(|pair| pair[0].chance >= pair[1].chance)
        );
        assert!(axi.drops[0].location.contains("Interception"));
        assert_eq!(
            axi.drops[0].location,
            "Eris, Phalan (Interception), Rotation B"
        );
        assert_eq!(
            mission_node_label("Void/Mot (Survival), Rotation C"),
            "Void, Mot (Survival), Rotation C"
        );
        assert_eq!(mission_node_label("Baro Ki'Teer"), "Baro Ki'Teer");
    }

    #[test]
    fn unowned_relics_included() {
        let fixture = fixture(fixtures::inventory());
        let owned = plan(&fixture.view(), &[], 1, true, None);
        assert_eq!(owned.len(), 1);

        let everything = plan(&fixture.view(), &[], 1, false, None);
        assert_eq!(everything.len(), 2);
        let unowned = relic(&everything, "Axi A1");
        assert_eq!(unowned.ownership.owned, 0);
        assert!(unowned.ownership.by_refinement.is_empty());
        assert!(unowned.values.iter().all(|value| value.chances.len() == 6));
        assert!(
            everything
                .iter()
                .all(|relic| !relic.relic.contains(REQUIEM_MARKER))
        );
    }

    #[test]
    fn owned_relic_values() {
        let fixture = fixture(fixtures::inventory());
        let wanted = wanted_names(&missing_parts(&fixture.account, &fixture.catalog));
        let plans = plan(&fixture.view(), &wanted, 1, true, None);

        assert_eq!(plans.len(), 1);
        let axi = &plans[0];
        assert_eq!(axi.relic, "Axi A21");
        assert_eq!(axi.tier, "Axi");
        assert_eq!(axi.ownership.owned, 22);
        assert_eq!(axi.ownership.by_refinement.len(), 2);
        assert_eq!(
            axi.ownership.by_refinement[0].refinement,
            Refinement::Intact
        );
        assert_eq!(axi.values.len(), 4);

        let intact = &axi.values[0];
        assert_eq!(intact.refinement, Refinement::Intact);
        assert!((intact.expected_plat - 32.2965).abs() < 1e-9);
        let radiant = &axi.values[3];
        assert_eq!(radiant.refinement, Refinement::Radiant);
        assert!((radiant.expected_plat - 28.5035).abs() < 1e-9);
        assert!((axi.best.plat - intact.expected_plat).abs() < f64::EPSILON);
    }

    #[test]
    fn pricy_rare_favours_radiant() {
        let fixture = Fixture::new(fixtures::catalog(), fixtures::inventory())
            .with_prices(FixedPrices::new([("alternox_prime_stock", 1000.0)]));
        let plans = plan(&fixture.view(), &[], 1, true, None);
        let axi = &plans[0];
        assert!((axi.values[0].expected_plat - 20.0).abs() < 1e-9);
        assert!((axi.values[3].expected_plat - 100.0).abs() < 1e-9);
        assert!((axi.best.plat - 100.0).abs() < 1e-9);

        let ducats = &axi.values[0];
        assert!(ducats.expected_ducats >= 0.0);
    }

    #[test]
    fn wanted_parts() {
        let fixture = fixture(fixtures::inventory());
        let wanted = vec![fixtures::TRINITY_PRIME_SYSTEMS.to_owned()];
        let plans = plan(&fixture.view(), &wanted, 1, true, None);
        assert_eq!(plans.len(), 1);
        assert!((plans[0].best.wanted_chance - 0.0).abs() < f64::EPSILON);

        let braton = plan(
            &fixture.view(),
            &["/Lotus/Types/Recipes/Weapons/WeaponParts/AlternoxPrimeStock".to_owned()],
            1,
            true,
            None,
        );
        assert!(braton[0].best.wanted_chance > 0.0);
    }

    #[test]
    fn squad_beats_solo() {
        let fixture = fixture(fixtures::inventory());
        let solo = plan(&fixture.view(), &[], 1, true, None);
        let squad = plan(&fixture.view(), &[], 4, true, None);
        assert!(squad[0].values[0].expected_plat > solo[0].values[0].expected_plat);
    }

    #[test]
    fn shares_sum_to_expected_plat() {
        let fixture = fixture(fixtures::inventory());
        let plans = plan(&fixture.view(), &[], DEFAULT_SQUAD_SIZE, true, None);
        let axi = &plans[0];
        assert_eq!(axi.relic, "Axi A21");
        assert_eq!(axi.rewards.len(), 6);
        for value in &axi.values {
            assert_eq!(value.chances.len(), 6);
            assert_eq!(value.expected_plat_shares.len(), 6);
            let total: f64 = value.expected_plat_shares.iter().sum();
            assert!(
                (total - value.expected_plat).abs() < 1e-9,
                "{:?} shares summed to {total} against {}",
                value.refinement,
                value.expected_plat
            );
        }
    }

    #[test]
    fn plat_per_trace() {
        let pricy = Fixture::new(fixtures::catalog(), fixtures::inventory())
            .with_prices(FixedPrices::new([("alternox_prime_stock", 1000.0)]));
        let plans = plan(&pricy.view(), &[], 1, true, None);
        let axi = &plans[0];
        assert_eq!(
            axi.values
                .iter()
                .map(|value| value.traces)
                .collect::<Vec<u32>>(),
            vec![0, 25, 50, 100]
        );
        assert_eq!(axi.values[0].plat_per_trace, None);
        assert!((axi.intact_to_radiant.plat - 80.0).abs() < 1e-9);
        for value in &axi.values[1..] {
            let per_trace = value.plat_per_trace.unwrap();
            assert!((per_trace - 0.8).abs() < 1e-9);
        }
        let best_per_trace = axi.best.plat_per_trace.unwrap();
        assert!((best_per_trace.value - 0.8).abs() < 1e-9);
        assert_eq!(axi.best.refinement, Refinement::Radiant);

        let mixed = plan(&fixture(fixtures::inventory()).view(), &[], 1, true, None);
        assert_eq!(mixed[0].best.refinement, Refinement::Intact);
        assert!(mixed[0].intact_to_radiant.plat < 0.0);
    }

    #[test]
    fn reward_breakdown() {
        let fixture = fixture(fixtures::inventory());
        let plans = plan(&fixture.view(), &[], 1, true, None);
        let axi = &plans[0];

        let stock = axi
            .rewards
            .iter()
            .position(|reward| reward.name == "Alternox Prime Stock")
            .expect("Alternox Prime Stock");
        let alternox = &axi.rewards[stock];
        assert_eq!(alternox.rarity, "Rare");
        assert_eq!(alternox.plat, Some(10.0));
        assert!((axi.values[0].chances[stock] - 2.0).abs() < f64::EPSILON);
        assert!((axi.values[0].expected_plat_shares[stock] - 0.2).abs() < 1e-9);
        assert!((axi.values[3].chances[stock] - 10.0).abs() < f64::EPSILON);

        let forma = axi
            .rewards
            .iter()
            .find(|reward| reward.name == "Forma Blueprint")
            .expect("Forma Blueprint");
        assert_eq!(forma.plat, None);
        assert_eq!(forma.component, None);
        assert!(forma.ownership.owned > 0);
        assert!(!forma.ownership.needed_for_set);
    }

    #[test]
    fn chances_per_refinement() {
        let fixture = fixture(fixtures::inventory());
        let catalog = fixtures::catalog();
        let plans = plan(&fixture.view(), &[], DEFAULT_SQUAD_SIZE, true, None);
        let axi = &plans[0];
        let relic = catalog
            .relics()
            .find(|relic| relic.name == "Axi A21")
            .expect("Axi A21");
        for (value, refinement) in axi.values.iter().zip(Refinement::ALL) {
            let rewards = relic.rewards_for(refinement);
            assert_eq!(rewards.len(), axi.rewards.len());
            for (index, reward) in rewards.iter().enumerate() {
                assert_eq!(reward.item_unique_name, axi.rewards[index].unique_name);
                assert_eq!(rarity_name(reward.rarity), axi.rewards[index].rarity);
                assert!((value.chances[index] - reward.chance).abs() < f64::EPSILON);
            }
        }
    }

    #[test]
    fn best_refinement() {
        for prices in [
            FixedPrices::new([("alternox_prime_stock", 1000.0)]),
            FixedPrices::new([("styanax_prime_blueprint", 400.0)]),
        ] {
            let plans = plan(
                &Fixture::new(fixtures::catalog(), fixtures::inventory())
                    .with_prices(prices)
                    .view(),
                &[],
                DEFAULT_SQUAD_SIZE,
                true,
                None,
            );
            let axi = &plans[0];
            let top = axi
                .values
                .iter()
                .max_by(|a, b| a.expected_plat.total_cmp(&b.expected_plat))
                .unwrap();
            assert_eq!(axi.best.refinement, top.refinement);
            assert!((axi.best.plat - top.expected_plat).abs() < f64::EPSILON);
            let named = axi
                .values
                .iter()
                .find(|value| value.refinement == axi.best.refinement)
                .unwrap();
            assert!((named.expected_plat - axi.best.plat).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn wanted_chance_grows_with_squad() {
        let fixture = fixture(fixtures::inventory());
        let wanted = vec!["/Lotus/Types/Recipes/Weapons/WeaponParts/AlternoxPrimeStock".to_owned()];
        let solo = plan(&fixture.view(), &wanted, 1, true, None);
        let squad = plan(&fixture.view(), &wanted, 4, true, None);
        assert!(solo[0].best.wanted_chance > 0.0);
        assert!(squad[0].best.wanted_chance > solo[0].best.wanted_chance);
        assert!(squad[0].best.wanted_chance <= 100.0);
    }

    #[test]
    fn no_requiem_relics() {
        let fixture = fixture(fixtures::inventory());
        let plans = plan(&fixture.view(), &[], 4, true, None);
        assert!(!plans.iter().any(|entry| entry.relic.contains("Requiem")));
    }
}
