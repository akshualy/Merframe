use wf_inventory::Inventory;

use super::intrinsics::{intrinsics, row as intrinsic_row};
use super::items::{includes_founders, items, totals};
use super::routes::level_up_routes;
use super::star_chart::star_chart;
use super::xp::{mastery_xp, xp_for_rank};
use super::{
    CategoryTotals, MasteryItem, MasteryOptions, MasteryOrdering, MasterySummary, MasteryTab,
    percent,
};
use crate::view::View;

fn owned_parts(item: &MasteryItem) -> usize {
    item.components
        .iter()
        .filter(|component| component.enough)
        .count()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "XP and platinum stay far below 2^53"
)]
fn order_items(
    unmastered: Vec<MasteryItem>,
    extras: Vec<MasteryItem>,
    options: MasteryOptions,
) -> Vec<MasteryItem> {
    match options.ordering {
        MasteryOrdering::ByPlatinum => {
            let mut rows: Vec<MasteryItem> = unmastered
                .into_iter()
                .filter(|item| !item.owned && item.acquisition.purchasable)
                .collect();
            rows.sort_by(|left, right| {
                let ratio = |item: &MasteryItem| {
                    if item.acquisition.plat_cost == 0 {
                        f64::INFINITY
                    } else {
                        item.level.xp_remaining as f64 / item.acquisition.plat_cost as f64
                    }
                };
                ratio(right)
                    .total_cmp(&ratio(left))
                    .then_with(|| owned_parts(left).cmp(&owned_parts(right)))
                    .then_with(|| left.name.cmp(&right.name))
            });
            rows
        }
        MasteryOrdering::FromRelics => {
            let mut rows: Vec<MasteryItem> = unmastered
                .into_iter()
                .filter(|item| {
                    !item.owned
                        && item.acquisition.relic_probability > 0.0
                        && !item.components.is_empty()
                })
                .collect();
            rows.sort_by(|left, right| {
                right
                    .acquisition
                    .relic_probability
                    .total_cmp(&left.acquisition.relic_probability)
                    .then_with(|| {
                        left.acquisition
                            .missing_parts
                            .cmp(&right.acquisition.missing_parts)
                    })
                    .then_with(|| left.name.cmp(&right.name))
            });
            rows
        }
        MasteryOrdering::Closest => {
            let mut rows: Vec<MasteryItem> = unmastered
                .into_iter()
                .chain(extras)
                .filter(|item| options.include_forma_ranks || item.level.current < 30)
                .filter(|item| {
                    item.owned
                        || (!item.components.is_empty()
                            && item.components.iter().all(|component| component.enough))
                })
                .collect();
            rows.sort_by(|left, right| {
                let rank = |item: &MasteryItem| {
                    if item.owned {
                        1
                    } else {
                        item.acquisition.missing_parts
                    }
                };
                right
                    .owned
                    .cmp(&left.owned)
                    .then_with(|| rank(left).cmp(&rank(right)))
                    .then_with(|| right.level.xp_remaining.cmp(&left.level.xp_remaining))
                    .then_with(|| left.name.cmp(&right.name))
            });
            rows
        }
    }
}

fn favourite_percent(favourite_xp: u64, span: u64, rank_percent: f64) -> f64 {
    percent(favourite_xp, span).min(100.0 - rank_percent)
}

fn combined_percent(parts: &[CategoryTotals]) -> f64 {
    let current = parts.iter().map(|part| u64::from(part.current)).sum();
    let max = parts.iter().map(|part| u64::from(part.max)).sum();
    percent(current, max)
}

fn summary(inventory: &Inventory, all: &[MasteryItem]) -> MasterySummary {
    let (star_normal, star_steel, star_junctions, star_steel_junctions) = star_chart(inventory);
    let (intrinsic_railjack, intrinsic_duviri) = intrinsics(inventory);
    let (warframes, weapons, companions) = totals(all);
    MasterySummary {
        content_percent: combined_percent(&[warframes, weapons, companions]),
        star_percent: combined_percent(&[
            star_normal,
            star_steel,
            star_junctions,
            star_steel_junctions,
        ]),
        intrinsic_percent: combined_percent(&[intrinsic_railjack, intrinsic_duviri]),
        warframes,
        weapons,
        companions,
        star_normal,
        star_steel,
        star_junctions,
        star_steel_junctions,
        intrinsic_railjack,
        intrinsic_duviri,
    }
}

fn unfinished_intrinsics(summary: &MasterySummary) -> Vec<MasteryItem> {
    let started = |totals: CategoryTotals| totals.current > 0 && totals.current < totals.max;
    let mut extras = Vec::new();
    if started(summary.intrinsic_duviri) {
        extras.push(intrinsic_row(
            "duviriIntrinsics",
            "Duviri intrinsics",
            "game/intrinsics-drifter.png",
        ));
    }
    if started(summary.intrinsic_railjack) {
        extras.push(intrinsic_row(
            "railjackIntrinsics",
            "Railjack intrinsics",
            "game/intrinsics-railjack.png",
        ));
    }
    extras
}

