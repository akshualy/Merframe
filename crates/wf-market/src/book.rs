use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::models::{Order, OrderType, UserStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub sell: Vec<Order>,
    pub buy: Vec<Order>,
}

fn reachable(order: &Order) -> bool {
    order
        .user
        .as_ref()
        .is_some_and(|user| user.status == Some(UserStatus::Ingame) && user.locale == "en")
}

fn per_trade(order: &Order) -> u64 {
    u64::from(order.per_trade.unwrap_or(1))
}

fn by_unit_price(left: &Order, right: &Order) -> Ordering {
    (u64::from(left.platinum) * per_trade(right))
        .cmp(&(u64::from(right.platinum) * per_trade(left)))
}

pub fn order_book(orders: Vec<Order>) -> OrderBook {
    let (mut sell, mut buy): (Vec<Order>, Vec<Order>) = orders
        .into_iter()
        .filter(reachable)
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

    #[test]
    fn reachable_traders_only() {
        let orders = envelope::<Vec<Order>>(BOOK).unwrap();
        assert_eq!(orders.len(), 11);
        let book = order_book(orders);
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
        let book = order_book(orders);
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
        let book = order_book(Vec::new());
        assert!(book.sell.is_empty());
        assert!(book.buy.is_empty());
    }
}
