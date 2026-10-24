use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wf_mongo::{MongoDate, ObjectId};

use crate::mission_types::mission_type_name;
use crate::nodes::node_info;
use crate::window::Window;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldRewardBundle {
    #[serde(default)]
    items: Vec<String>,
    #[serde(default)]
    counted_items: Vec<CountedItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CountedItem {
    item_type: String,
    item_count: i64,
}

impl WorldRewardBundle {
    fn items(&self) -> BTreeMap<String, i64> {
        let single = self.items.iter().map(|item| (item.as_str(), 1));
        let counted = self
            .counted_items
            .iter()
            .filter(|item| item.item_count > 0)
            .map(|item| (item.item_type.as_str(), item.item_count));
        single
            .chain(counted)
            .map(|(item_type, count)| {
                (
                    item_type.replacen("/Lotus/StoreItems/", "/Lotus/", 1),
                    count,
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawInvasion {
    #[serde(rename = "_id")]
    id: ObjectId,
    faction: String,
    defender_faction: String,
    node: String,
    completed: bool,
    #[serde(default)]
    attacker_reward: WorldRewardBundle,
    #[serde(default)]
    defender_reward: WorldRewardBundle,
    activation: MongoDate,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawAlert {
    #[serde(rename = "_id")]
    id: ObjectId,
    #[serde(flatten)]
    window: Window,
    mission_info: AlertMission,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlertMission {
    mission_type: String,
    location: String,
    #[serde(default)]
    mission_reward: WorldRewardBundle,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawGoal {
    #[serde(rename = "_id")]
    id: ObjectId,
    #[serde(flatten)]
    window: Window,
    #[serde(default)]
    node: Option<String>,
    #[serde(default)]
    goal: i64,
    #[serde(default)]
    interim_goals: Vec<i64>,
    #[serde(default)]
    reward: WorldRewardBundle,
    #[serde(default)]
    interim_rewards: Vec<WorldRewardBundle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum WorldRewardSource {
    Invasion,
    Alert,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRewardOffer {
    pub key: String,
    pub source: WorldRewardSource,
    pub node_id: Option<String>,
    pub node_name: Option<&'static str>,
    pub mission: Option<String>,
    pub side: Option<String>,
    pub event_target: Option<i64>,
    pub item_type: String,
    pub count: i64,
    pub expiry: Option<DateTime<Utc>>,
}

struct Origin<'a> {
    id: String,
    source: WorldRewardSource,
    node_id: Option<&'a str>,
    mission: Option<String>,
    side: Option<String>,
    event_target: Option<i64>,
    expiry: Option<DateTime<Utc>>,
}

impl Origin<'_> {
    fn offers(&self, bundle: &WorldRewardBundle) -> impl Iterator<Item = WorldRewardOffer> {
        let node = self.node_id.and_then(node_info);
        bundle
            .items()
            .into_iter()
            .map(move |(item_type, count)| WorldRewardOffer {
                key: format!("{}#{item_type}", self.id),
                source: self.source,
                node_id: self.node_id.map(str::to_owned),
                node_name: node.map(|node| node.value),
                mission: self
                    .mission
                    .clone()
                    .or_else(|| node.map(|node| node.kind.to_owned())),
                side: self.side.clone(),
                event_target: self.event_target,
                item_type,
                count,
                expiry: self.expiry,
            })
    }
}

fn side_name(faction: &str) -> &'static str {
    match faction {
        "FC_GRINEER" => "Grineer",
        "FC_CORPUS" => "Corpus",
        "FC_INFESTATION" => "Infested",
        _ => "this side",
    }
}

fn origin_id(source: &str, id: &ObjectId, activation: MongoDate) -> String {
    format!("{source}/{}@{}", id.as_str(), activation.timestamp_millis())
}

pub fn world_reward_offers(
    invasions: &[RawInvasion],
    alerts: &[RawAlert],
    goals: &[RawGoal],
    now: DateTime<Utc>,
) -> Vec<WorldRewardOffer> {
    let mut out = Vec::new();
    for invasion in invasions
        .iter()
        .filter(|invasion| !invasion.completed && invasion.activation.datetime() <= now)
    {
        let id = origin_id("invasion", &invasion.id, invasion.activation);
        let sides = [
            ("attacker", &invasion.faction, &invasion.attacker_reward),
            (
                "defender",
                &invasion.defender_faction,
                &invasion.defender_reward,
            ),
        ];
        for (side, faction, bundle) in sides {
            let origin = Origin {
                id: format!("{id}/{side}"),
                source: WorldRewardSource::Invasion,
                node_id: Some(&invasion.node),
                mission: None,
                side: Some(format!("Support {}", side_name(faction))),
                event_target: None,
                expiry: None,
            };
            out.extend(origin.offers(bundle));
        }
    }

    for alert in alerts.iter().filter(|alert| alert.window.contains(now)) {
        let origin = Origin {
            id: origin_id("alert", &alert.id, alert.window.activation),
            source: WorldRewardSource::Alert,
            node_id: Some(&alert.mission_info.location),
            mission: Some(mission_type_name(&alert.mission_info.mission_type)),
            side: None,
            event_target: None,
            expiry: Some(alert.window.expiry.datetime()),
        };
        out.extend(origin.offers(&alert.mission_info.mission_reward));
    }

    for goal in goals.iter().filter(|goal| goal.window.contains(now)) {
        let id = origin_id("event", &goal.id, goal.window.activation);
        let stages = std::iter::once((format!("{id}/goal"), goal.goal, &goal.reward)).chain(
            goal.interim_rewards
                .iter()
                .enumerate()
                .map(|(index, bundle)| {
                    let target = goal.interim_goals.get(index).copied().unwrap_or(0);
                    (format!("{id}/stage{index}"), target, bundle)
                }),
        );
        for (id, target, bundle) in stages {
            let origin = Origin {
                id,
                source: WorldRewardSource::Event,
                node_id: goal.node.as_deref(),
                mission: None,
                side: None,
                event_target: (target > 0).then_some(target),
                expiry: Some(goal.window.expiry.datetime()),
            };
            out.extend(origin.offers(bundle));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::fixtures::at;

    const INVASIONS: &str = r#"[
        {"_id":{"$oid":"6a9e1976d7b7cca80208e4d9"},"Faction":"FC_CORPUS","DefenderFaction":"FC_GRINEER",
         "Node":"SolNode131","Completed":false,
         "AttackerReward":{"countedItems":[{"ItemType":"/Lotus/Types/Items/Research/EnergyComponent","ItemCount":3}]},
         "DefenderReward":{"countedItems":[{"ItemType":"/Lotus/Types/Recipes/Weapons/WeaponParts/DeraVandalBarrel","ItemCount":1}]},
         "Activation":{"$date":{"$numberLong":"1000"}}},
        {"_id":{"$oid":"6a9e1976d7b7cca80208e4da"},"Faction":"FC_INFESTATION","DefenderFaction":"FC_CORPUS",
         "Node":"SolNode50","Completed":true,
         "AttackerReward":{},
         "DefenderReward":{"countedItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/OrokinCatalyst","ItemCount":1}]},
         "Activation":{"$date":{"$numberLong":"1000"}}},
        {"_id":{"$oid":"6a9e1976d7b7cca80208e4db"},"Faction":"FC_GRINEER","DefenderFaction":"FC_CORPUS",
         "Node":"SolNode50","Completed":false,
         "AttackerReward":{},
         "DefenderReward":{"countedItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/OrokinCatalyst","ItemCount":1}]},
         "Activation":{"$date":{"$numberLong":"9000"}}}
    ]"#;

    const ALERTS: &str = r#"[{
        "_id":{"$oid":"6ad0d7380dfe89c5d8229302"},
        "Activation":{"$date":{"$numberLong":"1000"}},"Expiry":{"$date":{"$numberLong":"5000"}},
        "MissionInfo":{"missionType":"MT_HIVE","location":"SolNode153",
          "missionReward":{"credits":17000,"countedItems":[{"ItemType":"/Lotus/Types/Recipes/Components/OrokinReactorBlueprint","ItemCount":1}]}}
    }]"#;

    const GOALS: &str = r#"[{
        "_id":{"$oid":"6a71fe700000000000000000"},
        "Activation":{"$date":{"$numberLong":"1000"}},"Expiry":{"$date":{"$numberLong":"5000"}},
        "Goal":100,"InterimGoals":[25,50],"Node":"EventNode25",
        "InterimRewards":[
          {"items":["/Lotus/StoreItems/Types/StoreItems/AvatarImages/GlyphA"]},
          {"countedItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/WaterFightBucks","ItemCount":50}]}],
        "Reward":{"items":["/Lotus/StoreItems/Types/Items/MiscItems/OrokinCatalyst"],
                  "countedItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/OrokinCatalyst","ItemCount":3},
                                  {"ItemType":"/Lotus/Types/Items/MiscItems/WaterFightBucks","ItemCount":0}]}
    }]"#;

    fn offers(now_ms: i64) -> Vec<WorldRewardOffer> {
        let invasions: Vec<RawInvasion> = serde_json::from_str(INVASIONS).unwrap();
        let alerts: Vec<RawAlert> = serde_json::from_str(ALERTS).unwrap();
        let goals: Vec<RawGoal> = serde_json::from_str(GOALS).unwrap();
        world_reward_offers(&invasions, &alerts, &goals, at(now_ms))
    }

    #[test]
    fn invasion_sides() {
        let offers = offers(2000);
        let invasion: Vec<&WorldRewardOffer> = offers
            .iter()
            .filter(|offer| offer.source == WorldRewardSource::Invasion)
            .collect();
        assert_eq!(invasion.len(), 2);
        assert_eq!(
            invasion[0].key,
            "invasion/6a9e1976d7b7cca80208e4d9@1000/attacker#/Lotus/Types/Items/Research/EnergyComponent"
        );
        assert_eq!(invasion[0].side.as_deref(), Some("Support Corpus"));
        assert_eq!(invasion[0].count, 3);
        assert_eq!(invasion[0].node_name, Some("Pallas (Ceres)"));
        assert_eq!(invasion[0].mission.as_deref(), Some("Extermination"));
        assert_eq!(invasion[0].expiry, None);
        assert_eq!(invasion[1].side.as_deref(), Some("Support Grineer"));
        assert_eq!(
            invasion[1].item_type,
            "/Lotus/Types/Recipes/Weapons/WeaponParts/DeraVandalBarrel"
        );
    }

    #[test]
    fn alert_reward() {
        let active = offers(2000);
        let alert = active
            .iter()
            .find(|offer| offer.source == WorldRewardSource::Alert)
            .unwrap();
        assert_eq!(
            alert.key,
            "alert/6ad0d7380dfe89c5d8229302@1000#/Lotus/Types/Recipes/Components/OrokinReactorBlueprint"
        );
        assert_eq!(alert.mission.as_deref(), Some("Hive"));
        assert_eq!(alert.node_id.as_deref(), Some("SolNode153"));
        assert_eq!(alert.expiry, Some(at(5000)));
        assert!(
            offers(6000)
                .iter()
                .all(|offer| offer.source != WorldRewardSource::Alert)
        );
    }

    #[test]
    fn event_stages() {
        let offers = offers(2000);
        let stages: Vec<(&str, Option<i64>, i64)> = offers
            .iter()
            .filter(|offer| offer.source == WorldRewardSource::Event)
            .map(|offer| (offer.key.as_str(), offer.event_target, offer.count))
            .collect();
        assert_eq!(
            stages,
            vec![
                (
                    "event/6a71fe700000000000000000@1000/goal#/Lotus/Types/Items/MiscItems/OrokinCatalyst",
                    Some(100),
                    3
                ),
                (
                    "event/6a71fe700000000000000000@1000/stage0#/Lotus/Types/StoreItems/AvatarImages/GlyphA",
                    Some(25),
                    1
                ),
                (
                    "event/6a71fe700000000000000000@1000/stage1#/Lotus/Types/Items/MiscItems/WaterFightBucks",
                    Some(50),
                    50
                ),
            ]
        );
    }

    #[test]
    fn side_names() {
        assert_eq!(side_name("FC_INFESTATION"), "Infested");
        assert_eq!(side_name("FC_MITW"), "this side");
    }
}
