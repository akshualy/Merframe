use std::collections::{BTreeMap, BTreeSet, HashMap};

use wf_data::{Item, Rarity};
use wf_inventory::{EquipmentItem, Inventory, RIVEN_MARKER, RivenFingerprint, UpgradeSlot};

use super::{ModHolder, ModRow, UpgradePrices, display_name};
use crate::catalog::{ARCANE_PREFIX, Catalog, display_name_from_path};
use crate::prices::{PriceSource, market_slug};
use crate::view::View;

#[derive(Clone, Copy)]
pub(super) struct UnlistedUpgrade {
    unique_name: &'static str,
    pub(super) name: &'static str,
    rarity: Rarity,
    market_slug: Option<&'static str>,
    market_thumb: Option<&'static str>,
}

const UPGRADES_OUTSIDE_THE_EXPORT: [UnlistedUpgrade; 6] = [
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/AmmoEfficencyDuringUltimate",
        name: "Zid-An Haras",
        rarity: Rarity::Rare,
        market_slug: Some("zid-an-haras"),
        market_thumb: Some(
            "items/images/en/thumbs/zid-an-haras.56d691fea4cbd08f4dc35ac85d8fd1fb.128x128.webp",
        ),
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/HeatStatusProcOnUltimateKill",
        name: "Zid-An Uskos",
        rarity: Rarity::Rare,
        market_slug: Some("zid-an-uskos"),
        market_thumb: Some(
            "items/images/en/thumbs/zid-an-uskos.818fb0745c38781db3fa7319dcb0729a.128x128.webp",
        ),
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/StatusChanceOnUltimateHit",
        name: "Zid-An Asheir",
        rarity: Rarity::Rare,
        market_slug: Some("zid-an-asheir"),
        market_thumb: Some(
            "items/images/en/thumbs/zid-an-asheir.7b04215b7593ec4aca12d32ef276ef3c.128x128.webp",
        ),
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/UltimateInvisibilty",
        name: "Zid-An Sek-Eel",
        rarity: Rarity::Rare,
        market_slug: Some("zid-an-sek-eel"),
        market_thumb: Some(
            "items/images/en/thumbs/zid-an-sek-eel.8bc9e9fa897d0231cdeca3f6d8645e66.128x128.webp",
        ),
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/CosmeticEnhancers/Antiques/VoidSlingsOverguardStrip",
        name: "Zid-An Osbok",
        rarity: Rarity::Rare,
        market_slug: Some("zid-an-osbok"),
        market_thumb: Some(
            "items/images/en/thumbs/zid-an-osbok.bea2033c8ac9089bcff1ea005cdff694.128x128.webp",
        ),
    },
    UnlistedUpgrade {
        unique_name: "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser",
        name: "Legendary Core",
        rarity: Rarity::Legendary,
        market_slug: Some("legendary_fusion_core"),
        market_thumb: Some(
            "items/images/en/thumbs/legendary_fusion_core.094c2850f995a3d365d934296517c0d5.128x128.webp",
        ),
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpgradeKind {
    Mod,
    Arcane,
    Neither,
}

pub(super) fn upgrade_kind(item_type: &str) -> UpgradeKind {
    let peculiar = item_type.starts_with("/Lotus/Upgrades/CosmeticEnhancers/Peculiars/");
    if item_type.starts_with(ARCANE_PREFIX) && !peculiar {
        return UpgradeKind::Arcane;
    }
    if item_type.contains("/Beginner/") || item_type.starts_with("/Lotus/Upgrades/Stickers/") {
        return UpgradeKind::Neither;
    }
    UpgradeKind::Mod
}

fn is_riven(item_type: &str) -> bool {
    item_type.contains(RIVEN_MARKER)
}

fn starter_variant(unique_name: &str) -> Option<String> {
    let (parent, leaf) = unique_name.rsplit_once('/')?;
    Some(format!("{parent}/Beginner/{leaf}Beginner"))
}

fn upgrade_item<'a>(catalog: &'a Catalog, unique_name: &str) -> Option<&'a Item> {
    if let Some(item) = catalog.item(unique_name) {
        return Some(item);
    }
    catalog.item(&starter_variant(unique_name)?)
}

