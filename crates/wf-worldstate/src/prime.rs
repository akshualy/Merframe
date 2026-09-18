use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::manifest::RawManifestItem;
use crate::mongo_date::MongoDate;
use crate::nodes::node_name;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PrimeVaultTrader {
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub node: String,
    #[serde(default)]
    pub manifest: Vec<RawManifestItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawPrimeAccessAvailability {
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VarziaItem {
    pub item_type: String,
    pub regal_aya: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Varzia {
    pub node_id: String,
    pub node_name: Option<&'static str>,
    pub expiry: DateTime<Utc>,
    pub items: Vec<VarziaItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrimeAccessAvailability {
    pub state: String,
}

pub fn current_varzia(traders: &[PrimeVaultTrader], now: DateTime<Utc>) -> Option<Varzia> {
    let trader = traders
        .iter()
        .find(|trader| now >= trader.activation.utc() && now < trader.expiry.utc())?;
    Some(Varzia {
        node_id: trader.node.clone(),
        node_name: node_name(&trader.node),
        expiry: trader.expiry.utc(),
        items: trader
            .manifest
            .iter()
            .filter_map(|entry| {
                entry.prime_price.map(|regal_aya| VarziaItem {
                    item_type: entry.item_type.clone(),
                    regal_aya,
                })
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    fn trader() -> PrimeVaultTrader {
        PrimeVaultTrader {
            activation: DateTime::<Utc>::from_timestamp_millis(1000).unwrap().into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(2000).unwrap().into(),
            node: "TradeHUB1".to_owned(),
            manifest: vec![
                RawManifestItem {
                    item_type: "/Lotus/StoreItems/Powersuits/Banshee/BansheePrime".to_owned(),
                    prime_price: Some(3),
                    regular_price: None,
                },
                RawManifestItem {
                    item_type: "/Lotus/StoreItems/Types/Game/Projections/T1VoidProjection"
                        .to_owned(),
                    prime_price: None,
                    regular_price: Some(1),
                },
            ],
        }
    }

    #[test]
    fn regal_aya_stock() {
        let now = DateTime::<Utc>::from_timestamp_millis(1500).unwrap();
        let varzia = current_varzia(&[trader()], now).unwrap();
        assert_eq!(varzia.node_name, Some("Maroo's Bazaar (Mars)"));
        assert_eq!(varzia.items.len(), 1);
        assert_eq!(varzia.items[0].regal_aya, 3);
    }

    #[test]
    fn outside_window() {
        let now = DateTime::<Utc>::from_timestamp_millis(2500).unwrap();
        assert!(current_varzia(&[trader()], now).is_none());
    }
}
