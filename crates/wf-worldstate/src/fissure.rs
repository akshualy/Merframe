use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use tracing::warn;

use crate::mission_types::mission_type_name;
use crate::mongo_date::MongoDate;
use crate::nodes::{node_info, node_name};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActiveMission {
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub node: String,
    pub mission_type: String,
    pub modifier: String,
    #[serde(default)]
    pub hard: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VoidStorm {
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub node: String,
    pub active_mission_tier: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum RelicTier {
    Lith,
    Meso,
    Neo,
    Axi,
    Requiem,
    Omnia,
}

impl RelicTier {
    pub fn from_modifier(modifier: &str) -> Option<Self> {
        match modifier {
            "VoidT1" => Some(Self::Lith),
            "VoidT2" => Some(Self::Meso),
            "VoidT3" => Some(Self::Neo),
            "VoidT4" => Some(Self::Axi),
            "VoidT5" => Some(Self::Requiem),
            "VoidT6" => Some(Self::Omnia),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Lith => "Lith",
            Self::Meso => "Meso",
            Self::Neo => "Neo",
            Self::Axi => "Axi",
            Self::Requiem => "Requiem",
            Self::Omnia => "Omnia",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(untagged)]
pub enum FissureTier {
    Relic(RelicTier),
    Unknown(String),
}

static UNKNOWN_MODIFIERS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

impl FissureTier {
    pub fn from_modifier(modifier: &str) -> Self {
        if let Some(tier) = RelicTier::from_modifier(modifier) {
            return Self::Relic(tier);
        }
        let mut reported = UNKNOWN_MODIFIERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if reported.insert(modifier.to_owned()) {
            warn!("Unknown fissure modifier {modifier}");
        }
        Self::Unknown(modifier.to_owned())
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Relic(tier) => tier.name(),
            Self::Unknown(modifier) => modifier,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fissure {
    pub node_id: String,
    pub node_name: Option<&'static str>,
    pub mission_type: String,
    pub mission_name: String,
    pub tier: FissureTier,
    pub steel_path: bool,
    pub is_storm: bool,
    pub activation: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
}

pub fn active_fissures(
    missions: &[ActiveMission],
    storms: &[VoidStorm],
    now: DateTime<Utc>,
) -> Vec<Fissure> {
    let running =
        |activation: &MongoDate, expiry: &MongoDate| now >= activation.utc() && now < expiry.utc();
    let mut fissures: Vec<Fissure> = missions
        .iter()
        .filter(|mission| running(&mission.activation, &mission.expiry))
        .map(|mission| Fissure {
            node_id: mission.node.clone(),
            node_name: node_name(&mission.node),
            mission_type: mission.mission_type.clone(),
            mission_name: mission_type_name(&mission.mission_type),
            tier: FissureTier::from_modifier(&mission.modifier),
            steel_path: mission.hard,
            is_storm: false,
            activation: mission.activation.utc(),
            expiry: mission.expiry.utc(),
        })
        .collect();
    fissures.extend(
        storms
            .iter()
            .filter(|storm| running(&storm.activation, &storm.expiry))
            .map(|storm| {
                let mission = node_info(&storm.node).map_or("Void Storm", |node| node.kind);
                Fissure {
                    node_id: storm.node.clone(),
                    node_name: node_name(&storm.node),
                    mission_type: mission.to_owned(),
                    mission_name: mission.to_owned(),
                    tier: FissureTier::from_modifier(&storm.active_mission_tier),
                    steel_path: false,
                    is_storm: true,
                    activation: storm.activation.utc(),
                    expiry: storm.expiry.utc(),
                }
            }),
    );
    fissures.sort_by(|left, right| left.tier.cmp(&right.tier));
    fissures
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    fn mission(modifier: &str, hard: bool, activation_ms: i64, expiry_ms: i64) -> ActiveMission {
        ActiveMission {
            activation: DateTime::<Utc>::from_timestamp_millis(activation_ms)
                .unwrap()
                .into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(expiry_ms)
                .unwrap()
                .into(),
            node: "SolNode1".to_owned(),
            mission_type: "MT_ARTIFACT".to_owned(),
            modifier: modifier.to_owned(),
            hard,
        }
    }

    #[test]
    fn active_fissures_by_tier() {
        let now = DateTime::<Utc>::from_timestamp_millis(1000).unwrap();
        let missions = vec![
            mission("VoidT4", false, 0, 2000),
            mission("VoidT1", true, 0, 2000),
            mission("VoidT3", false, 5000, 6000),
        ];
        let fissures = active_fissures(&missions, &[], now);
        assert_eq!(fissures.len(), 2);
        assert_eq!(fissures[0].tier, FissureTier::Relic(RelicTier::Lith));
        assert!(fissures[0].steel_path);
        assert!(!fissures[0].is_storm);
        assert_eq!(fissures[1].tier, FissureTier::Relic(RelicTier::Axi));
        assert_eq!(fissures[0].node_name, Some("Galatea (Neptune)"));
        assert_eq!(fissures[0].mission_type, "MT_ARTIFACT");
        assert_eq!(fissures[0].mission_name, "Disruption");
    }

    #[test]
    fn void_storm_railjack_fissures() {
        let now = DateTime::<Utc>::from_timestamp_millis(1000).unwrap();
        let storm = |node: &str, tier: &str, activation_ms: i64, expiry_ms: i64| VoidStorm {
            activation: DateTime::<Utc>::from_timestamp_millis(activation_ms)
                .unwrap()
                .into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(expiry_ms)
                .unwrap()
                .into(),
            node: node.to_owned(),
            active_mission_tier: tier.to_owned(),
        };
        let storms = vec![
            storm("CrewBattleNode531", "VoidT4", 0, 2000),
            storm("CrewBattleNode519", "VoidT1", 0, 2000),
            storm("CrewBattleNode511", "VoidT1", 5000, 6000),
        ];
        let fissures = active_fissures(&[mission("VoidT2", false, 0, 2000)], &storms, now);
        assert_eq!(fissures.len(), 3);
        assert!(fissures[0].is_storm);
        assert_eq!(fissures[0].tier, FissureTier::Relic(RelicTier::Lith));
        assert_eq!(fissures[0].node_name, Some("Korm's Belt (Earth)"));
        assert_eq!(fissures[0].mission_name, "Skirmish");
        assert!(!fissures[1].is_storm);
        assert_eq!(fissures[2].node_name, Some("Fenton's Field (Pluto)"));
        assert_eq!(fissures[2].mission_type, "Survival");
        assert!(!fissures[2].steel_path);
    }

    #[test]
    fn unknown_modifier_fissure() {
        let now = DateTime::<Utc>::from_timestamp_millis(1000).unwrap();
        let missions = vec![
            mission("VoidT7", false, 0, 2000),
            mission("VoidT1", false, 0, 2000),
        ];
        let fissures = active_fissures(&missions, &[], now);
        assert_eq!(fissures.len(), 2);
        assert_eq!(fissures[0].tier, FissureTier::Relic(RelicTier::Lith));
        assert_eq!(fissures[1].tier, FissureTier::Unknown("VoidT7".to_owned()));
        assert_eq!(fissures[1].tier.name(), "VoidT7");
        assert_eq!(
            serde_json::to_string(&fissures[1].tier).unwrap(),
            "\"VoidT7\""
        );
        assert_eq!(
            serde_json::to_string(&fissures[0].tier).unwrap(),
            "\"Lith\""
        );
    }
}
