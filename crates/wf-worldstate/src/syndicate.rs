use serde::Deserialize;

use crate::mongo_date::MongoDate;

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
        .map(|mission| mission.expiry.utc().timestamp_millis())
}
