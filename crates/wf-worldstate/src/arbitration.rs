use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, WorldStateError};
use crate::nodes::{node_and_planet, node_info};

pub const ARBITRATION_FORECAST_URL: &str = "https://browse.wf/arbys.txt";
pub const ARBITRATION_TIERS_URL: &str = "https://browse.wf/supplemental-data/arbyTiers.js";

const HOUR_SECS: i64 = 3600;
const SHORTEST_FORECAST_HOURS: usize = 24;
const TIERS_TABLE_START: &str = "window.arbyTiers";

fn forecast_error(line: usize, reason: &'static str) -> WorldStateError {
    WorldStateError::Forecast { line, reason }
}

fn is_node_id(id: &str) -> bool {
    ["SolNode", "ClanNode", "SettlementNode"]
        .into_iter()
        .filter_map(|prefix| id.strip_prefix(prefix))
        .any(|digits| {
            !digits.is_empty() && digits.len() <= 5 && digits.bytes().all(|b| b.is_ascii_digit())
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrationForecast {
    starts: DateTime<Utc>,
    node_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArbitrationForecastHour<'a> {
    pub node_id: &'a str,
    pub starts: DateTime<Utc>,
    pub ends: DateTime<Utc>,
}

impl ArbitrationForecast {
    pub fn parse(text: &str) -> Result<Self> {
        let mut first_second = None;
        let mut node_ids: Vec<String> = Vec::new();
        for (number, line) in text
            .lines()
            .map(str::trim)
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
        {
            let line_number = number + 1;
            let (seconds, node_id) = line.split_once(',').ok_or(forecast_error(
                line_number,
                "expected seconds and a node id",
            ))?;
            let seconds: i64 = seconds
                .parse()
                .map_err(|_| forecast_error(line_number, "the seconds are not a number"))?;
            if seconds % HOUR_SECS != 0 {
                return Err(forecast_error(line_number, "the time is not on the hour"));
            }
            if !is_node_id(node_id) {
                return Err(forecast_error(
                    line_number,
                    "the node id has an unknown shape",
                ));
            }

            let hours_so_far = i64::try_from(node_ids.len())
                .map_err(|_| forecast_error(line_number, "too many hours"))?;
            let first = *first_second.get_or_insert(seconds);
            if seconds != first + hours_so_far * HOUR_SECS {
                return Err(forecast_error(line_number, "the hours are not consecutive"));
            }
            node_ids.push(node_id.to_owned());
        }

        if node_ids.len() < SHORTEST_FORECAST_HOURS {
            return Err(WorldStateError::ForecastShape("shorter than a day"));
        }
        let starts = first_second
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
            .ok_or(WorldStateError::ForecastShape(
                "the first hour is out of range",
            ))?;
        Ok(Self { starts, node_ids })
    }

    pub fn starts(&self) -> DateTime<Utc> {
        self.starts
    }

    pub fn ends(&self) -> DateTime<Utc> {
        self.hour_at(self.node_ids.len())
    }

    pub fn hours(&self) -> usize {
        self.node_ids.len()
    }

    fn hour_at(&self, index: usize) -> DateTime<Utc> {
        self.starts + TimeDelta::hours(i64::try_from(index).unwrap_or(i64::MAX))
    }

    pub fn index_at(&self, now: DateTime<Utc>) -> Option<usize> {
        let elapsed = now.signed_duration_since(self.starts);
        let index = usize::try_from(elapsed.num_hours()).ok()?;
        (elapsed >= TimeDelta::zero() && index < self.node_ids.len()).then_some(index)
    }

    pub fn hour(&self, index: usize) -> Option<ArbitrationForecastHour<'_>> {
        let node_id = self.node_ids.get(index)?;
        Some(ArbitrationForecastHour {
            node_id,
            starts: self.hour_at(index),
            ends: self.hour_at(index + 1),
        })
    }

    pub fn hours_from(
        &self,
        index: usize,
        count: usize,
    ) -> impl Iterator<Item = ArbitrationForecastHour<'_>> {
        (index..index.saturating_add(count)).map_while(|index| self.hour(index))
    }

    pub fn unique_node_ids(&self) -> BTreeSet<&str> {
        self.node_ids.iter().map(String::as_str).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ArbitrationTier {
    S,
    A,
    B,
    C,
    D,
    F,
}

impl ArbitrationTier {
    fn parse(letter: &str) -> Option<Self> {
        Some(match letter {
            "S" => Self::S,
            "A" => Self::A,
            "B" => Self::B,
            "C" => Self::C,
            "D" => Self::D,
            "F" => Self::F,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::S => "S",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::F => "F",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArbitrationTiers(BTreeMap<String, ArbitrationTier>);

impl ArbitrationTiers {
    pub fn parse(script: &str) -> Result<Self> {
        let body = script
            .split_once(TIERS_TABLE_START)
            .and_then(|(_, rest)| rest.split_once('{'))
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(body, _)| body)
            .ok_or(WorldStateError::Tiers("no tier table"))?;
        let mut tiers = BTreeMap::new();
        for entry in body
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            let (node_id, tier) = entry
                .split_once(':')
                .ok_or(WorldStateError::Tiers("an entry has no colon"))?;
            let node_id = node_id.trim();
            let tier = ArbitrationTier::parse(tier.trim().trim_matches('"'))
                .ok_or(WorldStateError::Tiers("an entry has an unknown tier"))?;
            if !is_node_id(node_id) {
                return Err(WorldStateError::Tiers("an entry has an unknown node id"));
            }
            tiers.insert(node_id.to_owned(), tier);
        }

        if tiers.is_empty() {
            return Err(WorldStateError::Tiers("the tier table is empty"));
        }
        Ok(Self(tiers))
    }

    pub fn of(&self, node_id: &str) -> ArbitrationTier {
        self.0.get(node_id).copied().unwrap_or(ArbitrationTier::F)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArbitrationNode {
    pub node_id: String,
    pub node: String,
    pub planet: Option<&'static str>,
    pub mission_type: Option<&'static str>,
    pub faction: Option<&'static str>,
    pub tier: ArbitrationTier,
}

pub fn arbitration_node(node_id: &str, tiers: &ArbitrationTiers) -> ArbitrationNode {
    let info = node_info(node_id);
    let (node, planet) = info
        .and_then(|info| node_and_planet(info.value))
        .map_or((node_id, None), |(node, planet)| {
            (node, (!planet.is_empty()).then_some(planet))
        });
    ArbitrationNode {
        node_id: node_id.to_owned(),
        node: node.to_owned(),
        planet,
        mission_type: info.map(|info| info.kind).filter(|kind| !kind.is_empty()),
        faction: info.and_then(|info| info.faction),
        tier: tiers.of(node_id),
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    pub(crate) const FORECAST: &str = include_str!("../../../fixtures/arbitrations.txt");
    pub(crate) const TIERS: &str = include_str!("../../../fixtures/arbitration_tiers.js");
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::fixtures::{FORECAST, TIERS};
    use super::*;
    use crate::window::fixtures::at;

    const FIRST_HOUR_MS: i64 = 1_727_884_800_000;

    #[test]
    fn fixture_forecast() {
        let forecast = ArbitrationForecast::parse(FORECAST).unwrap();
        assert_eq!(forecast.hours(), 48);
        assert_eq!(forecast.starts(), at(FIRST_HOUR_MS));
        assert_eq!(forecast.ends(), at(FIRST_HOUR_MS + 48 * 3_600_000));
        assert_eq!(forecast.hour(0).unwrap().node_id, "SettlementNode11");
        assert_eq!(forecast.hour(47).unwrap().node_id, "SolNode718");
        assert_eq!(forecast.hour(48), None);
        assert_eq!(forecast.unique_node_ids().len(), 38);
    }

    #[test]
    fn index_at_hour_boundaries() {
        let forecast = ArbitrationForecast::parse(FORECAST).unwrap();
        assert_eq!(forecast.index_at(at(FIRST_HOUR_MS - 1)), None);
        assert_eq!(forecast.index_at(at(FIRST_HOUR_MS)), Some(0));
        assert_eq!(forecast.index_at(at(FIRST_HOUR_MS + 3_599_999)), Some(0));
        assert_eq!(forecast.index_at(at(FIRST_HOUR_MS + 3_600_000)), Some(1));
        assert_eq!(
            forecast.index_at(at(FIRST_HOUR_MS + 48 * 3_600_000 - 1)),
            Some(47)
        );
        assert_eq!(forecast.index_at(at(FIRST_HOUR_MS + 48 * 3_600_000)), None);
    }

    #[test]
    fn hours_from_stop_at_the_end() {
        let forecast = ArbitrationForecast::parse(FORECAST).unwrap();
        let hours: Vec<_> = forecast.hours_from(46, 5).collect();
        assert_eq!(hours.len(), 2);
        assert_eq!(hours[0].starts, at(FIRST_HOUR_MS + 46 * 3_600_000));
        assert_eq!(hours[0].ends, at(FIRST_HOUR_MS + 47 * 3_600_000));
        assert_eq!(hours[1].node_id, "SolNode718");
    }

    fn reason(text: &str) -> &'static str {
        match ArbitrationForecast::parse(text).unwrap_err() {
            WorldStateError::Forecast { reason, .. } | WorldStateError::ForecastShape(reason) => {
                reason
            }
            other => panic!("{other}"),
        }
    }

    #[test]
    fn rejects_broken_forecasts() {
        let hours = |count: usize| -> String {
            (0..count).fold(String::new(), |mut text, hour| {
                let _ = writeln!(text, "{},SolNode64", 1_727_884_800 + hour * 3600);
                text
            })
        };
        assert_eq!(reason(""), "shorter than a day");
        assert_eq!(reason(&hours(23)), "shorter than a day");
        assert!(ArbitrationForecast::parse(&hours(24)).is_ok());
        assert_eq!(
            reason(&(hours(24) + "1727971200 SolNode1\n")),
            "expected seconds and a node id"
        );
        assert_eq!(
            reason(&(hours(24) + "1727974800,SolNode1\n")),
            "the hours are not consecutive"
        );
        assert_eq!(
            reason(&(hours(24) + "1727971201,SolNode1\n")),
            "the time is not on the hour"
        );
        assert_eq!(
            reason(&(hours(24) + "1727971200,SolNode000000\n")),
            "the node id has an unknown shape"
        );
        assert_eq!(
            reason(&(hours(24) + "1727971200,CrewBattleNode1\n")),
            "the node id has an unknown shape"
        );
        assert_eq!(
            reason(&(hours(24) + "soon,SolNode1\n")),
            "the seconds are not a number"
        );
    }

    #[test]
    fn fixture_tiers() {
        let tiers = ArbitrationTiers::parse(TIERS).unwrap();
        assert_eq!(tiers.len(), 12);
        assert_eq!(tiers.of("SolNode450"), ArbitrationTier::S);
        assert_eq!(tiers.of("SolNode412"), ArbitrationTier::C);
        assert_eq!(tiers.of("SolNode1"), ArbitrationTier::F);
    }

    #[test]
    fn rejects_broken_tier_tables() {
        assert!(ArbitrationTiers::parse("var x = 1;").is_err());
        assert!(ArbitrationTiers::parse("window.arbyTiers = {};").is_err());
        assert!(ArbitrationTiers::parse("window.arbyTiers = { SolNode1: \"X\" };").is_err());
        assert!(ArbitrationTiers::parse("window.arbyTiers = { Node1: \"S\" };").is_err());
        let trailing =
            ArbitrationTiers::parse("window.arbyTiers = {\n\tSolNode1: \"S\",\n};").unwrap();
        assert_eq!(trailing.len(), 1);
    }

    #[test]
    fn tier_labels() {
        assert_eq!(ArbitrationTier::S.label(), "S");
        assert_eq!(ArbitrationTier::F.label(), "F");
        assert!(ArbitrationTier::S < ArbitrationTier::A);
        assert_eq!(serde_json::to_string(&ArbitrationTier::S).unwrap(), "\"S\"");
    }

    #[test]
    fn nodes_resolve_from_the_table() {
        let tiers = ArbitrationTiers::parse(TIERS).unwrap();
        let umbriel = arbitration_node("SolNode64", &tiers);
        assert_eq!(umbriel.node, "Umbriel");
        assert_eq!(umbriel.planet, Some("Uranus"));
        assert_eq!(umbriel.mission_type, Some("Interception"));
        assert_eq!(umbriel.faction, Some("Grineer"));
        assert_eq!(umbriel.tier, ArbitrationTier::S);

        let unknown = arbitration_node("SolNode99999", &tiers);
        assert_eq!(unknown.node, "SolNode99999");
        assert_eq!(unknown.planet, None);
        assert_eq!(unknown.mission_type, None);
        assert_eq!(unknown.faction, None);
        assert_eq!(unknown.tier, ArbitrationTier::F);
    }
}
