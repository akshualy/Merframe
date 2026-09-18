use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::InventoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "DateRepr")]
pub struct MongoDate(DateTime<Utc>);

impl MongoDate {
    pub fn datetime(self) -> DateTime<Utc> {
        self.0
    }

    pub fn timestamp_millis(self) -> i64 {
        self.0.timestamp_millis()
    }
}

impl From<MongoDate> for DateTime<Utc> {
    fn from(value: MongoDate) -> Self {
        value.0
    }
}

#[derive(Deserialize)]
pub struct DateRepr {
    #[serde(rename = "$date")]
    date: NumberLong,
}

#[derive(Deserialize)]
struct NumberLong {
    #[serde(rename = "$numberLong")]
    number_long: String,
}

impl TryFrom<DateRepr> for MongoDate {
    type Error = InventoryError;

    fn try_from(value: DateRepr) -> Result<Self, Self::Error> {
        let millis: i64 = value
            .date
            .number_long
            .parse()
            .map_err(|_| InventoryError::NumberLong(value.date.number_long.clone()))?;
        DateTime::from_timestamp_millis(millis)
            .map(MongoDate)
            .ok_or(InventoryError::Timestamp(millis))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "OidRepr")]
pub struct ObjectId(pub String);

const OID_SECONDS_HEX: usize = 8;

pub fn oid_seconds(oid: &str) -> Option<u32> {
    oid.get(..OID_SECONDS_HEX)
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
}

impl ObjectId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn seconds(&self) -> Option<u32> {
        oid_seconds(&self.0)
    }
}

#[derive(Deserialize)]
pub struct OidRepr {
    #[serde(rename = "$oid")]
    oid: String,
}

impl From<OidRepr> for ObjectId {
    fn from(value: OidRepr) -> Self {
        Self(value.oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn date_from_extended_json() {
        let date: MongoDate =
            serde_json::from_str(r#"{"$date":{"$numberLong":"1579089600000"}}"#).unwrap();
        let dt = date.datetime();
        assert_eq!(date.timestamp_millis(), 1_579_089_600_000);
        assert_eq!((dt.year(), dt.month(), dt.day()), (2020, 1, 15));
        assert_eq!((dt.hour(), dt.minute()), (12, 0));
    }

    #[test]
    fn date_rejects_non_numeric() {
        assert!(serde_json::from_str::<MongoDate>(r#"{"$date":{"$numberLong":"x"}}"#).is_err());
    }

    #[test]
    fn oid_from_extended_json() {
        let oid: ObjectId = serde_json::from_str(r#"{"$oid":"6a9eeb1f000000000000c001"}"#).unwrap();
        assert_eq!(oid.as_str(), "6a9eeb1f000000000000c001");
    }

    #[test]
    fn oid_seconds() {
        use super::*;
        assert_eq!(oid_seconds("6a9fb57d000000000000c101"), Some(0x6a9f_b57d));
        assert_eq!(oid_seconds("6a9fb109000000000000c102"), Some(0x6a9f_b109));
        assert!(oid_seconds("6a9fb57d000000000000c101") > oid_seconds("6a9fb109000000000000c102"));
        assert_eq!(oid_seconds("short"), None);
        assert_eq!(oid_seconds("zzzzzzzz14857beef3052fe1"), None);
    }
}
