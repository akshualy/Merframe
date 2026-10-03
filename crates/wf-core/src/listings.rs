use std::collections::HashSet;

use serde::Serialize;
use wf_market::{Auction, OrderType};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ListedRiven {
    name: String,
    weapon: String,
    mastery: u32,
    rerolls: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct PlacedOrders {
    pub sell: bool,
    pub buy: bool,
}

#[derive(Debug, Default)]
pub struct MarketListings {
    selling: HashSet<String>,
    buying: HashSet<String>,
    rivens: HashSet<ListedRiven>,
}

fn squashed(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn ordered_item(market_slug: &str) -> String {
    market_slug.to_lowercase().replace("_blueprint", "")
}

impl MarketListings {
    pub fn new<S: AsRef<str>, I: IntoIterator<Item = (S, OrderType)>>(
        orders: I,
        auctions: &[Auction],
    ) -> Self {
        let mut selling = HashSet::new();
        let mut buying = HashSet::new();
        for (slug, order_type) in orders {
            let item = ordered_item(slug.as_ref());
            match order_type {
                OrderType::Sell => selling.insert(item),
                OrderType::Buy => buying.insert(item),
            };
        }
        Self {
            selling,
            buying,
            rivens: auctions
                .iter()
                .map(|auction| ListedRiven {
                    name: squashed(&auction.item.name),
                    weapon: squashed(&auction.item.weapon_url_name),
                    mastery: auction.item.mastery_level,
                    rerolls: auction.item.re_rolls,
                })
                .collect(),
        }
    }

    pub fn orders_for(&self, market_slug: &str) -> PlacedOrders {
        if market_slug.is_empty() {
            return PlacedOrders::default();
        }
        let item = ordered_item(market_slug);
        PlacedOrders {
            sell: self.selling.contains(&item),
            buy: self.buying.contains(&item),
        }
    }

    pub fn lists_riven(&self, name: &str, weapon_slug: &str, mastery: u32, rerolls: u32) -> bool {
        self.rivens.contains(&ListedRiven {
            name: squashed(name),
            weapon: squashed(weapon_slug),
            mastery,
            rerolls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUCTIONS: &str = include_str!("../../wf-market/tests/fixtures/auctions_my.json");

    fn auctions() -> Vec<Auction> {
        wf_market::parse_v1_auctions(AUCTIONS).unwrap()
    }

    #[test]
    fn orders_for_blueprint_suffix_and_side() {
        let listings = MarketListings::new(
            [
                ("ash_prime_systems_blueprint", OrderType::Sell),
                ("braton_prime_set", OrderType::Buy),
                ("primed_continuity", OrderType::Sell),
                ("primed_continuity", OrderType::Buy),
            ],
            &[],
        );
        let sell = PlacedOrders {
            sell: true,
            buy: false,
        };
        let buy = PlacedOrders {
            sell: false,
            buy: true,
        };
        let both = PlacedOrders {
            sell: true,
            buy: true,
        };
        assert_eq!(listings.orders_for("ash_prime_systems"), sell);
        assert_eq!(listings.orders_for("ash_prime_systems_blueprint"), sell);
        assert_eq!(listings.orders_for("braton_prime_set"), buy);
        assert_eq!(listings.orders_for("primed_continuity"), both);
        assert_eq!(
            listings.orders_for("braton_prime_barrel"),
            PlacedOrders::default()
        );
        assert_eq!(listings.orders_for(""), PlacedOrders::default());
    }

    #[test]
    fn lists_riven() {
        let listings = MarketListings::new(Vec::<(String, OrderType)>::new(), &auctions());
        assert!(listings.lists_riven("Acri-vexicak", "okina", 12, 86));
        assert!(listings.lists_riven("Acri-Vexicak ", "Okina", 12, 86));
        assert!(
            !listings.lists_riven("Acri-vexicak", "okina", 12, 87),
            "another reroll count is another riven"
        );
        assert!(
            !listings.lists_riven("Acri-vexicak", "okina", 16, 86),
            "another mastery requirement is another riven"
        );
        assert!(
            !listings.lists_riven("Acri-vexicak", "boltor", 12, 86),
            "another weapon is another riven"
        );
        assert!(listings.lists_riven("Crita-critacan", "kuva_bramma", 16, 12));
    }

    #[test]
    fn empty_listings() {
        let listings = MarketListings::default();
        assert_eq!(
            listings.orders_for("braton_prime_set"),
            PlacedOrders::default()
        );
        assert!(!listings.lists_riven("Acri-vexicak", "okina", 12, 86));
    }
}
