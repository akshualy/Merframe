use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use wf_data::Item;
use wf_inventory::{EquipmentItem, Inventory, RivenFingerprint, UpgradeSlot};

use super::{ItemIndex, ItemSummary, ModHolder, ModRow, UpgradePrices, display_name};
use crate::catalog::{ARCANE_PREFIX, Catalog, display_name_from_path};
use crate::identity::{ItemKind, Variant, is_riven};
use crate::prices::PriceSource;
use crate::view::View;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpgradeKind {
    Mod,
    Arcane,
    Neither,
}

pub(crate) fn upgrade_kind(item_type: &str) -> UpgradeKind {
    let peculiar = item_type.starts_with("/Lotus/Upgrades/CosmeticEnhancers/Peculiars/");
    if item_type.starts_with(ARCANE_PREFIX) && !peculiar {
        return UpgradeKind::Arcane;
    }
    if item_type.contains("/Beginner/") || item_type.starts_with("/Lotus/Upgrades/Stickers/") {
        return UpgradeKind::Neither;
    }
    UpgradeKind::Mod
}

pub(crate) fn mods(view: &View, index: &mut ItemIndex) -> Vec<ModRow> {
    upgrade_rows(view, index, UpgradeKind::Mod)
}

pub(crate) fn arcanes(view: &View, index: &mut ItemIndex) -> Vec<ModRow> {
    upgrade_rows(view, index, UpgradeKind::Arcane)
}

fn equipped_holders(
    catalog: &Catalog,
    slots: &HashMap<&str, Vec<UpgradeSlot<'_>>>,
    references: &BTreeSet<&str>,
    index: &mut ItemIndex,
) -> Vec<usize> {
    let mut configs_by_item: BTreeMap<&str, (&EquipmentItem, BTreeSet<usize>)> = BTreeMap::new();
    for slot in references
        .iter()
        .filter_map(|reference| slots.get(*reference))
        .flatten()
    {
        configs_by_item
            .entry(slot.item.item_id.as_str())
            .or_insert_with(|| (slot.item, BTreeSet::new()))
            .1
            .insert(slot.config);
    }
    let mut holders: Vec<usize> = configs_by_item
        .into_iter()
        .map(|(item_id, (item, configs))| {
            let configs: Vec<usize> = configs.into_iter().collect();
            index.add_holder((item_id.to_owned(), configs.clone()), || {
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
                    configs,
                    forma: item.polarized.unwrap_or_default(),
                    archon_shards: item.archon_shards(),
                }
            })
        })
        .collect();
    holders.sort_by(|a, b| index.holder(*a).name.cmp(&index.holder(*b).name));
    holders
}

fn upgrade_prices(
    prices: &dyn PriceSource,
    slug: Option<&str>,
    rank: Option<u32>,
    max_rank: Option<u32>,
    wanted: UpgradeKind,
) -> UpgradePrices {
    let at_max_rank = slug
        .and_then(|slug| prices.plat_max_rank(slug))
        .filter(|p| *p > 0.0);
    let maxed = rank.is_some() && rank == max_rank && at_max_rank.is_some();
    let sell = if maxed && wanted == UpgradeKind::Mod {
        at_max_rank
    } else {
        slug.and_then(|slug| prices.plat(slug))
    };
    UpgradePrices {
        sell,
        sell_max_rank: if wanted == UpgradeKind::Arcane {
            at_max_rank
        } else {
            None
        },
        is_floor: rank.is_some_and(|rank| rank > 0) && !maxed && sell.is_some_and(|p| p > 0.0),
        buy: slug.and_then(|slug| prices.buy_plat(slug)),
    }
}

#[derive(Default)]
struct Tally<'a> {
    count: i64,
    references: BTreeSet<&'a str>,
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
            let tally = counted.entry((item.item_type.as_str(), rank)).or_default();
            tally.count += item.item_count;
            if rank.is_none() {
                tally.references.insert(item.item_type.as_str());
            }
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
        tally.references.insert(upgrade.item_id.as_str());
    }
    counted
}