pub(super) fn upgrade_outside_the_export(unique_name: &str) -> Option<UnlistedUpgrade> {
    UPGRADES_OUTSIDE_THE_EXPORT
        .into_iter()
        .find(|upgrade| upgrade.unique_name == unique_name)
}

pub(crate) fn mods(view: &View) -> Vec<ModRow> {
    upgrade_rows(view, UpgradeKind::Mod)
}

pub(crate) fn arcanes(view: &View) -> Vec<ModRow> {
    upgrade_rows(view, UpgradeKind::Arcane)
}

fn equipped_holders(
    catalog: &Catalog,
    slots: &HashMap<&str, Vec<UpgradeSlot<'_>>>,
    instances: &BTreeSet<&str>,
) -> Vec<ModHolder> {
    let mut configs_by_item: BTreeMap<&str, (&EquipmentItem, BTreeSet<usize>)> = BTreeMap::new();
    for slot in instances
        .iter()
        .filter_map(|item_id| slots.get(*item_id))
        .flatten()
    {
        configs_by_item
            .entry(slot.item.item_id.as_str())
            .or_insert_with(|| (slot.item, BTreeSet::new()))
            .1
            .insert(slot.config);
    }
    let mut holders: Vec<ModHolder> = configs_by_item
        .into_iter()
        .map(|(item_id, (item, configs))| {
            let identity = item.identity_type();
            let known = catalog.item(identity);
            ModHolder {
                item_id: item_id.to_owned(),
                name: display_name(catalog, identity),
                custom_name: item.custom_name().map(str::to_owned),
                image_name: known.and_then(|known| known.image_name.clone()),
                rank: known.map(|known| known.mastery_rank_at(item.xp)),
                takes_orokin_reactor: known.is_some_and(Item::takes_orokin_reactor),
                orokin_upgrade: item.has_orokin_upgrade(),
                exilus_adapter: item.has_exilus_adapter(),
                configs: configs.into_iter().collect(),
                forma: item.polarized.unwrap_or_default(),
                archon_shards: item.archon_shards(),
            }
        })
        .collect();
    holders.sort_by(|a, b| a.name.cmp(&b.name));
    holders
}

fn upgrade_prices(
    prices: &dyn PriceSource,
    slug: &str,
    rank: Option<u32>,
    max_rank: Option<u32>,
    wanted: UpgradeKind,
) -> UpgradePrices {
    let at_max_rank = prices.plat_max_rank(slug).filter(|p| *p > 0.0);
    let maxed = rank.is_some() && rank == max_rank && at_max_rank.is_some();
    let sell = if maxed && wanted == UpgradeKind::Mod {
        at_max_rank
    } else {
        prices.plat(slug)
    };
    UpgradePrices {
        sell,
        sell_max_rank: if wanted == UpgradeKind::Arcane {
            at_max_rank
        } else {
            None
        },
        is_floor: rank.is_some_and(|rank| rank > 0) && !maxed && sell.is_some_and(|p| p > 0.0),
        buy: prices.buy_plat(slug),
    }
}

#[derive(Default)]
struct Tally<'a> {
    count: i64,
    holders: BTreeSet<&'a str>,
}

fn count_by_rank(
    inventory: &Inventory,
    wanted: UpgradeKind,
) -> BTreeMap<(&str, Option<u32>), Tally<'_>> {
    let keep = |item_type: &str| upgrade_kind(item_type) == wanted;
    let mut counted: BTreeMap<(&str, Option<u32>), Tally<'_>> = BTreeMap::new();
    for item in &inventory.raw_upgrades {
        if keep(&item.item_type) && item.item_count > 0 {
            let rank = if is_riven(&item.item_type) {
                Some(0)
            } else {
                None
            };
            counted
                .entry((item.item_type.as_str(), rank))
                .or_default()
                .count += item.item_count;
        }
    }
    for upgrade in &inventory.upgrades {
        if !keep(&upgrade.item_type) {
            continue;
        }
        let fingerprint = upgrade.fingerprint();
        if is_riven(&upgrade.item_type) {
            if fingerprint
                .as_ref()
                .is_some_and(RivenFingerprint::is_unveiled)
            {
                continue;
            }
            counted
                .entry((upgrade.item_type.as_str(), Some(0)))
                .or_default()
                .count += 1;
            continue;
        }
        let rank = fingerprint.map(|fingerprint| fingerprint.lvl);
        let tally = counted
            .entry((upgrade.item_type.as_str(), rank))
            .or_default();
        tally.count += 1;
        tally.holders.insert(upgrade.item_id.as_str());
    }
    counted
}

