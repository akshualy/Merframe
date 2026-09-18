use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Deserializer;

#[derive(Deserialize)]
struct NumberLong {
    #[serde(rename = "$numberLong")]
    number_long: String,
}

#[derive(Deserialize)]
struct DateWrapper {
    #[serde(rename = "$date")]
    date: NumberLong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MongoDate(DateTime<Utc>);

impl MongoDate {
    pub fn utc(self) -> DateTime<Utc> {
        self.0
    }
}

impl From<DateTime<Utc>> for MongoDate {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value)
    }
}

impl<'de> Deserialize<'de> for MongoDate {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wrapper = DateWrapper::deserialize(deserializer)?;
        let millis: i64 = wrapper
            .date
            .number_long
            .parse()
            .map_err(serde::de::Error::custom)?;
        DateTime::from_timestamp_millis(millis)
            .map(MongoDate)
            .ok_or_else(|| serde::de::Error::custom("mongo date out of range"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_number_long() {
        let raw = r#"{"$date":{"$numberLong":"1788801182502"}}"#;
        let parsed: MongoDate = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.utc().timestamp_millis(), 1_788_801_182_502);
    }

    #[test]
    fn malformed_number() {
        let raw = r#"{"$date":{"$numberLong":"not-a-number"}}"#;
        let parsed: std::result::Result<MongoDate, _> = serde_json::from_str(raw);
        assert!(parsed.is_err());
    }

    #[test]
    fn out_of_range_millis() {
        let raw = r#"{"$date":{"$numberLong":"9223372036854775807"}}"#;
        let parsed: std::result::Result<MongoDate, _> = serde_json::from_str(raw);
        assert!(parsed.is_err());
    }
}
