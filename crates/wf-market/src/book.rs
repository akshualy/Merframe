use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::models::{Order, OrderType, UserStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub sell: Vec<Order>,
    pub buy: Vec<Order>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraderStatus {
    #[default]
    Ingame,
    Online,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    pub status: TraderStatus,
    pub locale: Option<String>,
}

impl Reach {
    fn covers(&self, order: &Order) -> bool {
        order.user.as_ref().is_some_and(|user| {
            let status = match self.status {
                TraderStatus::Ingame => user.status == Some(UserStatus::Ingame),
                TraderStatus::Online => {
                    matches!(user.status, Some(UserStatus::Ingame | UserStatus::Online))
                }
                TraderStatus::Any => true,
            };
            status
                && self
                    .locale
                    .as_ref()
                    .is_none_or(|locale| *locale == user.locale)
        })
    }
}

fn per_trade(order: &Order) -> u64 {
    u64::from(order.per_trade.unwrap_or(1))
}

fn by_unit_price(left: &Order, right: &Order) -> Ordering {
    (u64::from(left.platinum) * per_trade(right))
        .cmp(&(u64::from(right.platinum) * per_trade(left)))
}

pub fn order_book(orders: Vec<Order>, reach: &Reach) -> OrderBook {
    let (mut sell, mut buy): (Vec<Order>, Vec<Order>) = orders
        .into_iter()
        .filter(|order| reach.covers(order))
        .partition(|order| order.order_type == OrderType::Sell);
    sell.sort_by(by_unit_price);
    buy.sort_by(|left, right| by_unit_price(right, left));
    OrderBook { sell, buy }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Order;
    use crate::parse::envelope;

    const BOOK: &str = include_str!("../tests/fixtures/orders_item_book.json");

    fn names(orders: &[crate::models::Order]) -> Vec<String> {
        orders
            .iter()
            .map(|order| {
                order
                    .user
                    .as_ref()
                    .map(|user| user.ingame_name.clone())
                    .unwrap_or_default()
            })
            .collect()
    }

    fn ingame_english() -> Reach {
        Reach {
            status: TraderStatus::Ingame,
            locale: Some("en".to_owned()),
        }
    }

    #[test]
    fn wider_reach() {
        let listed = |status, locale: Option<&str>| {
            let book = order_book(
                envelope::<Vec<Order>>(BOOK).unwrap(),
                &Reach {
                    status,
                    locale: locale.map(str::to_owned),
                },
            );
            [names(&book.sell), names(&book.buy)].concat()
        };
        let online = listed(TraderStatus::Online, Some("en"));
        assert!(online.iter().any(|name| name == "online_seller"));
        assert!(!online.iter().any(|name| name == "offline_seller"));
        assert!(!online.iter().any(|name| name == "german_seller"));
        assert_eq!(listed(TraderStatus::Any, None).len(), 11);
        let german = listed(TraderStatus::Any, Some("de"));
        assert_eq!(german, ["german_seller"]);
    }

    #[test]
    fn reachable_traders_only() {
        let orders = envelope::<Vec<Order>>(BOOK).unwrap();
        assert_eq!(orders.len(), 11);
        let book = order_book(orders, &ingame_english());
        let listed = [names(&book.sell), names(&book.buy)].concat();
        assert!(!listed.iter().any(|name| name == "offline_seller"));
        assert!(!listed.iter().any(|name| name == "online_seller"));
        assert!(!listed.iter().any(|name| name == "invisible_seller"));
        assert!(!listed.iter().any(|name| name == "german_seller"));
        assert!(!listed.iter().any(|name| name == "chinese_buyer"));
        assert_eq!(listed.len(), 6);
    }

    #[test]
    fn book_sort_order() {
        let orders = envelope::<Vec<Order>>(BOOK).unwrap();
        let book = order_book(orders, &ingame_english());
        assert_eq!(
            names(&book.sell),
            [
                "bulk_seller",
                "cheap_seller",
                "dear_seller",
                "bundle_seller"
            ]
        );
        assert_eq!(names(&book.buy), ["rich_buyer", "thrifty_buyer"]);
    }

    #[test]
    fn empty_book() {
        let book = order_book(Vec::new(), &ingame_english());
        assert!(book.sell.is_empty());
        assert!(book.buy.is_empty());
    }
}
