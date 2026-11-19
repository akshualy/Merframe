use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, WorldStateError};
use crate::nodes::{node_and_planet, node_info, node_name};

pub const ARBITRATION_FORECAST_URL: &str = "https://browse.wf/arbys.txt";

const HOUR_SECS: i64 = 3600;
const SHORTEST_FORECAST_HOURS: usize = 24;

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
    #[serde(rename = "SP")]
    Special,
    F,
}

impl ArbitrationTier {
    pub fn of(node_id: &str) -> Self {
        node_name(node_id)
            .and_then(|name| {
                ARBITRATION_TIER_NODE_NAMES
                    .into_iter()
                    .find(|(_, names)| names.contains(&name))
            })
            .map_or(Self::F, |(tier, _)| tier)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::S => "S",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::Special => "Special",
            Self::F => "F",
        }
    }
}

const ARBITRATION_TIER_NODE_NAMES: [(ArbitrationTier, &[&str]); 6] = [
    (
        ArbitrationTier::S,
        &[
            "Tyana Pass (Mars)",
            "Alator (Mars)",
            "Callisto (Jupiter)",
            "Cytherean (Venus)",
            "Xini (Eris)",
            "Munio (Deimos)",
        ],
    ),
    (
        ArbitrationTier::A,
        &[
            "Sinai (Jupiter)",
            "Seimeni (Ceres)",
            "Casta (Ceres)",
            "Cinxia (Ceres)",
        ],
    ),
    (
        ArbitrationTier::B,
        &[
            "Spear (Mars)",
            "Kadesh (Mars)",
            "Akkad (Eris)",
            "Kala-azar (Eris)",
            "Coba (Earth)",
            "Lith (Earth)",
            "Sechura (Pluto)",
            "Larzac (Europa)",
            "Belenus (Void)",
            "Taranis (Void)",
            "Hyf (Deimos)",
            "Oestrus (Eris)",
        ],
    ),
    (
        ArbitrationTier::C,
        &[
            "Hydron (Sedna)",
            "Helene (Saturn)",
            "Ose (Europa)",
            "Mithra (Void)",
            "Paimon (Europa)",
            "Tessera (Venus)",
            "Odin (Mercury)",
            "Io (Jupiter)",
            "Stephano (Uranus)",
            "Gulliver (Phobos)",
            "Romula (Venus)",
            "Proteus (Neptune)",
            "Outer Terminus (Pluto)",
        ],
    ),
    (
        ArbitrationTier::D,
        &[
            "Umbriel (Uranus)",
            "Cerberus (Pluto)",
            "Gaia (Earth)",
            "Stöfler (Lua)",
            "Lares (Mercury)",
        ],
    ),
    (
        ArbitrationTier::Special,
        &[
            "Terrorem (Deimos)",
            "Ani (Void)",
            "Mot (Void)",
            "Kappa (Sedna)",
            "Ur (Uranus)",
            "Laomedeia (Neptune)",
            "Apollo (Lua)",
            "Ganymede (Jupiter)",
        ],
    ),
];

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

pub fn arbitration_node(node_id: &str) -> ArbitrationNode {
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
        tier: ArbitrationTier::of(node_id),
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    pub(crate) const FORECAST: &str = include_str!("../../../fixtures/arbitrations.txt");
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::fixtures::FORECAST;
    use super::*;
    use crate::embedded::SOL_NODES;
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
    fn tiered_node_names_exist() {
        for (tier, names) in ARBITRATION_TIER_NODE_NAMES {
            for name in names {
                let (node_id, _) = SOL_NODES
                    .iter()
                    .find(|(_, node)| node.value == *name)
                    .unwrap_or_else(|| panic!("{name}"));
                assert_eq!(ArbitrationTier::of(node_id), tier);
            }
        }
    }

    #[test]
    fn tier_order_and_wire_name() {
        assert!(ArbitrationTier::D < ArbitrationTier::Special);
        assert!(ArbitrationTier::Special < ArbitrationTier::F);
        assert_eq!(
            serde_json::to_string(&ArbitrationTier::Special).unwrap(),
            "\"SP\""
        );
    }

    #[test]
    fn nodes_resolve_from_the_table() {
        let umbriel = arbitration_node("SolNode64");
        assert_eq!(umbriel.node, "Umbriel");
        assert_eq!(umbriel.planet, Some("Uranus"));
        assert_eq!(umbriel.mission_type, Some("Interception"));
        assert_eq!(umbriel.faction, Some("Grineer"));
        assert_eq!(umbriel.tier, ArbitrationTier::D);

        let unknown = arbitration_node("SolNode99999");
        assert_eq!(unknown.node, "SolNode99999");
        assert_eq!(unknown.planet, None);
        assert_eq!(unknown.mission_type, None);
        assert_eq!(unknown.faction, None);
        assert_eq!(unknown.tier, ArbitrationTier::F);
    }
}
