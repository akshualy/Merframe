use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::manifest::ManifestItem;
use crate::manifest::RawManifestItem;
use crate::mongo_date::MongoDate;
use crate::nodes::node_name;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VoidTrader {
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub character: String,
    pub node: String,
    #[serde(default)]
    pub manifest: Vec<RawManifestItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BaroStatus {
    Away {
        arrives: DateTime<Utc>,
    },
    Present {
        leaves: DateTime<Utc>,
        character: String,
        node_id: String,
        node_name: Option<&'static str>,
        items: Vec<ManifestItem>,
    },
}

fn character_name(shipped: &str) -> String {
    if shipped == "Baro'Ki Teel" {
        "Baro Ki'Teer".to_owned()
    } else {
        shipped.to_owned()
    }
}

pub fn baro_status(traders: &[VoidTrader], now: DateTime<Utc>) -> Option<BaroStatus> {
    let present = traders
        .iter()
        .find(|trader| now >= trader.activation.utc() && now < trader.expiry.utc());
    if let Some(trader) = present {
        return Some(BaroStatus::Present {
            leaves: trader.expiry.utc(),
            character: character_name(&trader.character),
            node_id: trader.node.clone(),
            node_name: node_name(&trader.node),
            items: trader.manifest.iter().map(ManifestItem::from).collect(),
        });
    }
    traders
        .iter()
        .filter(|trader| trader.activation.utc() > now)
        .min_by_key(|trader| trader.activation.utc())
        .map(|trader| BaroStatus::Away {
            arrives: trader.activation.utc(),
        })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    fn trader(activation_ms: i64, expiry_ms: i64) -> VoidTrader {
        VoidTrader {
            activation: DateTime::<Utc>::from_timestamp_millis(activation_ms)
                .unwrap()
                .into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(expiry_ms)
                .unwrap()
                .into(),
            character: "Baro'Ki Teel".to_owned(),
            node: "SolNode1".to_owned(),
            manifest: vec![],
        }
    }

    #[test]
    fn away_before_activation() {
        let now = DateTime::<Utc>::from_timestamp_millis(0).unwrap();
        let traders = vec![trader(1000, 2000)];
        match baro_status(&traders, now) {
            Some(BaroStatus::Away { arrives }) => {
                assert_eq!(arrives.timestamp_millis(), 1000);
            }
            other => panic!("expected Away, got {other:?}"),
        }
    }

    #[test]
    fn wire_character_spelling() {
        assert_eq!(character_name("Baro'Ki Teel"), "Baro Ki'Teer");
        assert_eq!(character_name("Varzia"), "Varzia");
    }

    #[test]
    fn present_within_window() {
        let now = DateTime::<Utc>::from_timestamp_millis(1500).unwrap();
        let traders = vec![trader(1000, 2000)];
        match baro_status(&traders, now) {
            Some(BaroStatus::Present {
                node_name,
                character,
                ..
            }) => {
                assert_eq!(node_name, Some("Galatea (Neptune)"));
                assert_eq!(character, "Baro Ki'Teer");
            }
            other => panic!("expected Present, got {other:?}"),
        }
    }
}
