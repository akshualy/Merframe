use std::collections::HashSet;

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

impl MarketListings {
    pub fn new<S: Into<String>, I: IntoIterator<Item = (S, OrderType)>>(
        orders: I,
        auctions: &[Auction],
    ) -> Self {
        let mut selling = HashSet::new();
        let mut buying = HashSet::new();
        for (item_id, order_type) in orders {
            match order_type {
                OrderType::Sell => selling.insert(item_id.into()),
                OrderType::Buy => buying.insert(item_id.into()),
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

    pub fn listed_slugs<'a>(&self, side: OrderType, items: &'a ItemTable) -> HashSet<&'a str> {
        let item_ids = match side {
            OrderType::Sell => &self.selling,
            OrderType::Buy => &self.buying,
        };
        item_ids
            .iter()
            .filter_map(|item_id| items.by_market_id(item_id)?.market_slug.as_deref())
            .collect()
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

    const AUCTIONS: &str = include_str!("../../wf-market/tests/fixtures/auctions_my.json");

    fn auctions() -> Vec<Auction> {
        wf_market::parse_v1_auctions(AUCTIONS).unwrap()
    }

    #[test]
    fn listed_slugs() {
        let mut items = ItemTable::build(&fixtures::catalog());
        let listed = [
            market_item(
                "trinity_prime_systems",
                "Trinity Prime Systems Blueprint",
                "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint",
                &["component", "blueprint"],
            ),
            market_item(
                "braton_prime_set",
                "Braton Prime Set",
                "/Lotus/Weapons/Tenno/Rifle/BratonPrime",
                &["set"],
            ),
            market_item(
                "braton_prime_barrel",
                "Braton Prime Barrel",
                "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel",
                &["component"],
            ),
        ];
        items.index_market(&listed);
        let listings = MarketListings::new(
            [
                ("trinity_prime_systems", OrderType::Sell),
                ("braton_prime_set", OrderType::Buy),
                ("braton_prime_barrel", OrderType::Sell),
                ("braton_prime_barrel", OrderType::Buy),
                ("unknown", OrderType::Sell),
            ],
            &[],
        );
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
        let items = ItemTable::build(&fixtures::catalog());
        assert!(listings.listed_slugs(OrderType::Sell, &items).is_empty());
        assert!(listings.listed_slugs(OrderType::Buy, &items).is_empty());
        assert!(!listings.lists_riven("Acri-vexicak", "okina", 12, 86));
    }
}