fn upgrade_rows(view: &View, index: &mut ItemIndex, wanted: UpgradeKind) -> Vec<ModRow> {
    let View {
        account,
        catalog,
        items,
        prices,
        favourites,
        ..
    } = *view;
    let inventory = &account.inventory;
    let slots = inventory.upgrade_slots();
    let mut rows: Vec<ModRow> = count_by_rank(inventory, wanted)
        .into_iter()
        .filter(|(_, tally)| tally.count > 0)
        .map(|((unique_name, rank), Tally { count, references })| {
            let riven = is_riven(unique_name);
            let record = match items.variant(unique_name, Variant::Veiled) {
                Some(veiled) => Cow::Borrowed(veiled),
                None => items.resolve(unique_name, || {
                    let name = display_name_from_path(unique_name);
                    if riven {
                        format!("{name} (Veiled)")
                    } else {
                        name
                    }
                }),
            };
            let (rarity, max_rank) = match record.kind {
                ItemKind::Upgrade { rarity, max_rank } => (rarity, max_rank),
                _ => (None, None),
            };
            let slug = record.market_slug.as_deref();
            ModRow {
                item: index.add_marked(
                    ItemSummary::new(unique_name, &record),
                    favourites.contains(unique_name),
                ),
                count,
                rank,
                max_rank,
                prices: upgrade_prices(prices, slug, rank, max_rank, wanted),
                rarity,
                equipped_in: equipped_holders(catalog, &slots, &references, index),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        index[a.item]
            .name
            .cmp(&index[b.item].name)
            .then(a.rank.cmp(&b.rank))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::super::tests::prices;
    use super::*;
    use crate::catalog::fixtures;
    use crate::prices::FixedPrices;
    use crate::view::Fixture;
    use wf_data::Rarity;

    #[test]
    fn every_upgrade_resolves() {
        let fixture =
            Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory()).with_prices(prices());
        let view = fixture.view();
        let mut index = ItemIndex::default();
        let rows = [mods(&view, &mut index), arcanes(&view, &mut index)].concat();
        let unresolved: Vec<&str> = rows
            .iter()
            .filter(|row| fixture.items.get(&index[row.item].unique_name).is_none())
            .map(|row| index[row.item].unique_name.as_str())
            .collect();
        assert!(unresolved.is_empty(), "{unresolved:?}");
        assert!(rows.iter().all(|row| !index[row.item].name.is_empty()));
    }

    #[test]
    fn stances_and_precepts_are_mods() {
        let fixture =
            Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory()).with_prices(prices());
        let (rows, index) = fixture.rows(mods);
        let stance = rows
            .iter()
            .find(|row| index[row.item].name == "Gaia's Tragedy")
            .unwrap();
        assert_eq!(
            index[stance.item].market_slug.as_deref(),
            Some("gaias_tragedy")
        );
        assert!(stance.count > 0);
    }

    #[test]
    fn starter_only_mods() {
        let fixture =
            Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory()).with_prices(prices());
        let (rows, index) = fixture.rows(mods);
        let maglev = rows
            .iter()
            .find(|row| {
                index[row.item].unique_name == "/Lotus/Upgrades/Mods/Warframe/AvatarSlideBoostMod"
            })
            .unwrap();
        assert_eq!(index[maglev.item].name, "Maglev");
        assert_eq!(
            index[maglev.item].image_name.as_deref(),
            Some("SlideResistanceDecreaseMod.jpg")
        );
        assert!(
            rows.iter()
                .any(|row| index[row.item].name == "Quick Thinking"),
            "the other starter-only mod resolves as well"
        );
        assert!(
            !rows
                .iter()
                .any(|row| index[row.item].unique_name.contains("/Beginner/")),
            "the starter copies themselves stay out of the tab"
        );
    }

    #[test]
    fn unlisted_upgrades() {
        let fixture =
            Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory()).with_prices(prices());
        let (found, index) = fixture.rows(arcanes);
        let arcane = found
            .into_iter()
            .find(|row| {
                index[row.item].unique_name
                    == "/Lotus/Upgrades/CosmeticEnhancers/Antiques/AmmoEfficencyDuringUltimate"
                    && row.rank.is_none()
            })
            .unwrap();
        assert_eq!(index[arcane.item].name, "Zid-An Haras");
        assert_eq!(arcane.rarity, Some(Rarity::Rare));
        assert_eq!(arcane.count, 94);
        assert_eq!(
            index[arcane.item].market_slug.as_deref(),
            Some("zid-an-haras")
        );

        let (found, index) = fixture.rows(mods);
        let core = found
            .into_iter()
            .find(|row| {
                index[row.item].unique_name == "/Lotus/Upgrades/Mods/Fusers/LegendaryModFuser"
            })
            .expect("Legendary Core");
        assert_eq!(index[core.item].name, "Legendary Core");
        assert_eq!(core.count, 6);
        assert_eq!(
            index[core.item].market_slug.as_deref(),
            Some("legendary_fusion_core")
        );
        assert_eq!(
            index[core.item].image_name.as_deref(),
            Some("game/legendary-core.png")
        );
    }

    #[test]
    fn arcane_prices() {
        let fixture = Fixture {
            prices: FixedPrices::new([("arcane_energize", 12.0), ("virtuos_surge", 9.0)])
                .with_max_rank([("arcane_energize", 240.0)])
                .with_buy([("arcane_energize", 7.0)]),
            ..Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory())
        };
        let (rows, index) = fixture.rows(arcanes);

        let energize: Vec<&ModRow> = rows
            .iter()
            .filter(|row| index[row.item].market_slug.as_deref() == Some("arcane_energize"))
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
            .find(|row| index[row.item].market_slug.as_deref() == Some("virtuos_surge"))
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

        let (mod_rows, _) = fixture.rows(mods);
        assert!(
            mod_rows
                .iter()
                .all(|row| row.prices.sell_max_rank.is_none())
        );
    }

    #[test]
    fn listed_slug_wins_over_the_derived_one() {
        let mut fixture = Fixture {
            prices: FixedPrices::new([("arcane_energize", 12.0), ("arcane\u{2019}energize", 30.0)]),
            ..Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory())
        };
        let listed: Vec<wf_market::Item> = serde_json::from_str(
            r#"[{"id":"1","slug":"arcane\u2019energize","gameRef":"/Lotus/Upgrades/CosmeticEnhancers/Utility/GolemArcaneRadialEnergyOnEnergyPickup","tags":[],"i18n":{}}]"#,
        )
        .unwrap();
        fixture.items.index_market(&listed);
        let (found, index) = fixture.rows(arcanes);
        let energize = found
            .into_iter()
            .find(|row| index[row.item].name == "Arcane Energize")
            .expect("Arcane Energize");
        assert_eq!(
            index[energize.item].market_slug.as_deref(),
            Some("arcane\u{2019}energize")
        );
        assert_eq!(energize.prices.sell, Some(30.0));
    }

    #[test]
    fn rank_groups_sorted_by_name() {
        let fixture =
            Fixture::new(fixtures::catalog(), fixtures::inventory()).with_prices(prices());
        let (mod_rows, mod_index) = fixture.rows(mods);
        let (arcane_rows, arcane_index) = fixture.rows(arcanes);

        assert!(
            mod_rows
                .iter()
                .all(
                    |row| upgrade_kind(&mod_index[row.item].unique_name) == UpgradeKind::Mod
                        && row.count > 0
                )
        );
        assert!(
            arcane_rows
                .iter()
                .all(|row| upgrade_kind(&arcane_index[row.item].unique_name)
                    == UpgradeKind::Arcane
                    && row.count > 0)
        );
        assert!(!mod_rows.is_empty());
        assert!(!arcane_rows.is_empty());

        let unranked = mod_rows
            .iter()
            .find(|row| {
                mod_index[row.item]
                    .unique_name
                    .ends_with("Warframe/AvatarShieldMaxMod")
                    && row.rank.is_none()
            })
            .expect("unranked row");
        assert_eq!(unranked.count, 6288);
        assert_eq!(mod_index[unranked.item].name, "Avatar Shield Max Mod");
        assert!(arcane_rows.iter().any(|row| row.rank == Some(5)));
        let names: Vec<&str> = mod_rows
            .iter()
            .map(|row| mod_index[row.item].name.as_str())
            .collect();
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
        let fixture =
            Fixture::new(fixtures::catalog(), fixtures::inventory()).with_prices(prices());
        let (rows, index) = fixture.rows(mods);
        let riven_rows: Vec<&ModRow> = rows
            .iter()
            .filter(|row| is_riven(&index[row.item].unique_name))
            .collect();
        assert!(!riven_rows.is_empty());
        assert!(
            riven_rows
                .iter()
                .all(|row| index[row.item].name.ends_with(" (Veiled)") && row.rank == Some(0))
        );
        let veiled_total: i64 = riven_rows.iter().map(|row| row.count).sum();
        assert_eq!(
            veiled_total, 184,
            "2 veiled rivens from Upgrades plus 182 pre-veiled stacks from RawUpgrades, and none of the 13 unveiled"
        );
    }

    #[test]
    fn equipped_in() {
        let fixture =
            Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory()).with_prices(prices());
        let (rows, index) = fixture.rows(mods);
        let equipped: Vec<&ModRow> = rows
            .iter()
            .filter(|row| !row.equipped_in.is_empty())
            .collect();
        assert!(!equipped.is_empty());
        assert!(equipped.len() < rows.len());
        assert!(
            equipped
                .iter()
                .all(|row| row.equipped_in.iter().all(|&holder| {
                    let holder = index.holder(holder);
                    !holder.name.is_empty()
                        && !holder.configs.is_empty()
                        && holder.configs.is_sorted()
                }))
        );
        assert!(equipped.iter().all(|row| {
            row.equipped_in
                .windows(2)
                .all(|pair| index.holder(pair[0]).name <= index.holder(pair[1]).name)
        }));
    }

    #[test]
    fn unranked_stack_equipped_by_type() {
        const SCOURGE: &str = "/Lotus/Upgrades/Mods/Melee/DualStat/PoisonEventMeleeMod";
        let mut value: serde_json::Value = serde_json::from_str(fixtures::INVENTORY).unwrap();
        value["RawUpgrades"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "ItemType": SCOURGE, "ItemCount": 4 }));
        let inventory = Inventory::parse(&value.to_string()).unwrap();
        let fixture = Fixture::new(fixtures::mastery_catalog(), inventory);
        let (rows, index) = fixture.rows(mods);
        let stack = rows
            .iter()
            .find(|row| index[row.item].unique_name == SCOURGE && row.rank.is_none())
            .unwrap();
        assert_eq!(stack.count, 4);
        let holders: Vec<&ModHolder> = stack
            .equipped_in
            .iter()
            .map(|&holder| index.holder(holder))
            .collect();
        assert_eq!(holders.len(), 1);
        assert_eq!(holders[0].name, "Wolf Sledge");
        assert_eq!(holders[0].configs, [0]);
    }

    #[test]
    fn equipped_holders() {
        let fixture =
            Fixture::new(fixtures::mastery_catalog(), fixtures::inventory()).with_prices(prices());
        let (rows, index) = fixture.rows(mods);
        let braton = rows
            .iter()
            .flat_map(|row| &row.equipped_in)
            .map(|&holder| index.holder(holder))
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
        let (found, arcane_index) = fixture.rows(arcanes);
        let equinox = found
            .iter()
            .flat_map(|row| &row.equipped_in)
            .map(|&holder| arcane_index.holder(holder))
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
        let fixture = Fixture {
            prices: FixedPrices::new([("arcane_energize", 12.0), ("virtuos_surge", 9.0)])
                .with_max_rank([("arcane_energize", 240.0)]),
            ..Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory())
        };
        let (rows, index) = fixture.rows(arcanes);

        let at_max = rows
            .iter()
            .find(|row| {
                index[row.item].market_slug.as_deref() == Some("arcane_energize")
                    && row.rank == Some(5)
            })
            .expect("maxed Energize");
        assert!(!at_max.prices.is_floor);

        let ranked_without_max_price = rows
            .iter()
            .find(|row| {
                index[row.item].market_slug.as_deref() == Some("virtuos_surge")
                    && row.rank.is_some_and(|rank| rank > 0)
            })
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
        let fixture = Fixture {
            prices: FixedPrices::new([("serration", 10.0)]).with_max_rank([("serration", 90.0)]),
            ..Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory())
        };
        let (rows, index) = fixture.rows(mods);
        let serration: Vec<&ModRow> = rows
            .iter()
            .filter(|row| index[row.item].market_slug.as_deref() == Some("serration"))
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
        let fixture = Fixture {
            prices: FixedPrices::new([("serration", 10.0)]),
            ..Fixture::new(fixtures::upgrade_catalog(), fixtures::inventory())
        };
        let (rows, index) = fixture.rows(mods);
        let maxed = rows
            .iter()
            .find(|row| {
                index[row.item].market_slug.as_deref() == Some("serration") && row.rank == Some(10)
            })
            .unwrap();
        assert_eq!(maxed.prices.sell, Some(10.0));
        assert!(maxed.prices.is_floor);
    }
}