pub(crate) fn tab(view: &View, options: MasteryOptions) -> MasteryTab {
    let View {
        account, catalog, ..
    } = *view;
    let inventory = &account.inventory;
    let all = items(view, options);
    let summary = summary(inventory, &all);
    let extras = if options.ordering == MasteryOrdering::Closest {
        unfinished_intrinsics(&summary)
    } else {
        Vec::new()
    };

    let routes = level_up_routes(account, catalog, view.items, &all);
    let unmastered: Vec<MasteryItem> = all.into_iter().filter(|item| !item.mastered).collect();
    let recommended = order_items(unmastered, extras, options);
    let plat_total = recommended
        .iter()
        .map(|item| item.acquisition.plat_cost)
        .sum();

    let total = mastery_xp(account, catalog).total();
    let rank = inventory.player_level;
    let current = xp_for_rank(rank);
    let next = xp_for_rank(rank + 1);
    let span = next.saturating_sub(current);
    let earned = total.saturating_sub(current);

    let percent = percent(earned, span).min(100.0);
    let favourite_xp = recommended
        .iter()
        .filter(|item| item.favourite)
        .map(|item| item.level.xp_remaining)
        .sum();

    MasteryTab {
        rank,
        founder: inventory.is_founder(),
        include_founders: includes_founders(inventory, options.include_founders_items),
        percent,
        rank_xp_earned: earned,
        rank_xp_span: span,
        summary,
        plat_total,
        favourite_xp,
        favourite_percent: favourite_percent(favourite_xp, span, percent),
        recommended,
        routes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::Account;
    use crate::catalog::fixtures;
    use crate::mastery::items::{FOUNDERS_ITEMS, unmastered_types};
    use crate::mastery::support::{excluding_founders, fixture};

    #[test]
    fn fixture_rank_progress() {
        let fixture = fixture(fixtures::inventory());
        let tab = tab(&fixture.view(), MasteryOptions::default());

        assert_eq!(tab.rank, 14);
        assert_eq!(tab.rank_xp_earned, 45_138);
        assert_eq!(tab.rank_xp_span, 72_500);
        assert!((tab.percent - 62.259_310).abs() < 1e-6);
        assert!((tab.summary.star_percent - 100.0).abs() < f64::EPSILON);
        assert!((tab.summary.intrinsic_percent - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    #[allow(
        clippy::cast_precision_loss,
        reason = "mastery XP stays far below 2^53"
    )]
    fn favourite_xp_band() {
        let mut fixture = fixture(fixtures::inventory());
        let options = MasteryOptions::default();
        let plain = tab(&fixture.view(), options);
        assert!(!plain.recommended.is_empty());
        assert!(plain.recommended.iter().all(|item| !item.favourite));
        assert_eq!(plain.favourite_xp, 0);
        assert!((plain.favourite_percent - 0.0).abs() < f64::EPSILON);

        let first = &plain.recommended[0];
        fixture.favourites = [first.unique_name.clone()].into_iter().collect();
        let marked = tab(&fixture.view(), options);
        assert_eq!(
            marked
                .recommended
                .iter()
                .filter(|item| item.favourite)
                .count(),
            1
        );
        assert_eq!(marked.favourite_xp, first.level.xp_remaining);
        let expected = (100.0 * (first.level.xp_remaining as f64 / marked.rank_xp_span as f64))
            .min(100.0 - marked.percent);
        assert!((marked.favourite_percent - expected).abs() < 0.001);
    }

    #[test]
    fn favourite_percent_clamps() {
        assert!((favourite_percent(0, 100, 27.0) - 0.0).abs() < f64::EPSILON);
        assert!((favourite_percent(10, 100, 27.0) - 10.0).abs() < f64::EPSILON);
        assert!((favourite_percent(900, 100, 27.0) - 73.0).abs() < f64::EPSILON);
        assert!((favourite_percent(10, 0, 0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn non_founder_account() {
        let fixture = fixture(fixtures::inventory());
        assert!(!fixture.account.inventory.is_founder());
        let shown = items(&fixture.view(), MasteryOptions::default());
        assert!(
            !shown
                .iter()
                .any(|item| FOUNDERS_ITEMS.contains(&item.unique_name.as_str()))
        );
        let tab = tab(&fixture.view(), MasteryOptions::default());
        assert!(!tab.founder);
        assert!(!tab.include_founders);
    }

    #[test]
    fn founder_account() {
        let fixture = fixture(fixtures::founder_inventory());
        let shown = items(&fixture.view(), MasteryOptions::default());
        assert!(
            shown
                .iter()
                .any(|item| item.unique_name == FOUNDERS_ITEMS[0])
        );

        let hidden = items(&fixture.view(), excluding_founders());
        assert!(
            !hidden
                .iter()
                .any(|item| FOUNDERS_ITEMS.contains(&item.unique_name.as_str()))
        );
        assert_eq!(shown.len() - hidden.len(), 3);

        let tab = tab(&fixture.view(), MasteryOptions::default());
        assert!(tab.founder);
        assert!(tab.include_founders);
    }

    #[test]
    fn excluded_founders_items() {
        let fixture = fixture(fixtures::founder_inventory());
        let with = tab(&fixture.view(), MasteryOptions::default());
        let without = tab(&fixture.view(), excluding_founders());

        assert!(!without.include_founders);
        assert!(without.founder);
        assert_eq!(
            with.summary.warframes.max - without.summary.warframes.max,
            1
        );
        assert_eq!(with.summary.weapons.max - without.summary.weapons.max, 2);

        let names = |tab: &MasteryTab| {
            let mut names: Vec<String> = tab
                .recommended
                .iter()
                .map(|item| item.name.clone())
                .collect();
            names.extend(
                tab.routes
                    .iter()
                    .flat_map(|route| route.members.iter().map(|member| member.name.clone())),
            );
            names
        };
        assert!(names(&with).iter().any(|name| name == "Excalibur Prime"));
        assert!(names(&without).iter().all(|name| name != "Excalibur Prime"));

        let unmastered = unmastered_types(&fixture.account, &fixture.catalog);
        assert!(unmastered.contains(FOUNDERS_ITEMS[0]));
        let no_founder = unmastered_types(&Account::new(fixtures::inventory()), &fixture.catalog);
        assert!(!no_founder.contains(FOUNDERS_ITEMS[0]));
    }

    #[test]
    fn founder_xp_unchanged() {
        let fixture = fixture(fixtures::founder_inventory());
        let breakdown = mastery_xp(&fixture.account, &fixture.catalog);
        assert_eq!(
            breakdown.total(),
            mastery_xp(&Account::new(fixtures::inventory()), &fixture.catalog).total()
        );
        let with = tab(&fixture.view(), MasteryOptions::default());
        let without = tab(&fixture.view(), excluding_founders());
        assert_eq!(with.rank_xp_earned, without.rank_xp_earned);
    }

    #[test]
    fn default_ordering() {
        let fixture = fixture(fixtures::inventory());
        let tab = tab(&fixture.view(), MasteryOptions::default());
        assert!(tab.recommended.iter().all(|item| !item.mastered));
        let first_unowned = tab.recommended.iter().position(|item| !item.owned);
        let last_owned = tab.recommended.iter().rposition(|item| item.owned);
        if let (Some(first_unowned), Some(last_owned)) = (first_unowned, last_owned) {
            assert!(last_owned < first_unowned);
        }
        assert!(
            !tab.recommended
                .iter()
                .any(|item| item.kind.ends_with("Intrinsics")),
            "the fixture account has both intrinsic trees maxed"
        );
    }

    #[test]
    fn forma_rank_filter() {
        let fixture = fixture(fixtures::inventory());
        let without_forma = tab(
            &fixture.view(),
            MasteryOptions {
                include_forma_ranks: false,
                ..MasteryOptions::default()
            },
        );
        assert!(
            without_forma
                .recommended
                .iter()
                .all(|item| item.level.current < 30)
        );
        let with_forma = tab(&fixture.view(), MasteryOptions::default());
        assert!(with_forma.recommended.len() >= without_forma.recommended.len());
    }

    #[test]
    fn platinum_ordering() {
        let fixture = fixture(fixtures::inventory());
        let tab = tab(
            &fixture.view(),
            MasteryOptions {
                ordering: MasteryOrdering::ByPlatinum,
                ..MasteryOptions::default()
            },
        );
        assert!(
            tab.recommended
                .iter()
                .all(|item| !item.owned && item.acquisition.purchasable)
        );
        assert_eq!(
            tab.plat_total,
            tab.recommended
                .iter()
                .map(|item| item.acquisition.plat_cost)
                .sum::<u64>()
        );
    }

    #[test]
    fn relic_ordering() {
        let fixture = fixture(fixtures::inventory());
        let tab = tab(
            &fixture.view(),
            MasteryOptions {
                ordering: MasteryOrdering::FromRelics,
                ..MasteryOptions::default()
            },
        );
        assert!(
            tab.recommended
                .iter()
                .all(|item| !item.owned && item.acquisition.relic_probability > 0.0)
        );
    }
}
