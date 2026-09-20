use std::collections::HashSet;

use serde::Serialize;
use wf_data::{Component, Item, store_item_to_type};
use wf_inventory::Inventory;

use crate::catalog::{
    Catalog, DUCATS_ITEM, Stock, component_image, item_name, names_a_prime, part_identity,
    part_market_slug, part_name,
};
use crate::favourites::Favourites;
use crate::prices::{PriceSource, market_slug, set_slug};

use super::missing_parts;
use super::rewards::{MasteredItems, RewardOwnership, favourite_reward};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ranked {
    pub store_item: String,
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub plat: Option<f64>,
    pub ducats: Option<u32>,
    pub set_plat: Option<f64>,
    pub ownership: Option<RewardOwnership>,
    pub vaulted: bool,
    pub favourite: bool,
    pub best: bool,
    pub components: Vec<RankedComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RankedComponent {
    pub unique_name: String,
    pub name: String,
    pub image_name: Option<String>,
    pub owned: Option<i64>,
    pub needed: u32,
    pub enough: bool,
    pub this_reward: bool,
    pub favourite: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RewardScreen {
    pub ranked: Vec<Ranked>,
    pub account: Option<AccountBalance>,
}

impl RewardScreen {
    #[must_use]
    pub fn chat_line(&self) -> String {
        self.ranked
            .iter()
            .filter_map(|reward| {
                let name = reward.name.trim_end_matches(" Blueprint");
                Some(format!("[{name}] {}:platinum:", reward.plat?.round()))
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AccountBalance {
    pub plat: i64,
    pub ducats: i64,
}

struct Holdings<'a> {
    missing: HashSet<String>,
    stock: Stock<'a>,
    mastered: MasteredItems<'a>,
    building: HashSet<&'a str>,
}

impl<'a> Holdings<'a> {
    fn new(inventory: &'a Inventory, catalog: &'a Catalog) -> Self {
        Self {
            missing: missing_parts(inventory, catalog)
                .iter()
                .map(|part| part_identity(&part.unique_name).to_owned())
                .collect(),
            stock: Stock::new(inventory),
            mastered: MasteredItems::new(inventory, catalog),
            building: inventory
                .pending_recipes
                .iter()
                .map(|recipe| recipe.item_type.as_str())
                .collect(),
        }
    }

    fn ownership(&self, unique_name: &str, parent: Option<(&Item, &Component)>) -> RewardOwnership {
        RewardOwnership {
            owned: self.stock.count(unique_name),
            needed: parent.map_or(1, |(_, component)| component.item_count),
            needed_for_set: self.missing.contains(part_identity(unique_name)),
            parent_owned: parent.is_some_and(|(item, _)| self.built_or_building(item)),
        }
    }

    fn built_or_building(&self, item: &Item) -> bool {
        self.mastered.holds(&item.unique_name)
            || item
                .components
                .as_deref()
                .unwrap_or_default()
                .iter()
                .any(|component| self.building.contains(component.unique_name.as_str()))
    }
}

fn reward_identity(
    catalog: &Catalog,
    unique_name: &str,
    parent: Option<(&Item, &Component)>,
) -> (String, String, Option<u32>, Option<String>) {
    if let Some((item, component)) = parent {
        (
            part_name(item, component),
            part_market_slug(item, component),
            component.ducats,
            component_image(item, component),
        )
    } else {
        let name = item_name(catalog, unique_name);
        let slug = market_slug(&name);
        let image_name = catalog
            .item(unique_name)
            .and_then(|item| item.image_name.clone());
        (name, slug, None, image_name)
    }
}

fn mark_best(ranked: &mut [Ranked]) {
    let highest = ranked
        .iter()
        .filter_map(|entry| entry.plat)
        .max_by(f64::total_cmp);
    for entry in ranked {
        entry.best = match (highest, entry.plat) {
            (Some(highest), Some(plat)) => plat >= highest,
            (Some(_), None) => false,
            (None, _) => true,
        };
    }
}

pub(crate) fn recommend(
    inventory: Option<&Inventory>,
    catalog: &Catalog,
    prices: &dyn PriceSource,
    favourites: &Favourites,
    rewards: &[String],
) -> RewardScreen {
    let holdings = inventory.map(|inventory| Holdings::new(inventory, catalog));
    let mut ranked: Vec<Ranked> = rewards
        .iter()
        .map(|store_item| {
            let unique_name = store_item_to_type(store_item);
            let parent = catalog.component_for_reward(&unique_name);
            let (name, slug, ducats, image_name) = reward_identity(catalog, &unique_name, parent);
            Ranked {
                store_item: store_item.clone(),
                image_name,
                plat: prices.plat(&slug),
                ducats,
                set_plat: parent.and_then(|(item, _)| prices.plat(&set_slug(&item.name))),
                ownership: holdings
                    .as_ref()
                    .map(|holdings| holdings.ownership(&unique_name, parent)),
                vaulted: parent.is_some_and(|(item, _)| item.vaulted.unwrap_or_default())
                    && !name.contains("Forma Blueprint"),
                favourite: favourite_reward(catalog, favourites, &unique_name),
                best: false,
                components: match parent {
                    Some((item, component)) => set_components(
                        item,
                        &component.unique_name,
                        &name,
                        holdings.as_ref().map(|holdings| &holdings.stock),
                        favourites,
                    ),
                    None => Vec::new(),
                },
                unique_name,
                name,
            }
        })
        .collect();
    mark_best(&mut ranked);
    RewardScreen {
        ranked,
        account: inventory.map(|inventory| AccountBalance {
            plat: inventory.premium_credits,
            ducats: inventory.counted(DUCATS_ITEM),
        }),
    }
}

fn is_prime_part(unique_name: &str) -> bool {
    !unique_name.contains("/MiscItems/") && !unique_name.contains("/Research/")
}

fn set_components(
    item: &Item,
    reward_component: &str,
    reward_name: &str,
    stock: Option<&Stock>,
    favourites: &Favourites,
) -> Vec<RankedComponent> {
    if !names_a_prime(reward_name) && !names_a_prime(&item.name) {
        return Vec::new();
    }
    item.components
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|component| is_prime_part(&component.unique_name))
        .map(|component| {
            let owned = stock.map(|stock| stock.count(&component.unique_name));
            RankedComponent {
                unique_name: component.unique_name.clone(),
                name: part_name(item, component),
                image_name: component_image(item, component),
                owned,
                needed: component.item_count,
                enough: owned.is_some_and(|owned| owned >= i64::from(component.item_count)),
                this_reward: component.unique_name == reward_component,
                favourite: favourites
                    .any([component.unique_name.as_str(), item.unique_name.as_str()]),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::tests::prices;
    use super::*;
    use crate::catalog::fixtures;
    use crate::prices::FixedPrices;
    use wf_inventory::Inventory;

    #[test]
    fn kavasa_prime_band() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::with_skins(fixtures::ITEMS);
        let rewards = vec![
            "/Lotus/StoreItems/Types/Recipes/Kubrow/Collars/PrimeKubrowCollarABandComponent"
                .to_owned(),
        ];
        let screen = recommend(
            Some(&inventory),
            &catalog,
            &prices(),
            &Favourites::default(),
            &rewards,
        );
        let band = &screen.ranked[0];
        assert_eq!(band.name, "Kavasa Prime Band");
        assert_eq!(band.ducats, Some(45));
        assert_eq!(
            band.image_name.as_deref(),
            Some("GenericComponentPrimeLatch.png")
        );
    }

    #[test]
    fn chat_line_priced_rewards() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let rewards = vec![
            "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/StyanaxPrimeBlueprint".to_owned(),
            "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint".to_owned(),
            "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint"
                .to_owned(),
        ];
        let screen = recommend(
            Some(&inventory),
            &catalog,
            &prices(),
            &Favourites::default(),
            &rewards,
        );
        assert_eq!(
            screen.chat_line(),
            "[Styanax Prime] 100:platinum: | [Trinity Prime Systems] 12:platinum:"
        );
    }

    #[test]
    fn screen_order_and_best() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let rewards = vec![
            "/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/AlternoxPrimeStock".to_owned(),
            "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/StyanaxPrimeBlueprint".to_owned(),
            "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint".to_owned(),
            "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint"
                .to_owned(),
        ];
        let ranked = recommend(
            Some(&inventory),
            &catalog,
            &prices(),
            &Favourites::default(),
            &rewards,
        )
        .ranked;
        assert_eq!(ranked.len(), 4);
        assert_eq!(ranked[0].name, "Alternox Prime Stock");
        assert_eq!(ranked[1].name, "Styanax Prime Blueprint");
        assert_eq!(ranked[2].name, "Forma Blueprint");
        assert_eq!(ranked[3].name, "Trinity Prime Systems");

        assert!(!ranked[0].best);
        assert!(ranked[1].best);
        assert!(!ranked[2].best);
        assert!(!ranked[3].best);

        assert_eq!(ranked[1].plat, Some(100.0));
        assert_eq!(ranked[3].plat, Some(12.0));
        assert_eq!(ranked[3].ducats, Some(15));
        assert!(ranked[3].ownership.unwrap().needed_for_set);
        assert_eq!(ranked[3].ownership.unwrap().owned, 0);
        assert_eq!(ranked[2].plat, None);
        assert!(ranked[2].ownership.unwrap().owned > 0);
        assert!(ranked.iter().all(|entry| !entry.favourite));
    }

    #[test]
    fn starred_part_or_parent() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let rewards = vec![
            "/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock".to_owned(),
            "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint"
                .to_owned(),
            "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint".to_owned(),
        ];
        let starred: Favourites = [
            "/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock",
            "/Lotus/Powersuits/Trinity/TrinityPrime",
        ]
        .into_iter()
        .collect();
        let ranked = recommend(Some(&inventory), &catalog, &prices(), &starred, &rewards).ranked;
        assert!(ranked[0].favourite, "the part itself is starred");
        assert!(
            ranked[1].favourite,
            "the Warframe the part builds is starred"
        );
        assert!(!ranked[2].favourite);
    }
    const TRINITY_SYSTEMS_REWARD: &str =
        "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint";
    const TRINITY_SYSTEMS: &str =
        "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsComponent";
    const FORMA_REWARD: &str = "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint";

    fn screen(inventory: &Inventory, rewards: &[&str]) -> RewardScreen {
        let rewards: Vec<String> = rewards.iter().map(|reward| (*reward).to_owned()).collect();
        recommend(
            Some(inventory),
            &fixtures::catalog(),
            &prices(),
            &Favourites::default(),
            &rewards,
        )
    }

    fn inventory_without_trinity_prime(building: Option<&str>) -> Inventory {
        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        for key in ["Suits", "XPInfo"] {
            value
                .get_mut(key)
                .and_then(serde_json::Value::as_array_mut)
                .unwrap()
                .retain(|entry| {
                    entry.get("ItemType").and_then(serde_json::Value::as_str)
                        != Some("/Lotus/Powersuits/Trinity/TrinityPrime")
                });
        }
        if let Some(item_type) = building {
            value
                .get_mut("PendingRecipes")
                .and_then(serde_json::Value::as_array_mut)
                .unwrap()
                .push(serde_json::json!({
                    "ItemType": item_type,
                    "CompletionDate": { "$date": { "$numberLong": "1700000000000" } },
                    "ItemId": { "$oid": "000000000000000000000000" }
                }));
        }
        Inventory::parse(&value.to_string()).unwrap()
    }

    #[test]
    fn parent_owned() {
        let owned = screen(&fixtures::inventory(), &[TRINITY_SYSTEMS_REWARD]);
        assert!(owned.ranked[0].ownership.unwrap().parent_owned);

        let missing = inventory_without_trinity_prime(None);
        assert!(
            !screen(&missing, &[TRINITY_SYSTEMS_REWARD]).ranked[0]
                .ownership
                .unwrap()
                .parent_owned
        );

        let building = inventory_without_trinity_prime(Some(
            "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeChassisComponent",
        ));
        assert!(
            screen(&building, &[TRINITY_SYSTEMS_REWARD]).ranked[0]
                .ownership
                .unwrap()
                .parent_owned,
            "a component of the set is in the foundry"
        );
    }

    #[test]
    fn owned_against_needed() {
        let held = fixtures::inventory_owning(&[(TRINITY_SYSTEMS, 2)]);
        let ranked = screen(&held, &[TRINITY_SYSTEMS_REWARD, FORMA_REWARD]).ranked;
        assert_eq!(ranked[0].ownership.unwrap().owned, 2);
        assert_eq!(ranked[0].ownership.unwrap().needed, 1);
        assert!(ranked[0].ownership.unwrap().owned > 0);
        assert_eq!(
            ranked[1].ownership.unwrap().owned,
            90,
            "65 Forma and 25 blueprints"
        );
        assert_eq!(ranked[1].ownership.unwrap().needed, 1);
    }

    #[test]
    fn vaulted_flag() {
        let ranked = screen(
            &fixtures::inventory(),
            &[
                TRINITY_SYSTEMS_REWARD,
                "/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock",
                FORMA_REWARD,
            ],
        )
        .ranked;
        assert!(ranked[0].vaulted, "Trinity Prime is vaulted");
        assert!(!ranked[1].vaulted, "Braton Prime is not");
        assert!(!ranked[2].vaulted, "a Forma blueprint is never vaulted");
    }

    #[test]
    fn prime_set_components() {
        let held = fixtures::inventory_owning(&[(TRINITY_SYSTEMS, 1)]);
        let ranked = screen(
            &held,
            &[
                TRINITY_SYSTEMS_REWARD,
                "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/ExcaliburChassisComponent",
                FORMA_REWARD,
            ],
        )
        .ranked;

        let components = &ranked[0].components;
        assert_eq!(components.len(), 4, "the Orokin Cell is not a part");
        assert!(
            components
                .iter()
                .all(|component| !component.unique_name.contains("MiscItems"))
        );
        let systems = components
            .iter()
            .find(|component| component.unique_name == TRINITY_SYSTEMS)
            .unwrap();
        assert!(systems.this_reward);
        assert_eq!(systems.name, "Trinity Prime Systems");
        assert_eq!(systems.owned, Some(1));
        assert_eq!(systems.needed, 1);
        assert!(systems.enough);
        assert!(
            components
                .iter()
                .filter(|component| component.unique_name != TRINITY_SYSTEMS)
                .all(|component| !component.this_reward && !component.enough)
        );

        assert!(
            ranked[1].components.is_empty(),
            "neither the part nor the set it builds is Prime"
        );
        assert!(ranked[2].components.is_empty());
    }

    #[test]
    fn starred_component() {
        let starred: Favourites = [TRINITY_SYSTEMS].into_iter().collect();
        let ranked = recommend(
            Some(&fixtures::inventory()),
            &fixtures::catalog(),
            &prices(),
            &starred,
            &[TRINITY_SYSTEMS_REWARD.to_owned()],
        )
        .ranked;
        let stars: Vec<&str> = ranked[0]
            .components
            .iter()
            .filter(|component| component.favourite)
            .map(|component| component.unique_name.as_str())
            .collect();
        assert_eq!(stars, vec![TRINITY_SYSTEMS]);
    }

    #[test]
    fn set_price() {
        let ranked = screen(
            &fixtures::inventory(),
            &[TRINITY_SYSTEMS_REWARD, FORMA_REWARD],
        )
        .ranked;
        assert_eq!(ranked[0].set_plat, Some(140.0));
        assert_eq!(ranked[1].set_plat, None);
    }

    #[test]
    fn account_plat_and_ducats() {
        let screen = screen(&fixtures::inventory(), &[FORMA_REWARD]);
        assert_eq!(
            screen.account,
            Some(AccountBalance {
                plat: 5249,
                ducats: 937
            })
        );
    }

    #[test]
    fn no_inventory() {
        let screen = recommend(
            None,
            &fixtures::catalog(),
            &prices(),
            &Favourites::default(),
            &[TRINITY_SYSTEMS_REWARD.to_owned(), FORMA_REWARD.to_owned()],
        );
        assert_eq!(screen.account, None);
        let systems = &screen.ranked[0];
        assert_eq!(systems.name, "Trinity Prime Systems");
        assert_eq!(systems.set_plat, Some(140.0));
        assert_eq!(systems.ownership, None);
        assert!(!systems.components.is_empty());
        assert!(
            systems
                .components
                .iter()
                .all(|component| component.owned.is_none() && !component.enough)
        );
    }

    #[test]
    fn unpriced_rewards_all_best() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let unpriced = FixedPrices::new([("nothing_this_relic_drops", 1.0)]);
        let rewards = vec![
            "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint".to_owned(),
            "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/StyanaxPrimeBlueprint".to_owned(),
        ];
        let ranked = recommend(
            Some(&inventory),
            &catalog,
            &unpriced,
            &Favourites::default(),
            &rewards,
        )
        .ranked;
        assert!(ranked.iter().all(|entry| entry.best));
    }
}
