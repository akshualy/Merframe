use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::embedded::{SORTIE_BOSSES, SORTIE_MODIFIERS, SortieBoss, lookup};
use crate::mission_types::{mission_type_name, title_case_key};
use crate::mongo_date::MongoDate;
use crate::nodes::node_name;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSortieVariant {
    pub mission_type: String,
    pub modifier_type: String,
    pub node: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawSortie {
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub boss: String,
    pub variants: Vec<RawSortieVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SortieMission {
    pub mission_type: String,
    pub mission_name: String,
    pub modifier: String,
    pub modifier_name: String,
    pub node_id: String,
    pub node_name: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sortie {
    pub boss: String,
    pub boss_name: String,
    pub faction: Option<&'static str>,
    pub activation: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
    pub missions: Vec<SortieMission>,
}

pub fn boss(id: &str) -> Option<&'static SortieBoss> {
    lookup(SORTIE_BOSSES, id)
}

pub fn boss_name(id: &str) -> String {
    match boss(id) {
        Some(boss) => boss.name.to_owned(),
        None => title_case_key(id.strip_prefix("SORTIE_BOSS_").unwrap_or(id)),
    }
}

fn modifier_name(id: &str) -> String {
    match lookup(SORTIE_MODIFIERS, id) {
        Some(modifier) => modifier.name.to_owned(),
        None => title_case_key(id.strip_prefix("SORTIE_MODIFIER_").unwrap_or(id)),
    }
}

pub fn current_sortie(sorties: &[RawSortie], now: DateTime<Utc>) -> Option<Sortie> {
    sorties
        .iter()
        .find(|sortie| now >= sortie.activation.utc() && now < sortie.expiry.utc())
        .map(|sortie| Sortie {
            boss: sortie.boss.clone(),
            boss_name: boss_name(&sortie.boss),
            faction: boss(&sortie.boss).map(|boss| boss.faction),
            activation: sortie.activation.utc(),
            expiry: sortie.expiry.utc(),
            missions: sortie
                .variants
                .iter()
                .map(|variant| SortieMission {
                    mission_type: variant.mission_type.clone(),
                    mission_name: mission_type_name(&variant.mission_type),
                    modifier: variant.modifier_type.clone(),
                    modifier_name: modifier_name(&variant.modifier_type),
                    node_id: variant.node.clone(),
                    node_name: node_name(&variant.node),
                })
                .collect(),
        })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    #[test]
    fn active_sortie() {
        let now = DateTime::<Utc>::from_timestamp_millis(1500).unwrap();
        let sorties = vec![RawSortie {
            activation: DateTime::<Utc>::from_timestamp_millis(1000).unwrap().into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(2000).unwrap().into(),
            boss: "SORTIE_BOSS_KELA".to_owned(),
            variants: vec![
                RawSortieVariant {
                    mission_type: "MT_RETRIEVAL".to_owned(),
                    modifier_type: "SORTIE_MODIFIER_SHOTGUN_ONLY".to_owned(),
                    node: "SolNode1".to_owned(),
                },
                RawSortieVariant {
                    mission_type: "MT_EXCAVATE".to_owned(),
                    modifier_type: "SORTIE_MODIFIER_ARMOR".to_owned(),
                    node: "SolNode1".to_owned(),
                },
                RawSortieVariant {
                    mission_type: "MT_ASSASSINATION".to_owned(),
                    modifier_type: "SORTIE_MODIFIER_EXIMUS".to_owned(),
                    node: "SolNode1".to_owned(),
                },
            ],
        }];
        let sortie = current_sortie(&sorties, now).unwrap();
        assert_eq!(sortie.missions.len(), 3);
        assert_eq!(sortie.boss, "SORTIE_BOSS_KELA");
        assert_eq!(sortie.boss_name, "Kela De Thaym");
        assert_eq!(sortie.faction, Some("Grineer"));
        assert_eq!(
            sortie.missions[0].modifier_name,
            "Weapon Restriction: Shotgun Only"
        );
        assert_eq!(sortie.missions[1].modifier_name, "Augmented Enemy Armor");
        assert_eq!(sortie.missions[2].modifier_name, "Eximus Stronghold");
        assert_eq!(sortie.missions[0].mission_name, "Hijack");
        assert_eq!(sortie.missions[1].mission_name, "Excavation");
        assert_eq!(sortie.missions[2].mission_name, "Assassination");
    }

    #[test]
    fn unknown_boss_and_modifier() {
        assert_eq!(boss_name("SORTIE_BOSS_NEW_GUY"), "New Guy");
        assert!(boss("SORTIE_BOSS_NEW_GUY").is_none());
        assert_eq!(modifier_name("SORTIE_MODIFIER_NEW_THING"), "New Thing");
    }

    #[test]
    fn embedded_tables_sorted() {
        assert!(SORTIE_BOSSES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(
            SORTIE_MODIFIERS
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0)
        );
        assert!(
            SORTIE_BOSSES
                .iter()
                .all(|(id, _)| id.starts_with("SORTIE_BOSS_"))
        );
        assert!(
            SORTIE_MODIFIERS
                .iter()
                .all(|(id, _)| id.starts_with("SORTIE_MODIFIER_"))
        );
    }

    #[test]
    fn no_active_sortie() {
        let now = DateTime::<Utc>::from_timestamp_millis(0).unwrap();
        let sorties = vec![RawSortie {
            activation: DateTime::<Utc>::from_timestamp_millis(1000).unwrap().into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(2000).unwrap().into(),
            boss: "SORTIE_BOSS_KELA".to_owned(),
            variants: vec![],
        }];
        assert!(current_sortie(&sorties, now).is_none());
    }
}
