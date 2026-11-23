use std::collections::HashMap;

use super::upgrades::ranked_to_max;
use super::{ItemIndex, MiscRow, ModRow, PartRow, RelicRow, SetRow};
use crate::prices::PriceSource;

pub(crate) trait InventoryHolding {
    fn item(&self) -> usize;
    fn count(&self) -> i64;
    fn listing(&self) -> Option<u32>;
    fn set_liquidation_value(&mut self, value: Option<f64>);

    fn at_max_rank(&self) -> bool {
        false
    }
}

pub(crate) fn set_liquidation_values<H: InventoryHolding>(
    holdings: &mut [H],
    index: &ItemIndex,
    prices: &dyn PriceSource,
) {
    set_liquidation_values_of(holdings, index, prices, H::count, H::set_liquidation_value);
}

pub(crate) fn set_liquidation_values_of<H: InventoryHolding>(
    holdings: &mut [H],
    index: &ItemIndex,
    prices: &dyn PriceSource,
    copies: impl Fn(&H) -> i64,
    set_value: impl Fn(&mut H, Option<f64>),
) {
    let mut pooled_copies: HashMap<(&str, bool), i64> = HashMap::new();
    for holding in holdings.iter() {
        if let Some(slug) = index[holding.item()].market_slug.as_deref() {
            *pooled_copies
                .entry((slug, holding.at_max_rank()))
                .or_default() += copies(holding);
        }
    }

    for holding in holdings {
        let count = copies(holding);
        let value = index[holding.item()]
            .market_slug
            .as_deref()
            .and_then(|slug| row_liquidation_value(&*holding, count, slug, &pooled_copies, prices));
        set_value(holding, value);
    }
}

fn row_liquidation_value(
    holding: &impl InventoryHolding,
    count: i64,
    slug: &str,
    pooled_copies: &HashMap<(&str, bool), i64>,
    prices: &dyn PriceSource,
) -> Option<f64> {
    let unranked_pool = pooled_copies.get(&(slug, false)).copied().unwrap_or(0);

    if !holding.at_max_rank() {
        return prices
            .liquidity(slug)
            .liquidation_value(holding.listing(), count, unranked_pool);
    }

    let maxed_pool = pooled_copies[&(slug, true)];
    let at_max_rank =
        prices
            .liquidity_max_rank(slug)
            .liquidation_value(holding.listing(), count, maxed_pool);
    let as_unranked =
        prices
            .liquidity(slug)
            .liquidation_value(None, count, unranked_pool + maxed_pool);
    at_max_rank.into_iter().chain(as_unranked).reduce(f64::max)
}

macro_rules! row_fields {
    () => {
        fn item(&self) -> usize {
            self.item
        }

        fn count(&self) -> i64 {
            self.count
        }

        fn set_liquidation_value(&mut self, value: Option<f64>) {
            self.liquidation_value = value;
        }
    };
}

impl InventoryHolding for PartRow {
    row_fields!();

    fn listing(&self) -> Option<u32> {
        self.prices.sell
    }
}

impl InventoryHolding for SetRow {
    row_fields!();

    fn listing(&self) -> Option<u32> {
        self.prices.sell
    }
}

impl InventoryHolding for RelicRow {
    row_fields!();

    fn listing(&self) -> Option<u32> {
        self.plat
    }
}

impl InventoryHolding for MiscRow {
    row_fields!();

    fn listing(&self) -> Option<u32> {
        self.plat
    }
}

impl InventoryHolding for ModRow {
    row_fields!();

    fn listing(&self) -> Option<u32> {
        if self.at_max_rank() {
            self.prices.sell_max_rank.or(self.prices.sell)
        } else {
            self.prices.sell
        }
    }

    fn at_max_rank(&self) -> bool {
        ranked_to_max(self.rank, self.max_rank)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ItemSummary, UpgradePrices, misc};
    use super::*;
    use crate::catalog::fixtures;
    use crate::prices::{FixedPrices, MarketLiquidity};
    use crate::view::Fixture;

    #[test]
    fn pooled_volume() {
        let fixture = Fixture::new(fixtures::misc_catalog(), fixtures::inventory());
        let (mut rows, index) = fixture.rows(misc);
        let sah = |row: &MiscRow| {
            index[row.item].unique_name == "/Lotus/Types/Items/FusionTreasures/OroFusexA"
        };
        let slug = rows
            .iter()
            .find(|row| sah(row))
            .and_then(|row| index[row.item].market_slug.clone())
            .unwrap();
        let prices = FixedPrices::default().with_liquidity([(
            slug,
            MarketLiquidity {
                mark: Some(10.0),
                bid: Some(2.0),
                volume_30d: Some(36),
            },
        )]);

        set_liquidation_values(&mut rows, &index, &prices);
        let values: Vec<_> = rows
            .iter()
            .filter(|row| sah(row))
            .map(|row| (row.count, row.liquidation_value))
            .collect();
        assert_eq!(values, vec![(9, Some(36.0)), (27, Some(108.0))]);
    }

    fn upgrade(item: usize, rank: Option<u32>, max_rank: Option<u32>, count: i64) -> ModRow {
        ModRow {
            item,
            count,
            rank,
            max_rank,
            prices: UpgradePrices {
                sell: Some(12),
                sell_max_rank: Some(240),
                is_floor: false,
                buy: None,
            },
            liquidation_value: None,
            rarity: None,
            equipped_in: Vec::new(),
        }
    }

    fn summary(slug: &str) -> ItemSummary {
        ItemSummary {
            unique_name: slug.to_owned(),
            name: slug.to_owned(),
            image_name: None,
            market_slug: Some(slug.to_owned()),
            prime: false,
            vault: None,
            ducats: None,
        }
    }

    #[test]
    fn upgrade_value() {
        let mut index = ItemIndex::default();
        let energize = index.add(summary("arcane_energize"));
        let unmarked = index.add(summary("arcane_unmarked"));
        let veiled = index.add(summary("veiled_riven"));
        let munitions = index.add(summary("hunter_munitions"));
        let mut rows = vec![
            upgrade(energize, Some(0), Some(5), 4),
            upgrade(energize, Some(5), Some(5), 4),
            upgrade(unmarked, Some(5), Some(5), 1),
            upgrade(veiled, Some(0), Some(0), 2),
            upgrade(munitions, Some(5), Some(5), 1),
        ];
        let prices = FixedPrices::default()
            .with_liquidity([
                (
                    "arcane_energize",
                    MarketLiquidity {
                        mark: Some(10.0),
                        bid: Some(2.0),
                        volume_30d: Some(8),
                    },
                ),
                (
                    "veiled_riven",
                    MarketLiquidity {
                        mark: Some(5.0),
                        ..MarketLiquidity::default()
                    },
                ),
                (
                    "hunter_munitions",
                    MarketLiquidity {
                        mark: Some(5.0),
                        ..MarketLiquidity::default()
                    },
                ),
            ])
            .with_liquidity_max_rank([
                (
                    "arcane_energize",
                    MarketLiquidity {
                        mark: Some(90.0),
                        bid: Some(30.0),
                        volume_30d: Some(8),
                    },
                ),
                (
                    "hunter_munitions",
                    MarketLiquidity {
                        mark: Some(9.0),
                        bid: None,
                        volume_30d: Some(0),
                    },
                ),
            ]);

        set_liquidation_values(&mut rows, &index, &prices);
        let values: Vec<_> = rows.iter().map(|row| row.liquidation_value).collect();
        assert_eq!(
            values,
            vec![Some(24.0), Some(240.0), Some(240.0), Some(10.0), Some(5.0)]
        );
    }
}
