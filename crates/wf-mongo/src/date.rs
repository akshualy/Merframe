use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{MongoError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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

impl From<DateTime<Utc>> for MongoDate {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value)
    }
}

impl From<MongoDate> for DateTime<Utc> {
    fn from(value: MongoDate) -> Self {
        value.0
    }
}

#[derive(Deserialize)]
struct DateRepr {
    #[serde(rename = "$date")]
    date: NumberLong,
}

#[derive(Deserialize)]
struct NumberLong {
    #[serde(rename = "$numberLong")]
    number_long: String,
}

impl TryFrom<DateRepr> for MongoDate {
    type Error = MongoError;

    fn try_from(value: DateRepr) -> Result<Self> {
        let millis: i64 = value
            .date
            .number_long
            .parse()
            .map_err(|_| MongoError::NumberLong(value.date.number_long.clone()))?;
        DateTime::from_timestamp_millis(millis)
            .map(MongoDate)
            .ok_or(MongoError::Timestamp(millis))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    use super::*;

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
    fn date_rejects_out_of_range_millis() {
        let raw = r#"{"$date":{"$numberLong":"9223372036854775807"}}"#;
        assert!(serde_json::from_str::<MongoDate>(raw).is_err());
    }
}
