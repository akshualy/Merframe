use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use wf_core::{Catalog, StoredTrade, Turnover, market_icon, traded_set};
use wf_market::{Item, OrderType};

use crate::market::{ItemTable, english_name};
use crate::runtime::{listed_item, trade_side};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeCategory {
    Set,
    Prime,
    Riven,
    Arcane,
    Relic,
    Mod,
    Other,
}

impl TradeCategory {
    fn of(item: &Item) -> Self {
        let tagged = |tag: &str| item.tags.iter().any(|owned| owned == tag);
        if item.slug.contains("riven_mod") {
            Self::Riven
        } else if tagged("set") {
            Self::Set
        } else if tagged("prime") {
            Self::Prime
        } else if tagged("arcane") || tagged("arcane_enhancement") {
            Self::Arcane
        } else if tagged("relic") {
            Self::Relic
        } else if tagged("mod") {
            Self::Mod
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MarketMover {
    pub slug: String,
    pub name: String,
    pub image_name: Option<String>,
    pub category: TradeCategory,
    pub unit_price: f64,
    pub volume: u32,
    pub value: f64,
    pub price_change: Option<f64>,
    pub volume_change: Option<f64>,
}

pub fn market_movers(
    table: &ItemTable,
    catalog: &Catalog,
    turnover: &HashMap<String, Turnover>,
) -> Vec<MarketMover> {
    table
        .items()
        .iter()
        .filter_map(|item| {
            let turnover = turnover.get(&item.slug)?;
            Some(MarketMover {
                slug: item.slug.clone(),
                name: english_name(item),
                image_name: market_icon(catalog, item),
                category: TradeCategory::of(item),
                unit_price: turnover.unit_price,
                volume: turnover.volume,
                value: turnover.value(),
                price_change: turnover.price_change,
                volume_change: turnover.volume_change,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CategoryStatement {
    pub category: TradeCategory,
    pub revenue: i64,
    pub expenses: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TradedTotal {
    pub name: String,
    pub image_name: Option<String>,
    pub amount: i64,
    pub value: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TradeAnalytics {
    pub categories: Vec<CategoryStatement>,
    pub sold: Vec<TradedTotal>,
    pub bought: Vec<TradedTotal>,
}

fn add_traded(totals: &mut Vec<TradedTotal>, traded: TradedTotal) {
    match totals.iter_mut().find(|known| known.name == traded.name) {
        Some(known) => {
            known.amount += traded.amount;
            known.value += traded.value;
        }
        None => totals.push(traded),
    }
}

pub fn trade_analytics(
    trades: &[StoredTrade],
    table: &ItemTable,
    catalog: &Catalog,
) -> TradeAnalytics {
    let mut analytics = TradeAnalytics::default();
    let mut categories: BTreeMap<TradeCategory, CategoryStatement> = BTreeMap::new();
    for stored in trades {
        let Some((side, items, plat)) = trade_side(&stored.trade) else {
            continue;
        };
        let set = traded_set(catalog, items);
        let Some(item) = set.as_ref().or(items.first()) else {
            continue;
        };
        let listed = listed_item(table, &item.name);
        let category = match listed {
            Some(listed) => TradeCategory::of(listed),
            None if item.rank.is_some() => TradeCategory::Riven,
            None => TradeCategory::Other,
        };
        let value = i64::from(plat);
        let total = TradedTotal {
            name: listed.map_or_else(|| item.name.clone(), english_name),
            image_name: listed.and_then(|listed| market_icon(catalog, listed)),
            amount: item.count,
            value,
        };
        let statement = categories.entry(category).or_insert(CategoryStatement {
            category,
            revenue: 0,
            expenses: 0,
        });
        match side {
            OrderType::Sell => {
                statement.revenue += value;
                add_traded(&mut analytics.sold, total);
            }
            OrderType::Buy => {
                statement.expenses += value;
                add_traded(&mut analytics.bought, total);
            }
        }
    }
    analytics.categories = categories.into_values().collect();
    analytics
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use wf_core::{Trade, TradeItem};

    use super::*;

    const ITEMS: &str = r#"[
        {"id":"1","slug":"braton_prime_set","gameRef":"","tags":["set","prime","weapon"],"i18n":{"en":{"name":"Braton Prime Set","icon":"","thumb":""}}},
        {"id":"2","slug":"braton_prime_barrel","gameRef":"","tags":["component","prime"],"i18n":{"en":{"name":"Braton Prime Barrel","icon":"","thumb":""}}},
        {"id":"3","slug":"arcane_energize","gameRef":"","tags":["arcane_enhancement"],"i18n":{"en":{"name":"Arcane Energize","icon":"","thumb":""}}},
        {"id":"4","slug":"axi_a21_relic","gameRef":"","tags":["relic","axi"],"i18n":{"en":{"name":"Axi A21 Relic","icon":"","thumb":""}}},
        {"id":"5","slug":"primed_flow","gameRef":"","tags":["mod","legendary"],"i18n":{"en":{"name":"Primed Flow","icon":"","thumb":""}}},
        {"id":"6","slug":"rifle_riven_mod_veiled","gameRef":"","tags":["mod"],"i18n":{"en":{"name":"Rifle Riven Mod (Veiled)","icon":"","thumb":""}}},
        {"id":"7","slug":"ayatan_anasa_sculpture","gameRef":"","tags":["sculpture"],"i18n":{"en":{"name":"Ayatan Anasa Sculpture","icon":"","thumb":""}}}
    ]"#;

    fn table() -> ItemTable {
        ItemTable::new(serde_json::from_str(ITEMS).unwrap(), Utc::now())
    }

    fn catalog() -> Catalog {
        Catalog::from_json("[]", "[]", "[]").unwrap()
    }

    fn item(name: &str, count: i64, rank: Option<u32>) -> TradeItem {
        TradeItem {
            name: name.to_owned(),
            count,
            rank,
        }
    }

    fn trade(
        partner: &str,
        offered: Vec<TradeItem>,
        received: Vec<TradeItem>,
        plat: i64,
    ) -> StoredTrade {
        StoredTrade {
            id: 0,
            at: DateTime::UNIX_EPOCH,
            partner: Some(partner.to_owned()),
            trade: Trade {
                offered,
                received,
                plat,
            },
        }
    }

    #[test]
    fn categories_from_market_tags() {
        let table = table();
        let categories: Vec<TradeCategory> = table.items().iter().map(TradeCategory::of).collect();
        assert_eq!(
            categories,
            [
                TradeCategory::Set,
                TradeCategory::Prime,
                TradeCategory::Arcane,
                TradeCategory::Relic,
                TradeCategory::Mod,
                TradeCategory::Riven,
                TradeCategory::Other,
            ]
        );
    }

    #[test]
    fn movers_carry_weekly_turnover() {
        let turnover = HashMap::from([
            (
                String::from("braton_prime_set"),
                Turnover {
                    unit_price: 45.5,
                    volume: 61,
                    price_change: Some(-0.01),
                    volume_change: None,
                },
            ),
            (
                String::from("not_on_the_item_list"),
                Turnover {
                    unit_price: 1.0,
                    volume: 1,
                    price_change: None,
                    volume_change: None,
                },
            ),
        ]);
        let movers = market_movers(&table(), &catalog(), &turnover);
        assert_eq!(movers.len(), 1);
        assert_eq!(movers[0].name, "Braton Prime Set");
        assert_eq!(movers[0].category, TradeCategory::Set);
        assert_eq!(movers[0].volume, 61);
        assert_eq!(movers[0].price_change, Some(-0.01));
        assert_eq!(movers[0].volume_change, None);
        assert!((movers[0].value - 2775.5).abs() < f64::EPSILON);
    }

    #[test]
    fn sales_and_purchases_totalled() {
        let barrel = |count| vec![item("Braton Prime Barrel", count, None)];
        let trades = [
            trade("TestSquadA", barrel(2), Vec::new(), 40),
            trade("TestSquadA", barrel(1), Vec::new(), 25),
            trade(
                "TestSquadB",
                vec![item("Torid Visi-critatis", 1, Some(8))],
                Vec::new(),
                300,
            ),
            trade(
                "TestSquadB",
                Vec::new(),
                vec![
                    item("Axi A21 Relic [RADIANT]", 3, None),
                    item("Primed Flow", 1, Some(0)),
                ],
                -15,
            ),
            trade(
                "TestSquadB",
                vec![item("Primed Flow", 1, Some(10))],
                vec![item("Ayatan Anasa Sculpture", 1, None)],
                0,
            ),
        ];
        let analytics = trade_analytics(&trades, &table(), &catalog());
        let categories: Vec<_> = analytics
            .categories
            .iter()
            .map(|entry| (entry.category, entry.revenue, entry.expenses))
            .collect();
        assert_eq!(
            categories,
            [
                (TradeCategory::Prime, 65, 0),
                (TradeCategory::Riven, 300, 0),
                (TradeCategory::Relic, 0, 15),
            ]
        );
        let totals = |traded: &[TradedTotal]| -> Vec<(String, i64, i64)> {
            traded
                .iter()
                .map(|total| (total.name.clone(), total.amount, total.value))
                .collect()
        };
        assert_eq!(
            totals(&analytics.sold),
            [
                (String::from("Braton Prime Barrel"), 3, 65),
                (String::from("Torid Visi-critatis"), 1, 300),
            ]
        );
        assert_eq!(
            totals(&analytics.bought),
            [(String::from("Axi A21 Relic"), 3, 15)]
        );
    }
}
