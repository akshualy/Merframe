use std::collections::HashSet;

use serde::Serialize;
use wf_data::{Component, Item};

use crate::account::Account;
use crate::catalog::{Catalog, DUCATS_ITEM, Stock, names_a_prime};
use crate::favourites::Favourites;
use crate::identity::{ItemTable, Variant, part_identity};
use crate::inventory_view::ItemSummary;
use crate::prices::PriceSource;

use super::missing_parts;
use super::rewards::{RewardOwnership, favourite_part, favourite_reward};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RankedReward {
    pub item: ItemSummary,
    pub plat: Option<f64>,
    pub set_plat: Option<f64>,
    pub ownership: Option<RewardOwnership>,
    pub favourite: bool,
    pub best: bool,
    pub components: Vec<RankedComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RankedComponent {
    pub item: ItemSummary,
    pub owned: Option<i64>,
    pub needed: u32,
    pub enough: bool,
    pub this_reward: bool,
    pub favourite: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct RewardScreen {
    pub ranked: Vec<RankedReward>,
    pub account: Option<AccountBalance>,
}

impl RewardScreen {
    #[must_use]
    pub fn chat_line(&self) -> String {
        self.ranked
            .iter()
            .filter_map(|reward| {
                let name = reward.item.name.trim_end_matches(" Blueprint");
                Some(format!("[{name}] {}:platinum:", reward.plat?.round()))
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct AccountBalance {
    pub plat: i64,
    pub ducats: i64,
}

struct Holdings<'a> {
    account: &'a Account,
    missing: HashSet<String>,
    building: HashSet<&'a str>,
}

impl<'a> Holdings<'a> {
    fn new(account: &'a Account, catalog: &Catalog, items: &ItemTable) -> Self {
        Self {
            account,
            missing: missing_parts(account, catalog, items)
                .iter()
                .map(|part| part_identity(&part.unique_name).to_owned())
                .collect(),
            building: account
                .inventory
                .pending_recipes
                .iter()
                .map(|recipe| recipe.item_type.as_str())
                .collect(),
        }
    }

    fn ownership(&self, unique_name: &str, parent: Option<(&Item, &Component)>) -> RewardOwnership {
        RewardOwnership {
            owned: self.account.stock.count(unique_name),
            needed: parent.map_or(1, |(_, component)| component.item_count),
            needed_for_set: self.missing.contains(part_identity(unique_name)),
            parent_owned: parent.is_some_and(|(item, _)| self.built_or_building(item)),
        }
    }

    fn built_or_building(&self, item: &Item) -> bool {
        self.account.holds(item)
            || item
                .components
                .as_deref()
                .unwrap_or_default()
                .iter()
                .any(|component| self.building.contains(component.unique_name.as_str()))
    }
}

fn mark_best(ranked: &mut [RankedReward]) {
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
    account: Option<&Account>,
    catalog: &Catalog,
    items: &ItemTable,
    prices: &dyn PriceSource,
    favourites: &Favourites,
    rewards: &[ItemSummary],
) -> RewardScreen {
    let holdings = account.map(|account| Holdings::new(account, catalog, items));
    let mut ranked: Vec<RankedReward> = rewards
        .iter()
        .map(|reward| {
            let unique_name = reward.unique_name.as_str();
            let parent = catalog.component_for_reward(unique_name);
            RankedReward {
                item: reward.clone(),
                plat: reward
                    .market_slug
                    .as_deref()
                    .and_then(|slug| prices.plat(slug)),
                set_plat: parent
                    .and_then(|(item, _)| items.variant(&item.unique_name, Variant::Set))
                    .and_then(|set| set.market_slug.as_deref())
                    .and_then(|slug| prices.plat(slug)),
                ownership: holdings
                    .as_ref()
                    .map(|holdings| holdings.ownership(unique_name, parent)),
                favourite: favourite_reward(catalog, favourites, unique_name),
                best: false,
                components: match parent {
                    Some((item, component)) => set_components(
                        items,
                        item,
                        &component.unique_name,
                        &reward.name,
                        account.map(|account| &account.stock),
                        favourites,
                    ),
                    None => Vec::new(),
                },
            }
        })
        .collect();
    mark_best(&mut ranked);
    RewardScreen {
        ranked,
        account: account.map(|account| AccountBalance {
            plat: account.inventory.premium_credits,
            ducats: account.inventory.counted(DUCATS_ITEM),
        }),
    }
}

fn is_prime_part(unique_name: &str) -> bool {
    !unique_name.contains("/MiscItems/") && !unique_name.contains("/Research/")
}

fn set_components(
    items: &ItemTable,
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
        .filter_map(|component| {
            let part = items.part(item, component)?;
            let owned = stock.map(|stock| stock.count(&component.unique_name));
            Some(RankedComponent {
                item: ItemSummary::from(part),
                owned,
                needed: component.item_count,
                enough: owned.is_some_and(|owned| owned >= i64::from(component.item_count)),
                this_reward: component.unique_name == reward_component,
                favourite: favourite_part(favourites, item, component),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::tests::prices;
    use super::*;
    use crate::account::Account;
    use crate::catalog::{VaultStatus, fixtures};
    use crate::prices::FixedPrices;
    use fixtures::{BRATON_PRIME_STOCK, TRINITY_PRIME_SYSTEMS, TRINITY_PRIME_SYSTEMS_BLUEPRINT};
    use serde_json::json;
    use wf_inventory::Inventory;

    const TRINITY_SYSTEMS_REWARD: &str =
        "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/TrinityPrimeSystemsBlueprint";
    const STYANAX_REWARD: &str =
        "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/StyanaxPrimeBlueprint";
    const FORMA_REWARD: &str = "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint";
    const BRATON_STOCK_REWARD: &str =
        "/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock";

    fn rank(
        account: Option<&Account>,
        catalog: &Catalog,
        prices: &dyn PriceSource,
        favourites: &Favourites,
        rewards: &[&str],
    ) -> RewardScreen {
        let items = ItemTable::build(catalog);
        let rewards: Vec<ItemSummary> =
            rewards.iter().map(|reward| items.summary(reward)).collect();
        recommend(account, catalog, &items, prices, favourites, &rewards)
    }

    fn screen(inventory: Inventory, rewards: &[&str]) -> RewardScreen {
        rank(
            Some(&Account::new(inventory)),
            &fixtures::catalog(),
            &prices(),
            &Favourites::default(),
            rewards,
        )
    }

    #[test]
    fn kavasa_prime_band() {
        let screen = rank(
            Some(&Account::new(fixtures::inventory())),
            &fixtures::with_skins(fixtures::ITEMS),
            &prices(),
            &Favourites::default(),
            &["/Lotus/StoreItems/Types/Recipes/Kubrow/Collars/PrimeKubrowCollarABandComponent"],
        );
        let band = &screen.ranked[0].item;
        assert_eq!(band.name, "Kavasa Prime Band");
        assert_eq!(band.ducats, Some(45));
        assert_eq!(
            band.image_name.as_deref(),
            Some("GenericComponentPrimeLatch.png")
        );
    }

    #[test]
    fn chat_line_priced_rewards() {
        let screen = screen(
            fixtures::inventory(),
            &[STYANAX_REWARD, FORMA_REWARD, TRINITY_SYSTEMS_REWARD],
        );
        assert_eq!(
            screen.chat_line(),
            "[Styanax Prime] 100:platinum: | [Trinity Prime Systems] 12:platinum:"
        );
    }

    #[test]
    fn screen_order_and_best() {
        let ranked = screen(
            fixtures::inventory(),
            &[
                "/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/AlternoxPrimeStock",
                STYANAX_REWARD,
                FORMA_REWARD,
                TRINITY_SYSTEMS_REWARD,
            ],
        )
        .ranked;
        assert_eq!(ranked.len(), 4);
        assert_eq!(ranked[0].item.name, "Alternox Prime Stock");
        assert_eq!(ranked[1].item.name, "Styanax Prime Blueprint");
        assert_eq!(ranked[2].item.name, "Forma Blueprint");
        assert_eq!(ranked[3].item.name, "Trinity Prime Systems");

        assert!(!ranked[0].best);
        assert!(ranked[1].best);
        assert!(!ranked[2].best);
        assert!(!ranked[3].best);

        assert_eq!(ranked[1].plat, Some(100.0));
        assert_eq!(ranked[3].plat, Some(12.0));
        assert_eq!(ranked[3].item.ducats, Some(15));
        assert!(ranked[3].ownership.unwrap().needed_for_set);
        assert_eq!(ranked[3].ownership.unwrap().owned, 0);
        assert_eq!(ranked[2].plat, None);
        assert!(ranked[2].ownership.unwrap().owned > 0);
        assert!(ranked.iter().all(|entry| !entry.favourite));
    }

    #[test]
    fn starred_part_or_parent() {
        let starred: Favourites = [BRATON_PRIME_STOCK, "/Lotus/Powersuits/Trinity/TrinityPrime"]
            .into_iter()
            .collect();
        let ranked = rank(
            Some(&Account::new(fixtures::inventory())),
            &fixtures::catalog(),
            &prices(),
            &starred,
            &[BRATON_STOCK_REWARD, TRINITY_SYSTEMS_REWARD, FORMA_REWARD],
        )
        .ranked;
        assert!(ranked[0].favourite, "the part itself is starred");
        assert!(
            ranked[1].favourite,
            "the Warframe the part builds is starred"
        );
        assert!(!ranked[2].favourite);
    }

    #[test]
    fn blueprint_reward_keeps_its_own_name() {
        let starred: Favourites = [TRINITY_PRIME_SYSTEMS_BLUEPRINT].into_iter().collect();
        let ranked = rank(
            Some(&Account::new(fixtures::inventory_owning(&[(
                TRINITY_PRIME_SYSTEMS_BLUEPRINT,
                2,
            )]))),
            &fixtures::catalog(),
            &prices(),
            &starred,
            &[TRINITY_SYSTEMS_REWARD],
        )
        .ranked;
        assert_eq!(ranked[0].item.unique_name, TRINITY_PRIME_SYSTEMS_BLUEPRINT);
        assert_eq!(ranked[0].item.name, "Trinity Prime Systems");
        assert!(ranked[0].favourite);
        assert_eq!(ranked[0].ownership.unwrap().owned, 2);
    }

    fn inventory_without_trinity_prime(building: Option<&str>) -> Inventory {
        fixtures::inventory_with(|value| {
            fixtures::drop_entries(
                value,
                &["Suits", "XPInfo"],
                "/Lotus/Powersuits/Trinity/TrinityPrime",
            );
            if let Some(item_type) = building {
                fixtures::entries(value, "PendingRecipes").push(json!({
                    "ItemType": item_type,
                    "CompletionDate": { "$date": { "$numberLong": "1700000000000" } },
                    "ItemId": { "$oid": "000000000000000000000000" }
                }));
            }
        })
    }

    #[test]
    fn parent_owned() {
        let owned = screen(fixtures::inventory(), &[TRINITY_SYSTEMS_REWARD]);
        assert!(owned.ranked[0].ownership.unwrap().parent_owned);

        let missing = inventory_without_trinity_prime(None);
        assert!(
            !screen(missing, &[TRINITY_SYSTEMS_REWARD]).ranked[0]
                .ownership
                .unwrap()
                .parent_owned
        );

        let building = inventory_without_trinity_prime(Some(
            "/Lotus/Types/Recipes/WarframeRecipes/TrinityPrimeChassisComponent",
        ));
        assert!(
            screen(building, &[TRINITY_SYSTEMS_REWARD]).ranked[0]
                .ownership
                .unwrap()
                .parent_owned,
            "a component of the set is in the foundry"
        );
    }

    #[test]
    fn owned_against_needed() {
        let held = fixtures::inventory_owning(&[(TRINITY_PRIME_SYSTEMS, 2)]);
        let ranked = screen(held, &[TRINITY_SYSTEMS_REWARD, FORMA_REWARD]).ranked;
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
    fn vault_status() {
        let ranked = screen(
            fixtures::inventory(),
            &[TRINITY_SYSTEMS_REWARD, BRATON_STOCK_REWARD, FORMA_REWARD],
        )
        .ranked;
        assert_eq!(ranked[0].item.vault, Some(VaultStatus::Vaulted));
        assert_eq!(ranked[1].item.vault, Some(VaultStatus::Available));
        assert_eq!(
            ranked[2].item.vault, None,
            "a Forma blueprint is never vaulted"
        );
    }

    #[test]
    fn prime_set_components() {
        let held = fixtures::inventory_owning(&[(TRINITY_PRIME_SYSTEMS, 1)]);
        let ranked = screen(
            held,
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
                .all(|component| !component.item.unique_name.contains("MiscItems"))
        );
        let systems = components
            .iter()
            .find(|component| component.item.unique_name == TRINITY_PRIME_SYSTEMS)
            .unwrap();
        assert!(systems.this_reward);
        assert_eq!(systems.item.name, "Trinity Prime Systems");
        assert_eq!(systems.owned, Some(1));
        assert_eq!(systems.needed, 1);
        assert!(systems.enough);
        assert!(
            components
                .iter()
                .filter(|component| component.item.unique_name != TRINITY_PRIME_SYSTEMS)
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
        let starred: Favourites = [TRINITY_PRIME_SYSTEMS].into_iter().collect();
        let ranked = rank(
            Some(&Account::new(fixtures::inventory())),
            &fixtures::catalog(),
            &prices(),
            &starred,
            &[TRINITY_SYSTEMS_REWARD],
        )
        .ranked;
        let stars: Vec<&str> = ranked[0]
            .components
            .iter()
            .filter(|component| component.favourite)
            .map(|component| component.item.unique_name.as_str())
            .collect();
        assert_eq!(stars, vec![TRINITY_PRIME_SYSTEMS]);
    }

    #[test]
    fn set_price() {
        let ranked = screen(
            fixtures::inventory(),
            &[TRINITY_SYSTEMS_REWARD, FORMA_REWARD],
        )
        .ranked;
        assert_eq!(ranked[0].set_plat, Some(140.0));
        assert_eq!(ranked[1].set_plat, None);
    }

    #[test]
    fn account_plat_and_ducats() {
        let screen = screen(fixtures::inventory(), &[FORMA_REWARD]);
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
        let screen = rank(
            None,
            &fixtures::catalog(),
            &prices(),
            &Favourites::default(),
            &[TRINITY_SYSTEMS_REWARD, FORMA_REWARD],
        );
        assert_eq!(screen.account, None);
        let systems = &screen.ranked[0];
        assert_eq!(systems.item.name, "Trinity Prime Systems");
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
        let unpriced = FixedPrices::new([("nothing_this_relic_drops", 1.0)]);
        let ranked = rank(
            Some(&Account::new(fixtures::inventory())),
            &fixtures::catalog(),
            &unpriced,
            &Favourites::default(),
            &[FORMA_REWARD, STYANAX_REWARD],
        )
        .ranked;
        assert!(ranked.iter().all(|entry| entry.best));
    }
}
