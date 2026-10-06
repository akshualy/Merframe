use serde::Serialize;
use wf_data::{Rarity, RelicReward};

use crate::catalog::{Catalog, display_name_from_path};
use crate::favourites::Favourites;
use crate::identity::market_slug;
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
    let component = catalog
        .component_for_reward(&reward.item_unique_name)
        .and_then(|(item, component)| Some((item, component, items.part(item, component)?)));
    let favourite = favourite_reward(catalog, favourites, &reward.item_unique_name);
    let stock = &account.stock;
    let owned = stock.count(&reward.item_unique_name);
    let (name, image_name, ducats, ownership) = match component {
        Some((item, component, part)) => (
            part.name.clone(),
            part.image_name.clone(),
            component.ducats,
            RewardOwnership {
                owned,
                needed: component.item_count,
                needed_for_set: stock.count(&component.unique_name)
                    < i64::from(component.item_count),
                parent_owned: account.holds(item),
            },
        ),
        None => match catalog.item(&reward.item_unique_name) {
            Some(item) => (
                item.name.clone(),
                item.image_name.clone(),
                None,
                RewardOwnership {
                    owned,
                    needed: 1,
                    needed_for_set: false,
                    parent_owned: account.holds(item),
                },
            ),
            None => (
                display_name_from_path(&reward.item_unique_name),
                None,
                None,
                RewardOwnership {
                    owned,
                    needed: 1,
                    needed_for_set: false,
                    parent_owned: false,
                },
            ),
        },
    };
    RewardBreakdown {
        forma: name.contains("Forma"),
        unique_name: reward.item_unique_name.clone(),
        component: component.map(|(_, component, _)| component.unique_name.clone()),
        name,
        image_name,
        rarity: rarity_name(reward.rarity),
        plat: prices.plat(&market_slug(&reward.item_name)),
        ducats,
        ownership,
        favourite,
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
