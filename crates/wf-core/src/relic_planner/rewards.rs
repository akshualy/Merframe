use serde::Serialize;
use wf_data::{Rarity, RelicReward};

use crate::catalog::Catalog;
use crate::favourites::Favourites;
use crate::view::View;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RewardOwnership {
    pub owned: i64,
    pub needed: u32,
    pub needed_for_set: bool,
    pub parent_owned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RewardBreakdown {
    pub unique_name: String,
    pub component: Option<String>,
    pub name: String,
    pub image_name: Option<String>,
    pub rarity: &'static str,
    pub plat: Option<f64>,
    pub ducats: Option<u32>,
    pub ownership: RewardOwnership,
    pub forma: bool,
    pub favourite: bool,
}

pub(super) fn reward_breakdown(view: &View, reward: &RelicReward) -> RewardBreakdown {
    let View {
        account,
        catalog,
        items,
        prices,
        favourites,
        ..
    } = *view;
    let unique_name = &reward.item_unique_name;
    let record = items.resolve(catalog, unique_name);
    let component = catalog.component_for_reward(unique_name);
    let stock = &account.stock;
    let owned = stock.count(unique_name);

    let ownership = match component {
        Some((item, component)) => RewardOwnership {
            owned,
            needed: component.item_count,
            needed_for_set: stock.count(&component.unique_name) < i64::from(component.item_count),
            parent_owned: account.holds(item),
        },
        None => RewardOwnership {
            owned,
            needed: 1,
            needed_for_set: false,
            parent_owned: catalog
                .item(unique_name)
                .is_some_and(|item| account.holds(item)),
        },
    };

    RewardBreakdown {
        forma: record.name.contains("Forma"),
        unique_name: unique_name.clone(),
        component: component.map(|(_, component)| component.unique_name.clone()),
        name: record.name.clone(),
        image_name: record.image_name.clone(),
        rarity: rarity_name(reward.rarity),
        plat: reward
            .market_slug
            .as_deref()
            .or(record.market_slug.as_deref())
            .and_then(|slug| prices.plat(slug)),
        ducats: component.and_then(|(_, component)| component.ducats),
        ownership,
        favourite: favourite_reward(catalog, favourites, unique_name),
    }
}

pub(super) fn favourite_reward(
    catalog: &Catalog,
    favourites: &Favourites,
    unique_name: &str,
) -> bool {
    favourites.contains(unique_name)
        || catalog
            .component_for_reward(unique_name)
            .is_some_and(|(item, component)| {
                favourites.any([component.unique_name.as_str(), item.unique_name.as_str()])
            })
}

pub(super) fn rarity_name(rarity: Rarity) -> &'static str {
    match rarity {
        Rarity::Common => "Common",
        Rarity::Uncommon => "Uncommon",
        Rarity::Rare => "Rare",
        Rarity::Legendary => "Legendary",
    }
}
