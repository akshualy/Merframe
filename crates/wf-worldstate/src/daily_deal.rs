use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;

use crate::mongo_date::MongoDate;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawDailyDeal {
    pub store_item: String,
    pub activation: MongoDate,
    pub expiry: MongoDate,
    pub discount: u32,
    pub original_price: u32,
    pub sale_price: u32,
    pub amount_total: u32,
    pub amount_sold: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DailyDeal {
    pub store_item: String,
    pub activation: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
    pub discount_percent: u32,
    pub original_price: u32,
    pub sale_price: u32,
    pub amount_total: u32,
    pub amount_sold: u32,
}

impl From<&RawDailyDeal> for DailyDeal {
    fn from(raw: &RawDailyDeal) -> Self {
        Self {
            store_item: raw.store_item.clone(),
            activation: raw.activation.utc(),
            expiry: raw.expiry.utc(),
            discount_percent: raw.discount,
            original_price: raw.original_price,
            sale_price: raw.sale_price,
            amount_total: raw.amount_total,
            amount_sold: raw.amount_sold,
        }
    }
}

pub fn daily_deals(raw: &[RawDailyDeal], now: DateTime<Utc>) -> Vec<DailyDeal> {
    raw.iter()
        .filter(|deal| now >= deal.activation.utc() && now < deal.expiry.utc())
        .map(DailyDeal::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    fn deal(activation_ms: i64, expiry_ms: i64) -> RawDailyDeal {
        RawDailyDeal {
            store_item: "/Lotus/StoreItems/Types/Items/Research/BioComponent".to_owned(),
            activation: DateTime::<Utc>::from_timestamp_millis(activation_ms)
                .unwrap()
                .into(),
            expiry: DateTime::<Utc>::from_timestamp_millis(expiry_ms)
                .unwrap()
                .into(),
            discount: 20,
            original_price: 10,
            sale_price: 8,
            amount_total: 165,
            amount_sold: 165,
        }
    }

    fn at(millis: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp_millis(millis).unwrap()
    }

    #[test]
    fn running_deal() {
        let deals = daily_deals(&[deal(0, 1000)], at(500));
        assert_eq!(deals.len(), 1);
        assert_eq!(deals[0].discount_percent, 20);
        assert_eq!(deals[0].sale_price, 8);
    }

    #[test]
    fn deals_outside_window() {
        let raw = vec![deal(2000, 3000), deal(0, 1000)];
        assert!(daily_deals(&raw, at(1500)).is_empty());
    }
}
