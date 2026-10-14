use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::manifest::ManifestItem;
use crate::manifest::RawManifestItem;
use crate::nodes::node_name;
use crate::window::Window;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VoidTrader {
    #[serde(flatten)]
    pub window: Window,
    pub character: String,
    pub node: String,
    #[serde(default)]
    pub manifest: Vec<RawManifestItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
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
    let present = traders.iter().find(|trader| trader.window.contains(now));
    if let Some(trader) = present {
        return Some(BaroStatus::Present {
            leaves: trader.window.expiry.datetime(),
            character: character_name(&trader.character),
            node_id: trader.node.clone(),
            node_name: node_name(&trader.node),
            items: trader.manifest.iter().map(ManifestItem::from).collect(),
        });
    }
    traders
        .iter()
        .filter(|trader| trader.window.activation.datetime() > now)
        .min_by_key(|trader| trader.window.activation.datetime())
        .map(|trader| BaroStatus::Away {
            arrives: trader.window.activation.datetime(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::fixtures::{at, window};

    fn trader(activation_ms: i64, expiry_ms: i64) -> VoidTrader {
        VoidTrader {
            window: window(activation_ms, expiry_ms),
            character: "Baro'Ki Teel".to_owned(),
            node: "SolNode1".to_owned(),
            manifest: vec![],
        }
    }

    #[test]
    fn away_before_activation() {
        let now = at(0);
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
        let now = at(1500);
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
