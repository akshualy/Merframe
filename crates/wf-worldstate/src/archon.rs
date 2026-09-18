use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::mission_types::mission_type_name;
use crate::mongo_date::MongoDate;
use crate::nodes::node_name;
use crate::sortie::{boss, boss_name};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteSortieMission {
    pub mission_type: String,
    pub node: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LiteSortie {
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub boss: String,
    pub missions: Vec<LiteSortieMission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchonMission {
    pub mission_type: String,
    pub mission_name: String,
    pub node_id: String,
    pub node_name: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchonHunt {
    pub boss: String,
    pub boss_name: String,
    pub faction: Option<&'static str>,
    pub activation: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
    pub missions: Vec<ArchonMission>,
}

pub fn current_archon_hunt(sorties: &[LiteSortie], now: DateTime<Utc>) -> Option<ArchonHunt> {
    sorties
        .iter()
        .find(|sortie| now >= sortie.activation.utc() && now < sortie.expiry.utc())
        .map(|sortie| ArchonHunt {
            boss: sortie.boss.clone(),
            boss_name: boss_name(&sortie.boss),
            faction: boss(&sortie.boss).map(|boss| boss.faction),
            activation: sortie.activation.utc(),
            expiry: sortie.expiry.utc(),
            missions: sortie
                .missions
                .iter()
                .map(|mission| ArchonMission {
                    mission_type: mission.mission_type.clone(),
                    mission_name: mission_type_name(&mission.mission_type),
                    node_id: mission.node.clone(),
                    node_name: node_name(&mission.node),
                })
                .collect(),
        })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    #[test]
    fn active_hunt() {
        let now = DateTime::<Utc>::from_timestamp_millis(1500).unwrap();
        let sorties = vec![LiteSortie {
            activation: DateTime::<Utc>::from_timestamp_millis(1000).unwrap().into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(2000).unwrap().into(),
            boss: "SORTIE_BOSS_AMAR".to_owned(),
            missions: vec![
                LiteSortieMission {
                    mission_type: "MT_RESCUE".to_owned(),
                    node: "SolNode1".to_owned(),
                },
                LiteSortieMission {
                    mission_type: "MT_TERRITORY".to_owned(),
                    node: "SolNode1".to_owned(),
                },
                LiteSortieMission {
                    mission_type: "MT_ASSASSINATION".to_owned(),
                    node: "SolNode1".to_owned(),
                },
            ],
        }];
        let hunt = current_archon_hunt(&sorties, now).unwrap();
        assert_eq!(hunt.missions.len(), 3);
        assert_eq!(hunt.boss, "SORTIE_BOSS_AMAR");
        assert_eq!(hunt.boss_name, "Archon Amar");
        assert_eq!(hunt.faction, Some("Narmer"));
        assert_eq!(hunt.missions[0].mission_name, "Rescue");
        assert_eq!(hunt.missions[1].mission_name, "Interception");
        assert_eq!(hunt.missions[2].mission_name, "Assassination");
    }
}