fn upgrade_rows(view: &View, wanted: UpgradeKind) -> Vec<ModRow> {
    let View {
        inventory,
        catalog,
        prices,
        favourites,
        listings,
    } = *view;
    let slots = inventory.upgrade_slots();
    let mut rows: Vec<ModRow> = count_by_rank(inventory, wanted)
        .into_iter()
        .filter(|(_, tally)| tally.count > 0)
        .map(|((unique_name, rank), Tally { count, holders })| {
            let known = upgrade_item(catalog, unique_name);
            let listed = upgrade_outside_the_export(unique_name);
            let mut name = match (known, listed) {
                (Some(item), _) => item.name.clone(),
                (None, Some(listed)) => listed.name.to_owned(),
                (None, None) => display_name_from_path(unique_name),
            };
            if is_riven(unique_name) {
                name.push_str(" (Veiled)");
            }
            let slug = match listed.and_then(|upgrade| upgrade.market_slug) {
                Some(slug) => slug.to_owned(),
                None => market_slug(&name),
            };
            let max_rank = known.map(Item::max_upgrade_rank);
            ModRow {
                count,
                rank,
                max_rank,
                prices: upgrade_prices(prices, &slug, rank, max_rank, wanted),
                equipped_in: equipped_holders(catalog, &slots, &holders),
                image_name: known.and_then(|item| item.image_name.clone()),
                market_thumb: listed
                    .and_then(|upgrade| upgrade.market_thumb)
                    .map(str::to_owned),
                rarity: known
                    .and_then(|item| item.rarity)
                    .or_else(|| listed.map(|upgrade| upgrade.rarity)),
                prime: name.contains("Prime"),
                favourite: favourites.contains(unique_name),
                order_placed: listings.has_order(&slug),
                unique_name: unique_name.to_owned(),
                market_slug: slug,
                name,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.rank.cmp(&b.rank)));
    rows
}

#[cfg(test)]
mod tests {
    use super::super::tests::{no_listings, prices};
    use super::*;
    use crate::catalog::fixtures;
    use crate::favourites::Favourites;
    use crate::prices::FixedPrices;

    const UPGRADE_ITEMS: &str = include_str!("../../../../fixtures/upgrade_items.json");

    fn upgrade_catalog() -> Catalog {
        Catalog::from_json(UPGRADE_ITEMS, fixtures::RELICS, "[]").unwrap()
    }

    #[test]
    fn every_upgrade_resolves() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let rows = [
            mods(&View {
                inventory: &inventory,
                catalog: &catalog,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &no_listings(),
            }),
            arcanes(&View {
                inventory: &inventory,
                catalog: &catalog,
                prices: &prices(),
                favourites: &Favourites::default(),
                listings: &no_listings(),
            }),
        ]
        .concat();
        let unresolved: Vec<&str> = rows
            .iter()
            .filter(|row| {
                upgrade_item(&catalog, &row.unique_name).is_none()
                    && upgrade_outside_the_export(&row.unique_name).is_none()
            })
            .map(|row| row.unique_name.as_str())
            .collect();
        assert!(unresolved.is_empty(), "{unresolved:?}");
        assert!(rows.iter().all(|row| !row.name.is_empty()));
    }

    #[test]
    fn stances_and_precepts_are_mods() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let rows = mods(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let stance = rows
            .iter()
            .find(|row| row.name == "Gaia's Tragedy")
            .unwrap();
        assert_eq!(stance.market_slug, "gaias_tragedy");
        assert!(stance.count > 0);
    }

    #[test]
    fn starter_only_mods() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let rows = mods(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let maglev = rows
            .iter()
            .find(|row| row.unique_name == "/Lotus/Upgrades/Mods/Warframe/AvatarSlideBoostMod")
            .unwrap();
        assert_eq!(maglev.name, "Maglev");
        assert_eq!(
            maglev.image_name.as_deref(),
            Some("SlideResistanceDecreaseMod.jpg")
        );
        assert!(
            rows.iter().any(|row| row.name == "Quick Thinking"),
            "the other starter-only mod resolves as well"
        );
        assert!(
            !rows
                .iter()
                .any(|row| row.unique_name.contains("/Beginner/")),
            "the starter copies themselves stay out of the tab"
        );
    }

    #[test]
    fn unlisted_upgrades() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let view = View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        };
        let arcane = arcanes(&view)
            .into_iter()
            .find(|row| {
                row.unique_name
                    == "/Lotus/Upgrades/CosmeticEnhancers/Antiques/AmmoEfficencyDuringUltimate"
                    && row.rank.is_none()
            })
            .unwrap();
        assert_eq!(arcane.name, "Zid-An Haras");
        assert_eq!(arcane.rarity, Some(Rarity::Rare));
        assert_eq!(arcane.count, 94);
        assert_eq!(arcane.market_slug, "zid-an-haras");
        assert_eq!(
            arcane.market_thumb.as_deref(),
            Some(
                "items/images/en/thumbs/zid-an-haras.56d691fea4cbd08f4dc35ac85d8fd1fb.128x128.webp"
            )
        );

        let core = mods(&view)
            .into_iter()
            .find(|row| row.unique_name == "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser")
            .expect("Legendary Core");
        assert_eq!(core.name, "Legendary Core");
        assert_eq!(core.count, 6);
        assert_eq!(core.market_slug, "legendary_fusion_core");
        assert_eq!(
            core.market_thumb.as_deref(),
            Some(
                "items/images/en/thumbs/legendary_fusion_core.094c2850f995a3d365d934296517c0d5.128x128.webp"
            )
        );
    }

    #[test]
    fn arcane_prices() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let prices = FixedPrices::new([("arcane_energize", 12.0), ("virtuos_surge", 9.0)])
            .with_max_rank([("arcane_energize", 240.0)])
            .with_buy([("arcane_energize", 7.0)]);
        let view = View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices,
            favourites: &Favourites::default(),
            listings: &no_listings(),
        };
        let rows = arcanes(&view);

        let energize: Vec<&ModRow> = rows
            .iter()
            .filter(|row| row.market_slug == "arcane_energize")
            .collect();
        assert!(!energize.is_empty());
        for row in &energize {
            assert_eq!(row.prices.sell, Some(12.0));
            assert_eq!(row.prices.sell_max_rank, Some(240.0));
            assert_eq!(row.prices.buy, Some(7.0));
            assert_eq!(row.max_rank, Some(5));
        }
        assert!(energize.iter().any(|row| row.rank == Some(5)));

        let surge = rows
            .iter()
            .find(|row| row.market_slug == "virtuos_surge")
            .expect("Virtuos Surge");
        assert_eq!(surge.prices.sell, Some(9.0));
        assert_eq!(
            surge.prices.sell_max_rank, None,
            "the bulk table carries no max rank sell price for it"
        );
        assert_eq!(surge.prices.buy, None);
        assert_eq!(
            surge.max_rank,
            Some(3),
            "an arcane that tops out at rank 3 says so"
        );

        let mod_rows = mods(&view);
        assert!(
            mod_rows
                .iter()
                .all(|row| row.prices.sell_max_rank.is_none())
        );
    }

    #[test]
    fn rank_groups_sorted_by_name() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let view = View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        };
        let mod_rows = mods(&view);
        let arcane_rows = arcanes(&view);

        assert!(
            mod_rows
                .iter()
                .all(|row| upgrade_kind(&row.unique_name) == UpgradeKind::Mod && row.count > 0)
        );
        assert!(
            arcane_rows
                .iter()
                .all(|row| upgrade_kind(&row.unique_name) == UpgradeKind::Arcane && row.count > 0)
        );
        assert!(!mod_rows.is_empty());
        assert!(!arcane_rows.is_empty());

        let unranked = mod_rows
            .iter()
            .find(|row| {
                row.unique_name.ends_with("Warframe/AvatarShieldMaxMod") && row.rank.is_none()
            })
            .expect("unranked row");
        assert_eq!(unranked.count, 6288);
        assert_eq!(unranked.name, "Avatar Shield Max Mod");
        assert!(arcane_rows.iter().any(|row| row.rank == Some(5)));
        let names: Vec<&str> = mod_rows.iter().map(|row| row.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn upgrade_kinds() {
        assert_eq!(
            upgrade_kind("/Lotus/Upgrades/CosmeticEnhancers/Peculiars/PeculiarBloom"),
            UpgradeKind::Mod
        );
        assert_eq!(
            upgrade_kind("/Lotus/Upgrades/CosmeticEnhancers/Enhancements/EnhancementEnergize"),
            UpgradeKind::Arcane
        );
        assert_eq!(
            upgrade_kind("/Lotus/Upgrades/Mods/Railjack/Hull/HullWeldMod"),
            UpgradeKind::Mod
        );
        assert_eq!(
            upgrade_kind("/Lotus/Upgrades/Mods/Beginner/BeginnerAmmoMod"),
            UpgradeKind::Neither
        );
        assert_eq!(
            upgrade_kind("/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod"),
            UpgradeKind::Mod
        );
        assert_eq!(
            upgrade_kind("/Lotus/Weapons/Tenno/Melee/MeleeTrees/FistCmbThreeMeleeTree"),
            UpgradeKind::Mod
        );
        assert_eq!(
            upgrade_kind("/Lotus/Types/Sentinels/SentinelPrecepts/BeastUniversalVacuum"),
            UpgradeKind::Mod
        );
        assert_eq!(
            upgrade_kind("/Lotus/Upgrades/Stickers/WeaponColdDamageSticker"),
            UpgradeKind::Neither
        );
    }

    #[test]
    fn veiled_rivens_only() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::catalog();
        let rows = mods(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let riven_rows: Vec<&ModRow> = rows
            .iter()
            .filter(|row| row.unique_name.contains(RIVEN_MARKER))
            .collect();
        assert!(!riven_rows.is_empty());
        assert!(
            riven_rows
                .iter()
                .all(|row| row.name.ends_with(" (Veiled)") && row.rank == Some(0))
        );
        let veiled_total: i64 = riven_rows.iter().map(|row| row.count).sum();
        assert_eq!(
            veiled_total, 184,
            "2 veiled rivens from Upgrades plus 182 pre-veiled stacks from RawUpgrades, and none of the 13 unveiled"
        );
    }

    #[test]
    fn equipped_in() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let rows = mods(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let equipped: Vec<&ModRow> = rows
            .iter()
            .filter(|row| !row.equipped_in.is_empty())
            .collect();
        assert!(!equipped.is_empty());
        assert!(equipped.len() < rows.len());
        assert!(
            equipped
                .iter()
                .all(|row| row.equipped_in.iter().all(|holder| {
                    !holder.name.is_empty()
                        && !holder.configs.is_empty()
                        && holder.configs.is_sorted()
                }))
        );
        assert!(equipped.iter().all(|row| {
            row.equipped_in
                .windows(2)
                .all(|pair| pair[0].name <= pair[1].name)
        }));
        assert!(
            rows.iter()
                .filter(|row| row.rank.is_none())
                .all(|row| row.equipped_in.is_empty()),
            "an unranked stack has no instance ids to trace"
        );
    }

    #[test]
    fn equipped_holders() {
        let inventory = fixtures::inventory();
        let catalog = fixtures::mastery_catalog();
        let rows = mods(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let braton = rows
            .iter()
            .flat_map(|row| &row.equipped_in)
            .find(|holder| holder.name == "Braton Prime")
            .unwrap();
        assert_eq!(braton.image_name.as_deref(), Some("BratonPrime.png"));
        assert_eq!(braton.configs, [0]);
        assert_eq!(braton.forma, 0);
        assert_eq!(braton.archon_shards, 0);
        assert_eq!(braton.rank, Some(30));
        assert!(!braton.takes_orokin_reactor);
        assert!(!braton.orokin_upgrade);
        assert!(!braton.exilus_adapter);
        assert_eq!(braton.custom_name, None);
        let equinox = arcanes(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &prices(),
            favourites: &Favourites::default(),
            listings: &no_listings(),
        })
        .into_iter()
        .flat_map(|row| row.equipped_in)
        .find(|holder| holder.name == "Equinox Prime")
        .unwrap();
        assert_eq!(equinox.forma, 1);
        assert_eq!(equinox.archon_shards, 5);
        assert_eq!(equinox.rank, Some(30));
        assert!(equinox.takes_orokin_reactor);
        assert!(equinox.orokin_upgrade);
        assert!(equinox.exilus_adapter);
    }

    #[test]
    fn floor_price_marker() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let priced = FixedPrices::new([("arcane_energize", 12.0), ("virtuos_surge", 9.0)])
            .with_max_rank([("arcane_energize", 240.0)]);
        let rows = arcanes(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &priced,
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });

        let at_max = rows
            .iter()
            .find(|row| row.market_slug == "arcane_energize" && row.rank == Some(5))
            .expect("maxed Energize");
        assert!(!at_max.prices.is_floor);

        let ranked_without_max_price = rows
            .iter()
            .find(|row| row.market_slug == "virtuos_surge" && row.rank.is_some_and(|rank| rank > 0))
            .expect("ranked Surge");
        assert!(ranked_without_max_price.prices.is_floor);

        assert!(
            rows.iter()
                .filter(|row| row.rank == Some(0) || row.rank.is_none())
                .all(|row| !row.prices.is_floor)
        );
        assert!(
            rows.iter()
                .filter(|row| row.prices.sell.is_none())
                .all(|row| !row.prices.is_floor)
        );
    }

    #[test]
    fn maxed_serration_price() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let priced = FixedPrices::new([("serration", 10.0)]).with_max_rank([("serration", 90.0)]);
        let rows = mods(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &priced,
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let serration: Vec<&ModRow> = rows
            .iter()
            .filter(|row| row.market_slug == "serration")
            .collect();
        assert_eq!(serration.len(), 3);
        assert!(serration.iter().all(|row| row.max_rank == Some(10)));

        let unranked = serration.iter().find(|row| row.rank.is_none()).unwrap();
        assert_eq!(unranked.prices.sell, Some(10.0));
        assert!(!unranked.prices.is_floor);

        let middling = serration
            .iter()
            .find(|row| row.rank == Some(5))
            .expect("rank 5");
        assert_eq!(middling.prices.sell, Some(10.0));
        assert!(middling.prices.is_floor);

        let maxed = serration.iter().find(|row| row.rank == Some(10)).unwrap();
        assert_eq!(maxed.prices.sell, Some(90.0));
        assert!(!maxed.prices.is_floor);
        assert_eq!(
            maxed.prices.sell_max_rank, None,
            "a mod shows the one price Aleca shows"
        );
    }

    #[test]
    fn maxed_without_max_rank_price() {
        let inventory = fixtures::inventory();
        let catalog = upgrade_catalog();
        let priced = FixedPrices::new([("serration", 10.0)]);
        let rows = mods(&View {
            inventory: &inventory,
            catalog: &catalog,
            prices: &priced,
            favourites: &Favourites::default(),
            listings: &no_listings(),
        });
        let maxed = rows
            .iter()
            .find(|row| row.market_slug == "serration" && row.rank == Some(10))
            .unwrap();
        assert_eq!(maxed.prices.sell, Some(10.0));
        assert!(maxed.prices.is_floor);
    }
}
