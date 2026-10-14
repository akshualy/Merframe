use chrono::{DateTime, Utc};
use serde::Deserialize;
use wf_mongo::MongoDate;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Window {
    pub activation: MongoDate,
    pub expiry: MongoDate,
}

impl Window {
    pub fn contains(self, now: DateTime<Utc>) -> bool {
        now >= self.activation.datetime() && now < self.expiry.datetime()
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use chrono::{DateTime, Utc};

    use super::Window;

    pub(crate) fn at(millis: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp_millis(millis).unwrap()
    }

    pub(crate) fn window(activation_ms: i64, expiry_ms: i64) -> Window {
        Window {
            activation: at(activation_ms).into(),
            expiry: at(expiry_ms).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{at, window};

    #[test]
    fn half_open_interval() {
        let window = window(1000, 2000);
        assert!(!window.contains(at(999)));
        assert!(window.contains(at(1000)));
        assert!(window.contains(at(1999)));
        assert!(!window.contains(at(2000)));
    }
}
