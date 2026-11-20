use std::collections::{HashMap, HashSet};

use wf_market::{Auction, OrderType};

use crate::identity::ItemTable;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ListedRiven {
    name: String,
    weapon: String,
    mastery: u32,
    rerolls: u32,
}

#[derive(Debug, Default)]
struct MarketListedQuantities {
    selling: HashMap<String, u32>,
    buying: HashMap<String, u32>,
}

#[derive(Debug, Default)]
pub struct MarketListings {
    orders: Option<MarketListedQuantities>,
    rivens: HashSet<ListedRiven>,
}

fn squashed(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

impl MarketListings {
    pub fn new(auctions: &[Auction]) -> Self {
        Self {
            orders: None,
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

    #[must_use]
    pub fn with_orders<S: Into<String>>(
        self,
        orders: impl IntoIterator<Item = (S, OrderType, u32)>,
    ) -> Self {
        let mut listed = MarketListedQuantities::default();
        for (item_id, order_type, quantity) in orders {
            let side = match order_type {
                OrderType::Sell => &mut listed.selling,
                OrderType::Buy => &mut listed.buying,
            };
            *side.entry(item_id.into()).or_default() += quantity;
        }
        Self {
            orders: Some(listed),
            ..self
        }
    }

    pub fn listed_slugs<'a>(&self, side: OrderType, items: &'a ItemTable) -> HashSet<&'a str> {
        let Some(orders) = &self.orders else {
            return HashSet::new();
        };

        let quantities = match side {
            OrderType::Sell => &orders.selling,
            OrderType::Buy => &orders.buying,
        };
        quantities
            .keys()
            .filter_map(|item_id| items.by_market_id(item_id)?.market_slug.as_deref())
            .collect()
    }

    pub(crate) fn selling(&self) -> Option<&HashMap<String, u32>> {
        self.orders.as_ref().map(|orders| &orders.selling)
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
    use crate::catalog::fixtures::{self, market_item};

    fn auctions() -> Vec<Auction> {
        wf_market::parse_v1_auctions(fixtures::MY_AUCTIONS).unwrap()
    }

    #[test]
    fn listed_slugs() {
        let mut items = ItemTable::build(&fixtures::catalog());
        let listed = [
            market_item(
                "trinity_prime_systems",
                "Trinity Prime Systems Blueprint",
                fixtures::TRINITY_PRIME_SYSTEMS_BLUEPRINT,
                &["component", "blueprint"],
            ),
            market_item(
                "braton_prime_set",
                "Braton Prime Set",
                fixtures::BRATON_PRIME,
                &["set"],
            ),
            market_item(
                "braton_prime_barrel",
                "Braton Prime Barrel",
                fixtures::BRATON_PRIME_BARREL,
                &["component"],
            ),
        ];
        items.index_market(&listed);
        let listings = MarketListings::new(&[]).with_orders([
            ("trinity_prime_systems", OrderType::Sell, 1),
            ("braton_prime_set", OrderType::Buy, 1),
            ("braton_prime_barrel", OrderType::Sell, 1),
            ("braton_prime_barrel", OrderType::Buy, 1),
            ("unknown", OrderType::Sell, 1),
        ]);
        let sorted = |side| {
            let mut slugs: Vec<&str> = listings.listed_slugs(side, &items).into_iter().collect();
            slugs.sort_unstable();
            slugs
        };
        assert_eq!(
            sorted(OrderType::Sell),
            ["braton_prime_barrel", "trinity_prime_systems_blueprint"]
        );
        assert_eq!(
            sorted(OrderType::Buy),
            ["braton_prime_barrel", "braton_prime_set"]
        );
    }

    #[test]
    fn lists_riven() {
        let listings = MarketListings::new(&auctions());
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
        let items = ItemTable::build(&fixtures::catalog());
        assert!(listings.listed_slugs(OrderType::Sell, &items).is_empty());
        assert!(listings.listed_slugs(OrderType::Buy, &items).is_empty());
        assert!(!listings.lists_riven("Acri-vexicak", "okina", 12, 86));
    }
}
