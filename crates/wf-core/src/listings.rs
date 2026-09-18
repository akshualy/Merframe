use std::collections::HashSet;

use wf_market::Auction;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ListedRiven {
    name: String,
    weapon: String,
    mastery: u32,
    rerolls: u32,
}

#[derive(Debug, Default)]
pub struct MarketListings {
    orders: HashSet<String>,
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
    pub fn new<I: IntoIterator<Item: AsRef<str>>>(order_slugs: I, auctions: &[Auction]) -> Self {
        Self {
            orders: order_slugs
                .into_iter()
                .map(|slug| ordered_item(slug.as_ref()))
                .collect(),
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

    pub fn has_order(&self, market_slug: &str) -> bool {
        !market_slug.is_empty() && self.orders.contains(&ordered_item(market_slug))
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
    fn has_order_blueprint_suffix() {
        let listings = MarketListings::new(
            [
                "ash_prime_systems_blueprint",
                "braton_prime_set",
                "primed_continuity",
            ],
            &[],
        );
        assert!(listings.has_order("ash_prime_systems"));
        assert!(listings.has_order("ash_prime_systems_blueprint"));
        assert!(listings.has_order("braton_prime_set"));
        assert!(listings.has_order("primed_continuity"));
        assert!(!listings.has_order("braton_prime_barrel"));
        assert!(!listings.has_order(""));
    }

    #[test]
    fn lists_riven() {
        let listings = MarketListings::new(Vec::<String>::new(), &auctions());
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
        assert!(!listings.has_order("braton_prime_set"));
        assert!(!listings.lists_riven("Acri-vexicak", "okina", 12, 86));
    }
}
