use serde::Deserialize;
use wf_mongo::MongoDate;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SyndicateMission {
    pub expiry: MongoDate,
    pub tag: String,
}

pub fn expiry_for_tag(missions: &[SyndicateMission], tag: &str) -> Option<i64> {
    missions
        .iter()
        .find(|mission| mission.tag == tag)
        .map(|mission| mission.expiry.timestamp_millis())
}
