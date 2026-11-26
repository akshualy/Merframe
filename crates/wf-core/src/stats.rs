use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::Serialize;

use crate::account::Account;
use crate::identity::ItemTable;
use crate::inventory_view::ItemSummary;
use crate::store::{RelicOpening, StatPoint, StoredDelta, TimeRange};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct DeltaRow {
    pub item_type: String,
    pub name: String,
    pub category: String,
    pub delta: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct OpeningRow {
    pub id: i64,
    pub at: DateTime<Utc>,
    pub relic: String,
    pub reward: String,
    pub player_count: u8,
}

pub(crate) fn delta_rows(items: &ItemTable, deltas: Vec<StoredDelta>) -> Vec<DeltaRow> {
    deltas
        .into_iter()
        .map(|delta| DeltaRow {
            name: items.name(&delta.item_type),
            item_type: delta.item_type,
            category: delta.category,
            delta: delta.delta,
        })
        .collect()
}

pub(crate) fn opening_rows(items: &ItemTable, openings: Vec<RelicOpening>) -> Vec<OpeningRow> {
    openings
        .into_iter()
        .map(|opening| OpeningRow {
            id: opening.id,
            at: opening.at,
            relic: opening.relic,
            reward: items.name(&opening.reward_item),
            player_count: opening.player_count,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct DailyCount {
    pub day: NaiveDate,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct StatsItemHistory {
    pub item: ItemSummary,
    pub counts: Vec<DailyCount>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct StatsSummary {
    pub account_created: DateTime<Utc>,
    pub snapshot_days: usize,
}

pub fn daily_series(points: &[StatPoint]) -> Vec<StatPoint> {
    let mut days: BTreeMap<NaiveDate, StatPoint> = BTreeMap::new();
    for point in points {
        let day = point.at.date_naive();
        days.insert(
            day,
            StatPoint {
                at: day.and_time(NaiveTime::MIN).and_utc(),
                ..*point
            },
        );
    }
    days.into_values().collect()
}

pub fn day_span(
    range: TimeRange,
    earliest: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<(NaiveDate, NaiveDate)> {
    let earliest = earliest?;
    let from = range.from.max(earliest);
    let to = range.to.min(now);
    (from <= to).then(|| (from.date_naive(), to.date_naive()))
}

pub fn daily_counts(
    times: impl IntoIterator<Item = DateTime<Utc>>,
    span: Option<(NaiveDate, NaiveDate)>,
) -> Vec<DailyCount> {
    let Some((from, to)) = span else {
        return Vec::new();
    };
    let mut counts: BTreeMap<NaiveDate, i64> = from
        .iter_days()
        .take_while(|day| *day <= to)
        .map(|day| (day, 0))
        .collect();
    for at in times {
        if let Some(count) = counts.get_mut(&at.date_naive()) {
            *count += 1;
        }
    }
    counts
        .into_iter()
        .map(|(day, count)| DailyCount { day, count })
        .collect()
}

pub(crate) fn daily_item_counts(
    current: i64,
    changes: &[(DateTime<Utc>, i64)],
    (from, to): (NaiveDate, NaiveDate),
) -> Vec<DailyCount> {
    let mut gained: BTreeMap<NaiveDate, i64> = BTreeMap::new();
    for (at, delta) in changes {
        *gained.entry(at.date_naive()).or_default() += delta;
    }

    let mut counts: Vec<DailyCount> = std::iter::successors(Some(to), NaiveDate::pred_opt)
        .take_while(|day| *day >= from)
        .scan(current, |later, day| {
            let count = *later;
            *later -= gained.get(&day).copied().unwrap_or(0);
            Some(DailyCount { day, count })
        })
        .collect();
    counts.reverse();
    counts
}

pub fn distinct_days(times: impl IntoIterator<Item = DateTime<Utc>>) -> usize {
    let days: BTreeSet<NaiveDate> = times.into_iter().map(|at| at.date_naive()).collect();
    days.len()
}

pub(crate) fn summary(account: &Account, series: &[StatPoint]) -> StatsSummary {
    StatsSummary {
        account_created: account.inventory.created.datetime(),
        snapshot_days: distinct_days(series.iter().map(|point| point.at)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::store::SnapshotId;

    fn at(text: &str) -> DateTime<Utc> {
        text.parse::<DateTime<Utc>>().unwrap()
    }

    fn day(text: &str) -> NaiveDate {
        text.parse().unwrap()
    }

    fn point(at: DateTime<Utc>) -> StatPoint {
        StatPoint {
            snapshot: SnapshotId(1),
            at,
            plat: 0,
            credits: 0,
            endo: 0,
            mr: 0,
        }
    }

    fn plat_point(snapshot: i64, at: DateTime<Utc>, plat: i64) -> StatPoint {
        StatPoint {
            snapshot: SnapshotId(snapshot),
            plat,
            ..point(at)
        }
    }

    #[test]
    fn last_point_per_day() {
        let series = daily_series(&[
            plat_point(1, at("2023-11-14T01:00:00Z"), 100),
            plat_point(2, at("2023-11-14T23:59:59Z"), 340),
            plat_point(3, at("2023-11-16T12:00:00Z"), 275),
        ]);
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].at, at("2023-11-14T00:00:00Z"));
        assert_eq!(series[0].snapshot, SnapshotId(2));
        assert_eq!(series[0].plat, 340);
        assert_eq!(series[1].at, at("2023-11-16T00:00:00Z"));
        assert_eq!(series[1].plat, 275);
        assert!(daily_series(&[]).is_empty());
    }

    #[test]
    fn point_before_midnight() {
        let series = daily_series(&[
            plat_point(1, at("2023-11-14T23:59:59Z"), 10),
            plat_point(2, at("2023-11-15T00:00:00Z"), 20),
        ]);
        assert_eq!(
            series.iter().map(|entry| entry.at).collect::<Vec<_>>(),
            vec![at("2023-11-14T00:00:00Z"), at("2023-11-15T00:00:00Z")]
        );
    }

    #[test]
    fn day_span_clamping() {
        let earliest = at("2023-11-14T22:13:20Z");
        let now = at("2023-11-21T18:13:20Z");
        assert_eq!(
            day_span(TimeRange::all(), Some(earliest), now),
            Some((day("2023-11-14"), day("2023-11-21")))
        );
        assert_eq!(
            day_span(
                TimeRange::since(at("2023-11-19T10:00:00Z")),
                Some(earliest),
                now
            ),
            Some((day("2023-11-19"), day("2023-11-21")))
        );
        assert_eq!(day_span(TimeRange::all(), None, now), None);
        assert_eq!(
            day_span(TimeRange::since(now), Some(earliest), earliest),
            None
        );
    }

    #[test]
    fn dense_daily_counts() {
        let times = [
            at("2023-11-14T00:00:00Z"),
            at("2023-11-14T23:59:59Z"),
            at("2023-11-16T12:00:00Z"),
        ];
        let span = Some((day("2023-11-14"), day("2023-11-18")));
        assert_eq!(
            daily_counts(times, span),
            vec![
                DailyCount {
                    day: day("2023-11-14"),
                    count: 2
                },
                DailyCount {
                    day: day("2023-11-15"),
                    count: 0
                },
                DailyCount {
                    day: day("2023-11-16"),
                    count: 1
                },
                DailyCount {
                    day: day("2023-11-17"),
                    count: 0
                },
                DailyCount {
                    day: day("2023-11-18"),
                    count: 0
                },
            ]
        );
        assert!(daily_counts(times, None).is_empty());
    }

    #[test]
    fn item_counts_walked_back_from_now() {
        let changes = [
            (at("2026-03-02T08:00:00Z"), 30),
            (at("2026-03-03T08:00:00Z"), -25),
            (at("2026-03-03T20:00:00Z"), 15),
        ];

        let counts = daily_item_counts(100, &changes, (day("2026-03-01"), day("2026-03-03")));

        assert_eq!(
            counts.iter().map(|entry| entry.count).collect::<Vec<_>>(),
            [80, 110, 100]
        );
    }

    #[test]
    fn days_played_over_window() {
        let series = daily_series(&[
            plat_point(1, at("2023-11-14T01:00:00Z"), 100),
            plat_point(2, at("2023-11-14T20:00:00Z"), 120),
            plat_point(3, at("2023-11-16T12:00:00Z"), 130),
        ]);
        let span = day_span(
            TimeRange::since(at("2023-11-13T00:00:00Z")),
            series.first().map(|entry| entry.at),
            at("2023-11-17T09:00:00Z"),
        );
        let played = daily_counts(series.iter().map(|entry| entry.at), span);
        assert_eq!(
            played.iter().map(|entry| entry.count).collect::<Vec<_>>(),
            vec![1, 0, 1, 0]
        );
        assert_eq!(played[0].day, day("2023-11-14"));
        assert_eq!(distinct_days(series.iter().map(|entry| entry.at)), 2);
    }

    #[test]
    fn times_outside_span() {
        let counts = daily_counts(
            [at("2023-11-14T09:00:00Z"), at("2023-11-19T09:00:00Z")],
            Some((day("2023-11-14"), day("2023-11-15"))),
        );
        assert_eq!(counts.len(), 2);
        assert_eq!(counts.iter().map(|entry| entry.count).sum::<i64>(), 1);
    }

    #[test]
    fn distinct_days_ignore_points() {
        assert_eq!(
            distinct_days([
                at("2023-11-14T00:00:00Z"),
                at("2023-11-14T23:59:59Z"),
                at("2023-11-16T12:00:00Z"),
            ]),
            2
        );
        assert_eq!(distinct_days([]), 0);
    }

    #[test]
    fn fixture_summary() {
        let catalog = fixtures::mastery_catalog();
        let account = Account::new(fixtures::inventory(), &catalog, &ItemTable::build(&catalog));
        let series = daily_series(&[
            point(at("2023-11-14T00:00:00Z")),
            point(at("2023-11-14T23:59:59Z")),
            point(at("2023-11-16T12:00:00Z")),
        ]);
        let summary = summary(&account, &series);
        assert_eq!(summary.snapshot_days, 2);
        assert_eq!(
            summary.account_created.to_rfc3339(),
            "2020-01-15T00:00:00+00:00"
        );
    }
}

#[cfg(test)]
mod naming_tests {
    use super::*;
    use crate::catalog::fixtures;
    use crate::store::{RelicOpening, StoredDelta};
    use wf_data::Refinement;

    #[test]
    fn rows_have_item_names() {
        let catalog = fixtures::catalog();
        let relic = catalog.relics().next().unwrap();
        let relic_unique_name = relic.unique_names[&Refinement::Radiant].clone();
        let items = ItemTable::build(&catalog);
        let deltas = delta_rows(
            &items,
            vec![StoredDelta {
                item_type: relic_unique_name,
                category: "MiscItems".to_owned(),
                delta: 3,
            }],
        );
        assert_eq!(deltas[0].name, format!("{} Relic (Radiant)", relic.name));
        let openings = opening_rows(
            &items,
            vec![RelicOpening {
                id: 1,
                at: Utc::now(),
                relic: relic.name.clone(),
                reward_item: "/Lotus/StoreItems/Types/Items/MiscItems/Forma".to_owned(),
                player_count: 2,
            }],
        );
        assert_eq!(openings[0].reward, "Forma");
    }
}
