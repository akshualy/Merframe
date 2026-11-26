use std::collections::HashMap;

use serde::Serialize;
use wf_data::{Item, Rarity, VosforPack};

use super::upgrades::{UpgradeKind, arcane_copies_by_rank, upgrade_kind};
use super::{ItemIndex, ItemSummary};
use crate::identity::{ItemKind, vosfor_pack_image};
use crate::view::View;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ArcaneCollectionStatus {
    CanBeMaxed,
    Collecting,
    NotOwned,
    Maxed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ArcaneRankCount {
    pub rank: u32,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct ArcaneCollectionRow {
    pub item: usize,
    pub rarity: Option<Rarity>,
    pub max_rank: u32,
    pub ranks: Vec<ArcaneRankCount>,
    pub copies: i64,
    pub required: i64,
    pub remaining: i64,
    pub status: ArcaneCollectionStatus,
    pub sell: Option<u32>,
    pub finish_cost: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct VosforPackNeededArcane {
    pub row: usize,
    pub chance_per_pack: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct VosforPackCompletion {
    pub packs: f64,
    pub vosfor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct VosforPackRow {
    pub name: String,
    pub image_name: Option<String>,
    pub arcane_count: usize,
    pub draws: u32,
    pub needed: Vec<VosforPackNeededArcane>,
    pub needed_per_pack: f64,
    pub needed_chance: f64,
    pub completion: Option<VosforPackCompletion>,
}

fn copies_fused_into(rank: u32) -> i64 {
    i64::from(rank + 1) * i64::from(rank + 2) / 2
}

fn released(arcane: &Item) -> bool {
    arcane.wikia_url.is_some()
}

fn status(ranks: &[ArcaneRankCount], max_rank: u32, remaining: i64) -> ArcaneCollectionStatus {
    if ranks.iter().any(|held| held.rank >= max_rank) {
        ArcaneCollectionStatus::Maxed
    } else if ranks.is_empty() {
        ArcaneCollectionStatus::NotOwned
    } else if remaining == 0 {
        ArcaneCollectionStatus::CanBeMaxed
    } else {
        ArcaneCollectionStatus::Collecting
    }
}

pub(crate) fn arcane_collection(view: &View, index: &mut ItemIndex) -> Vec<ArcaneCollectionRow> {
    let held = arcane_copies_by_rank(&view.account.inventory);
    let mut rows: Vec<ArcaneCollectionRow> = view
        .catalog
        .items()
        .filter(|item| upgrade_kind(&item.unique_name) == UpgradeKind::Arcane && released(item))
        .filter_map(|item| {
            let record = view.items.resolve(&item.unique_name);
            let ItemKind::Upgrade {
                rarity,
                max_rank: Some(max_rank @ 1..),
            } = record.kind
            else {
                return None;
            };

            let ranks: Vec<ArcaneRankCount> = held
                .get(item.unique_name.as_str())
                .into_iter()
                .flatten()
                .rev()
                .map(|(&rank, &count)| ArcaneRankCount { rank, count })
                .collect();

            let copies: i64 = ranks
                .iter()
                .map(|held| copies_fused_into(held.rank) * held.count)
                .sum();
            let required = copies_fused_into(max_rank);
            let remaining = (required - copies).max(0);

            let sell = record
                .market_slug
                .as_deref()
                .and_then(|slug| view.prices.plat(slug));
            Some(ArcaneCollectionRow {
                item: index.add_marked(
                    ItemSummary::from(&*record),
                    view.favourites.contains(&item.unique_name),
                ),
                rarity,
                max_rank,
                status: status(&ranks, max_rank, remaining),
                ranks,
                copies,
                required,
                remaining,
                sell,
                finish_cost: sell
                    .filter(|_| remaining > 0)
                    .map(|sell| i64::from(sell) * remaining),
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        (a.status, a.remaining, &index[a.item].name).cmp(&(
            b.status,
            b.remaining,
            &index[b.item].name,
        ))
    });
    rows
}

struct VosforPackNeed {
    row: usize,
    copies: i64,
    draw_chance: f64,
}

fn in_any_draw(draw_chance: f64, draws: u32) -> f64 {
    1.0 - (1.0 - draw_chance).powf(f64::from(draws))
}

fn still_missing_after(draws: f64, need: &VosforPackNeed) -> f64 {
    let mean = need.draw_chance * draws;
    let mut term = (-mean).exp();
    let mut fewer_copies = 0.0;
    let mut copy = 0.0;
    for _ in 0..need.copies {
        fewer_copies += term;
        copy += 1.0;
        term *= mean / copy;
    }

    fewer_copies
}

fn incomplete_after(draws: f64, needs: &[VosforPackNeed]) -> f64 {
    1.0 - needs
        .iter()
        .map(|need| 1.0 - still_missing_after(draws, need))
        .product::<f64>()
}

fn draws_to_finish(needs: &[VosforPackNeed]) -> f64 {
    let likeliest = needs
        .iter()
        .map(|need| need.draw_chance)
        .fold(0.0, f64::max);
    let step = 0.25 / likeliest;

    let mut expected = 0.0;
    let mut draws = 0.0;
    let mut start = 1.0;
    loop {
        let middle = incomplete_after(draws + step, needs);
        let end = incomplete_after(draws + 2.0 * step, needs);
        expected += step / 3.0 * (start + 4.0 * middle + end);
        if end < 1e-9 {
            return expected;
        }

        draws += 2.0 * step;
        start = end;
    }
}

fn whole_packs(draws: f64, draws_per_pack: u32) -> f64 {
    let draws_per_pack = f64::from(draws_per_pack);
    (draws + (draws_per_pack - 1.0) / 2.0) / draws_per_pack
}

pub(crate) fn vosfor_packs(
    packs: &[VosforPack],
    rows: &[ArcaneCollectionRow],
    index: &ItemIndex,
) -> Vec<VosforPackRow> {
    let incomplete: HashMap<&str, usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.remaining > 0)
        .map(|(position, row)| (index[row.item].unique_name.as_str(), position))
        .collect();
    let mut packs: Vec<VosforPackRow> = packs
        .iter()
        .map(|pack| {
            let mut needs: Vec<VosforPackNeed> = pack
                .arcanes
                .iter()
                .filter_map(|arcane| {
                    let row = *incomplete.get(arcane.unique_name.as_str())?;
                    Some(VosforPackNeed {
                        row,
                        copies: rows[row].remaining,
                        draw_chance: arcane.draw_chance,
                    })
                })
                .collect();
            needs.sort_unstable_by_key(|need| need.row);

            let completion = (!needs.is_empty()).then(|| {
                let packs = whole_packs(draws_to_finish(&needs), pack.draws);
                VosforPackCompletion {
                    packs,
                    vosfor: packs * f64::from(pack.vosfor),
                }
            });

            let needed_per_draw: f64 = needs.iter().map(|need| need.draw_chance).sum();
            VosforPackRow {
                name: pack.name.clone(),
                image_name: pack.icon.as_deref().and_then(vosfor_pack_image),
                arcane_count: pack.arcanes.len(),
                draws: pack.draws,
                needed: needs
                    .iter()
                    .map(|need| VosforPackNeededArcane {
                        row: need.row,
                        chance_per_pack: in_any_draw(need.draw_chance, pack.draws),
                    })
                    .collect(),
                needed_per_pack: f64::from(pack.draws) * needed_per_draw,
                needed_chance: in_any_draw(needed_per_draw, pack.draws),
                completion,
            }
        })
        .collect();
    packs.sort_by(|a, b| {
        b.needed_per_pack
            .total_cmp(&a.needed_per_pack)
            .then(a.name.cmp(&b.name))
    });
    packs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::prices::FixedPrices;
    use crate::view::Fixture;
    use wf_data::VosforPackArcane;

    fn collection(prices: FixedPrices) -> (Vec<ArcaneCollectionRow>, ItemIndex) {
        Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory())
            .with_prices(prices)
            .rows(arcane_collection)
    }

    #[test]
    fn copies_count_towards_max_rank() {
        let (rows, index) = collection(FixedPrices::new([("arcane_bodyguard", 4)]));
        let row = |name: &str| {
            rows.iter()
                .find(|row| index[row.item].name == name)
                .unwrap()
        };

        let bodyguard = row("Arcane Bodyguard");
        assert_eq!((bodyguard.copies, bodyguard.required), (7, 21));
        assert_eq!(bodyguard.status, ArcaneCollectionStatus::Collecting);
        assert_eq!(bodyguard.finish_cost, Some(56));

        let retaliation = row("Melee Retaliation");
        assert_eq!(retaliation.status, ArcaneCollectionStatus::CanBeMaxed);
        assert_eq!(retaliation.remaining, 0);

        let surge = row("Virtuos Surge");
        assert_eq!((surge.copies, surge.required), (19, 10));
        assert_eq!(surge.status, ArcaneCollectionStatus::Maxed);
        assert_eq!(
            surge.ranks,
            [
                ArcaneRankCount { rank: 3, count: 1 },
                ArcaneRankCount { rank: 0, count: 9 }
            ]
        );

        let barrier = row("Arcane Barrier");
        assert_eq!(barrier.status, ArcaneCollectionStatus::NotOwned);
        assert_eq!(barrier.remaining, 21);
    }

    #[test]
    fn fewest_missing_first() {
        let (rows, index) = collection(FixedPrices::default());
        let order: Vec<&str> = rows
            .iter()
            .map(|row| index[row.item].name.as_str())
            .filter(|name| {
                [
                    "Arcane Energize",
                    "Arcane Ice",
                    "Arcane Barrier",
                    "Arcane Bodyguard",
                    "Melee Retaliation",
                ]
                .contains(name)
            })
            .collect();
        assert_eq!(
            order,
            [
                "Melee Retaliation",
                "Arcane Bodyguard",
                "Arcane Barrier",
                "Arcane Ice",
                "Arcane Energize",
            ]
        );
    }

    #[test]
    fn unreleased_arcanes_are_left_out() {
        let unreleased = r#"[{
            "uniqueName": "/Lotus/Upgrades/CosmeticEnhancers/Defensive/ImpactProcResist",
            "name": "Arcane Shield", "category": "Arcanes", "type": "Warframe Arcane",
            "tradable": true, "rarity": "Rare", "levelStats": [{}, {}, {}, {}, {}, {}]
        }]"#;
        let catalog = fixtures::catalog_with(fixtures::UPGRADE_ITEMS, unreleased);
        let (rows, index) = Fixture::new(catalog, fixtures::inventory()).rows(arcane_collection);
        assert!(
            rows.iter()
                .all(|row| index[row.item].name != "Arcane Shield")
        );
    }

    fn need(copies: i64, draw_chance: f64) -> VosforPackNeed {
        VosforPackNeed {
            row: 0,
            copies,
            draw_chance,
        }
    }

    #[test]
    fn draws_until_every_copy_is_in() {
        let close = |draws: f64, expected: f64| (draws - expected).abs() < 1e-2;

        let one_copy = draws_to_finish(&[need(1, 0.5)]);
        assert!(close(one_copy, 2.0), "{one_copy}");
        let three_copies = draws_to_finish(&[need(3, 0.25)]);
        assert!(close(three_copies, 12.0), "{three_copies}");
        let either_order = draws_to_finish(&[need(1, 0.5), need(1, 0.5)]);
        assert!(close(either_order, 3.0), "{either_order}");
    }

    #[test]
    fn a_started_pack_counts_whole() {
        assert!((whole_packs(3.0, 3) - 4.0 / 3.0).abs() < 1e-9);
        assert!((whole_packs(4.0, 2) - 2.25).abs() < 1e-9);
    }

    fn pack(name: &str, arcanes: &[(&str, f64)]) -> VosforPack {
        VosforPack {
            name: name.to_owned(),
            icon: Some("/Lotus/Interface/Icons/CetusPackIcon.png".to_owned()),
            vosfor: 200,
            draws: 3,
            arcanes: arcanes
                .iter()
                .map(|&(unique_name, draw_chance)| VosforPackArcane {
                    unique_name: unique_name.to_owned(),
                    draw_chance,
                })
                .collect(),
        }
    }

    #[test]
    fn packs_rank_by_needed_arcanes() {
        const BARRIER: &str = "/Lotus/Upgrades/CosmeticEnhancers/Defensive/InstantShieldOnDamage";
        const ICE: &str = "/Lotus/Upgrades/CosmeticEnhancers/Defensive/FireProcResist";
        const ENERGIZE: &str =
            "/Lotus/Upgrades/CosmeticEnhancers/Utility/GolemArcaneRadialEnergyOnEnergyPickup";
        let packs = [
            pack("Maxed", &[(ENERGIZE, 1.0)]),
            pack("One Needed", &[(ICE, 0.25), (ENERGIZE, 0.75)]),
            pack("Two Needed", &[(ICE, 0.5), (BARRIER, 0.5)]),
        ];
        let (rows, index) = collection(FixedPrices::default());
        let ranked = vosfor_packs(&packs, &rows, &index);

        let names: Vec<&str> = ranked.iter().map(|pack| pack.name.as_str()).collect();
        assert_eq!(names, ["Two Needed", "One Needed", "Maxed"]);
        let needed: Vec<&str> = ranked[0]
            .needed
            .iter()
            .map(|arcane| index[rows[arcane.row].item].name.as_str())
            .collect();
        assert_eq!(needed, ["Arcane Barrier", "Arcane Ice"]);
        assert!((ranked[1].needed_per_pack - 0.75).abs() < 1e-9);
        assert!((ranked[1].needed_chance - (1.0 - 0.75_f64.powi(3))).abs() < 1e-9);
        assert_eq!(ranked[2].completion, None);
        assert_eq!(
            ranked[0].image_name.as_deref(),
            Some("game/vosfor-pack-cetus.png")
        );
    }
}
