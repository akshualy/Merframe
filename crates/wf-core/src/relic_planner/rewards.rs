use std::collections::HashSet;

use serde::Serialize;
use wf_data::{Rarity, RelicReward};
use wf_inventory::Inventory;

use crate::catalog::{Catalog, Stock, component_image, display_name_from_path, part_name};
use crate::favourites::Favourites;
use crate::prices::market_slug;
use crate::view::View;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RewardOwnership {
    pub owned: i64,
    pub needed: u32,
    pub needed_for_set: bool,
    pub parent_owned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RewardBreakdown {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub rarity: &'static str,
    pub plat: Option<f64>,
    pub ducats: Option<u32>,
    pub ownership: RewardOwnership,
    pub forma: bool,
    pub favourite: bool,
}

pub(super) struct MasteredItems<'a> {
    owned: HashSet<&'a str>,
    capped: HashSet<&'a str>,
}

impl<'a> MasteredItems<'a> {
    pub(super) fn new(inventory: &'a Inventory, catalog: &'a Catalog) -> Self {
        let affinity = inventory.affinity_index();
        let capped = catalog
            .items()
            .filter(|item| {
                affinity
                    .get(item.unique_name.as_str())
                    .copied()
                    .unwrap_or_default()
                    >= item.affinity_cap()
            })
            .map(|item| item.unique_name.as_str())
            .collect();
        Self {
            owned: inventory.owned_item_types(),
            capped,
        }
    }

    pub(super) fn holds(&self, unique_name: &str) -> bool {
        self.owned.contains(unique_name) || self.capped.contains(unique_name)
    }
}

pub(super) fn reward_breakdown(
    view: &View,
    stock: &Stock,
    mastered: &MasteredItems,
    reward: &RelicReward,
) -> RewardBreakdown {
    let View {
        catalog,
        prices,
        favourites,
        ..
    } = *view;
    let component = catalog.component_for_reward(&reward.item_unique_name);
    let favourite = favourite_reward(catalog, favourites, &reward.item_unique_name);
    let owned = stock.count(&reward.item_unique_name);
    let (name, image_name, ducats, ownership) = match component {
        Some((item, component)) => (
            part_name(item, component),
            component_image(item, component),
            component.ducats,
            RewardOwnership {
                owned,
                needed: component.item_count,
                needed_for_set: stock.count(&component.unique_name)
                    < i64::from(component.item_count),
                parent_owned: mastered.holds(&item.unique_name),
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
                    parent_owned: mastered.holds(&item.unique_name),
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
