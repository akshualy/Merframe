use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use wf_market::{PriceEntry, PriceTable};

pub trait PriceSource {
    fn plat(&self, market_slug: &str) -> Option<f64>;

    fn plat_max_rank(&self, _market_slug: &str) -> Option<f64> {
        None
    }

    fn buy_plat(&self, _market_slug: &str) -> Option<f64> {
        None
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Prices {
    pub sell: Option<f64>,
    pub buy: Option<f64>,
    pub ducats: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PriceQuote {
    pub sell: Option<f64>,
    pub sell_max_rank: Option<f64>,
    pub buy: Option<f64>,
}

impl From<&PriceEntry> for PriceQuote {
    fn from(entry: &PriceEntry) -> Self {
        Self {
            sell: entry.sell_r0.map(f64::from),
            sell_max_rank: entry.sell_max.map(f64::from),
            buy: entry.buy_r0.map(f64::from),
        }
    }
}

#[derive(Debug, Default)]
pub struct PriceCache {
    quotes: HashMap<String, PriceQuote>,
    checked_at: Option<DateTime<Utc>>,
}

impl PriceCache {
    pub fn load(&mut self, table: &PriceTable, now: DateTime<Utc>) -> usize {
        self.quotes = table
            .items
            .iter()
            .map(|(slug, entry)| (slug.clone(), PriceQuote::from(entry)))
            .collect();
        self.checked_at = Some(now);
        self.quotes.len()
    }

    pub fn checked(&mut self, now: DateTime<Utc>) {
        self.checked_at = Some(now);
    }

    pub fn checked_at(&self) -> Option<DateTime<Utc>> {
        self.checked_at
    }

    pub fn quote(&self, market_slug: &str) -> Option<PriceQuote> {
        self.quotes.get(market_slug).copied()
    }
}

impl PriceSource for PriceCache {
    fn plat(&self, market_slug: &str) -> Option<f64> {
        self.quote(market_slug)?.sell
    }

    fn plat_max_rank(&self, market_slug: &str) -> Option<f64> {
        self.quote(market_slug)?.sell_max_rank
    }

    fn buy_plat(&self, market_slug: &str) -> Option<f64> {
        self.quote(market_slug)?.buy
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct FixedPrices {
    quotes: HashMap<String, PriceQuote>,
}

#[cfg(test)]
impl FixedPrices {
    pub fn new<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = (S, f64)>,
        S: Into<String>,
    {
        Self {
            quotes: entries
                .into_iter()
                .map(|(slug, plat)| {
                    (
                        slug.into(),
                        PriceQuote {
                            sell: Some(plat),
                            ..PriceQuote::default()
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn with_max_rank<I, S>(mut self, entries: I) -> Self
    where
        I: IntoIterator<Item = (S, f64)>,
        S: Into<String>,
    {
        for (slug, plat) in entries {
            self.quotes.entry(slug.into()).or_default().sell_max_rank = Some(plat);
        }
        self
    }

    pub fn with_buy<I, S>(mut self, entries: I) -> Self
    where
        I: IntoIterator<Item = (S, f64)>,
        S: Into<String>,
    {
        for (slug, plat) in entries {
            self.quotes.entry(slug.into()).or_default().buy = Some(plat);
        }
        self
    }
}

#[cfg(test)]
impl PriceSource for FixedPrices {
    fn plat(&self, market_slug: &str) -> Option<f64> {
        self.quotes.get(market_slug)?.sell
    }

    fn plat_max_rank(&self, market_slug: &str) -> Option<f64> {
        self.quotes.get(market_slug)?.sell_max_rank
    }

    fn buy_plat(&self, market_slug: &str) -> Option<f64> {
        self.quotes.get(market_slug)?.buy
    }
}

pub(crate) fn market_slug(display_name: &str) -> String {
    let mut slug = String::with_capacity(display_name.len());
    let mut pending_separator = false;
    for ch in display_name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('_');
            }
            pending_separator = false;
            slug.push(ch.to_ascii_lowercase());
        } else if ch != '\'' {
            pending_separator = true;
        }
    }
    slug
}

pub fn set_slug(set_name: &str) -> String {
    format!("{}_set", market_slug(set_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = include_str!("../../wf-market/tests/fixtures/prices_bulk.json");

    fn table() -> PriceTable {
        wf_market::parse_price_table(TABLE).unwrap()
    }

    fn moment() -> DateTime<Utc> {
        DateTime::from_timestamp(1_757_410_800, 0).unwrap()
    }

    #[test]
    fn market_slugs() {
        assert_eq!(market_slug("Braton Prime Barrel"), "braton_prime_barrel");
        assert_eq!(
            market_slug("Trinity Prime Systems"),
            "trinity_prime_systems"
        );
        assert_eq!(market_slug("Axi A1 Relic"), "axi_a1_relic");
        assert_eq!(market_slug("Gaia's Tragedy"), "gaias_tragedy");
        assert_eq!(set_slug("Braton Prime"), "braton_prime_set");
        assert_eq!(
            market_slug("Zephyr Prime  Blueprint"),
            "zephyr_prime_blueprint"
        );
    }

    #[test]
    fn fixed_prices() {
        let prices = FixedPrices::new([("braton_prime_barrel", 12.0)])
            .with_max_rank([("primed_continuity", 165.0)])
            .with_buy([("braton_prime_barrel", 4.0)]);
        assert_eq!(prices.plat("braton_prime_barrel"), Some(12.0));
        assert_eq!(prices.buy_plat("braton_prime_barrel"), Some(4.0));
        assert_eq!(prices.plat_max_rank("braton_prime_barrel"), None);
        assert_eq!(prices.plat_max_rank("primed_continuity"), Some(165.0));
        assert_eq!(prices.plat("braton_prime_stock"), None);
        assert_eq!(prices.buy_plat("braton_prime_stock"), None);
    }

    #[test]
    fn loaded_table() {
        let mut cache = PriceCache::default();
        assert_eq!(cache.checked_at(), None);
        assert_eq!(cache.plat("braton_prime_set"), None);

        let loaded = cache.load(&table(), moment());
        assert_eq!(loaded, 10);
        assert_eq!(cache.checked_at(), Some(moment()));
        assert_eq!(cache.plat("braton_prime_set"), Some(45.0));
        assert_eq!(cache.buy_plat("braton_prime_set"), Some(30.0));
        assert_eq!(cache.plat_max_rank("braton_prime_set"), None);
        assert_eq!(cache.plat("nothing_is_traded_here"), None);
    }

    #[test]
    fn ranked_entry() {
        let mut cache = PriceCache::default();
        cache.load(&table(), moment());
        assert_eq!(cache.plat("arcane_energize"), Some(55.0));
        assert_eq!(cache.plat_max_rank("arcane_energize"), Some(940.0));
        assert_eq!(cache.buy_plat("arcane_energize"), Some(40.0));
        assert_eq!(
            cache.quote("magus_elevate"),
            Some(PriceQuote {
                sell: Some(25.0),
                sell_max_rank: Some(70.0),
                buy: Some(15.0),
            })
        );
    }

    #[test]
    fn item_without_orders() {
        let mut cache = PriceCache::default();
        cache.load(&table(), moment());
        assert_eq!(
            cache.quote("vasca_kavat_imprint"),
            Some(PriceQuote::default())
        );
        assert_eq!(cache.plat("vasca_kavat_imprint"), None);
        assert_eq!(cache.buy_plat("vasca_kavat_imprint"), None);
    }

    #[test]
    fn replacing_table() {
        let mut cache = PriceCache::default();
        cache.load(&table(), moment());
        let later = moment() + chrono::TimeDelta::minutes(15);
        let smaller = wf_market::parse_price_table(
            r#"{"updated_at":1757411700,"count":1,"items":{"arcane_energize":{"sell_r0":60,"sell_max":900,"buy_r0":45}}}"#,
        )
        .unwrap();
        assert_eq!(cache.load(&smaller, later), 1);
        assert_eq!(cache.plat("arcane_energize"), Some(60.0));
        assert_eq!(cache.plat("braton_prime_set"), None);
        assert_eq!(cache.checked_at(), Some(later));
    }

    #[test]
    fn checked_without_reload() {
        let mut cache = PriceCache::default();
        cache.load(&table(), moment());
        let later = moment() + chrono::TimeDelta::minutes(15);
        cache.checked(later);
        assert_eq!(cache.checked_at(), Some(later));
        assert_eq!(cache.plat("braton_prime_set"), Some(45.0));
    }
}
