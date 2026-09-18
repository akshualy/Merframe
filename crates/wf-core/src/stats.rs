use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::Serialize;
use wf_inventory::Inventory;

use crate::catalog::Catalog;
use crate::mastery::{self, MasteryOptions};
use crate::store::{StatPoint, TimeRange};

pub(crate) const AYA_ITEM: &str = "/Lotus/Types/Items/MiscItems/SchismKey";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DailyCount {
    pub day: NaiveDate,
    pub count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct StatsSummary {
    pub account_created: DateTime<Utc>,
    pub snapshot_days: usize,
    pub prime_owned: u32,
    pub prime_total: u32,
    pub prime_percent: f64,
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

pub fn distinct_days(times: impl IntoIterator<Item = DateTime<Utc>>) -> usize {
    let days: BTreeSet<NaiveDate> = times.into_iter().map(|at| at.date_naive()).collect();
    days.len()
}

pub(crate) fn summary(
    inventory: &Inventory,
    catalog: &Catalog,
    series: &[StatPoint],
) -> StatsSummary {
    let (prime_owned, prime_total) =
        mastery::prime_ownership(inventory, catalog, MasteryOptions::default());
    StatsSummary {
        account_created: inventory.created.datetime(),
        snapshot_days: distinct_days(series.iter().map(|point| point.at)),
        prime_owned,
        prime_total,
        prime_percent: prime_percent(prime_owned, prime_total),
    }
}

fn prime_percent(owned: u32, total: u32) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (100.0 * (f64::from(owned) / f64::from(total))).floor()
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
            ducats: 0,
            aya: None,
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
        let inventory = fixtures::inventory();
        let series = daily_series(&[
            point(at("2023-11-14T00:00:00Z")),
            point(at("2023-11-14T23:59:59Z")),
            point(at("2023-11-16T12:00:00Z")),
        ]);
        let summary = summary(&inventory, &fixtures::mastery_catalog(), &series);
        assert_eq!(summary.prime_owned, 24);
        assert_eq!(summary.prime_total, 65);
        assert!((summary.prime_percent - 36.0).abs() < f64::EPSILON);
        assert_eq!(summary.snapshot_days, 2);
        assert_eq!(
            summary.account_created.to_rfc3339(),
            "2020-01-15T00:00:00+00:00"
        );
    }

    #[test]
    fn prime_percent_floor() {
        assert!((prime_percent(0, 0) - 0.0).abs() < f64::EPSILON);
        assert!((prime_percent(1, 3) - 33.0).abs() < f64::EPSILON);
        assert!((prime_percent(2, 3) - 66.0).abs() < f64::EPSILON);
        assert!((prime_percent(199, 200) - 99.0).abs() < f64::EPSILON);
        assert!((prime_percent(200, 200) - 100.0).abs() < f64::EPSILON);
    }
}
