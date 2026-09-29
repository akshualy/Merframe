use serde::{Deserialize, Serialize};
use wf_data::Refinement;

use crate::catalog::{Catalog, part_name, refinement_name};
use crate::events::{ScannedTrade, ScannedTradeItem};
use crate::inventory_view::catalogued_name;
use crate::rivens::traded_riven_name;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeItem {
    pub name: String,
    pub count: i64,
    pub rank: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Trade {
    pub offered: Vec<TradeItem>,
    pub received: Vec<TradeItem>,
    pub plat: i64,
}

#[derive(Deserialize)]
struct Level {
    #[serde(default)]
    lvl: u32,
}

impl Trade {
    pub(crate) fn from_screen(screen: &ScannedTrade, catalog: &Catalog) -> Self {
        let mut trade = Self::default();
        for (items, receiving) in [(&screen.offered, false), (&screen.received, true)] {
            for item in items {
                let Some(item_type) = item.item_type.as_deref() else {
                    trade.plat += if receiving { item.count } else { -item.count };
                    continue;
                };
                let side = if receiving {
                    &mut trade.received
                } else {
                    &mut trade.offered
                };
                add_item(side, TradeItem::traded(catalog, item, item_type));
            }
        }
        trade
    }
}

impl TradeItem {
    fn traded(catalog: &Catalog, item: &ScannedTradeItem, item_type: &str) -> Self {
        let fingerprint = item.fingerprint.as_deref();
        let name = match catalog.relic_by_unique_name(item_type) {
            Some((relic, Refinement::Intact)) => format!("{} Relic", relic.name),
            Some((relic, refinement)) => format!(
                "{} Relic [{}]",
                relic.name,
                refinement_name(refinement).to_uppercase()
            ),
            None => fingerprint
                .and_then(|fingerprint| traded_riven_name(catalog, item_type, fingerprint))
                .or_else(|| catalogued_name(catalog, item_type))
                .unwrap_or_else(|| {
                    item.name
                        .strip_suffix(" Defiled")
                        .unwrap_or(&item.name)
                        .to_owned()
                }),
        };
        let level = fingerprint
            .and_then(|fingerprint| serde_json::from_str::<Level>(fingerprint).ok())
            .map_or(0, |level| level.lvl);
        Self {
            name,
            count: item.count,
            rank: item_type.starts_with("/Lotus/Upgrades/").then_some(level),
        }
    }
}

pub(crate) fn player_name(name: &str) -> String {
    name.trim_end_matches(|character: char| !character.is_ascii())
        .trim()
        .to_owned()
}

fn add_item(items: &mut Vec<TradeItem>, item: TradeItem) {
    match items
        .iter_mut()
        .find(|known| known.name == item.name && known.rank == item.rank)
    {
        Some(known) => known.count += item.count,
        None => items.push(item),
    }
}

pub fn traded_set(catalog: &Catalog, items: &[TradeItem]) -> Option<TradeItem> {
    let (first, rest) = items.split_first()?;
    if rest.is_empty() {
        return None;
    }
    let (set, _) = catalog
        .items()
        .flat_map(|item| {
            item.components
                .iter()
                .flatten()
                .map(move |part| (item, part))
        })
        .find(|(item, part)| same_part(&first.name, &part_name(item, part)))?;
    let parts: Vec<String> = set
        .components
        .iter()
        .flatten()
        .map(|part| part_name(set, part))
        .collect();
    if !items
        .iter()
        .all(|item| parts.iter().any(|part| same_part(&item.name, part)))
    {
        return None;
    }
    let mut count = items.iter().map(|item| item.count).min()?;
    if set.name == "Dual Decurion" {
        count /= 2;
    }
    Some(TradeItem {
        name: format!("{} Set", set.name),
        count,
        rank: None,
    })
}

pub fn same_part(traded: &str, catalog: &str) -> bool {
    without_blueprint(traded).eq_ignore_ascii_case(without_blueprint(catalog))
}

fn without_blueprint(name: &str) -> &str {
    let name = name.strip_suffix(" Blueprint").unwrap_or(name);
    if let Some((base, _)) = relic_refinement(name) {
        return base;
    }
    match name.rsplit_once(" (") {
        Some((base, size)) if size.len() == 2 && size.ends_with(')') => base,
        _ => name,
    }
}

pub fn relic_refinement(name: &str) -> Option<(&str, String)> {
    if let Some((base, refinement)) = name.rsplit_once(" [") {
        let refinement = refinement.strip_suffix(']')?;
        return base
            .ends_with(" Relic")
            .then(|| (base, refinement.to_lowercase()));
    }
    name.ends_with(" Relic")
        .then(|| (name, String::from("intact")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::fixtures;

    #[test]
    fn relic_refinements() {
        assert_eq!(
            relic_refinement("Axi D6 Relic [EXCEPTIONAL]"),
            Some(("Axi D6 Relic", String::from("exceptional")))
        );
        assert_eq!(
            relic_refinement("Axi A5 Relic"),
            Some(("Axi A5 Relic", String::from("intact")))
        );
        assert_eq!(relic_refinement("Forma Blueprint"), None);
        assert!(same_part("Lith Q3 Relic [RADIANT]", "Lith Q3 Relic"));
    }

    #[test]
    fn screen_items_named_from_the_catalog() {
        let arcane = "/Lotus/Upgrades/CosmeticEnhancers/Offensive/HealCompanionOnSixMeleeKills";
        let scanned = |name: &str, item_type: Option<&str>, count, fingerprint: Option<&str>| {
            ScannedTradeItem {
                name: name.to_owned(),
                item_type: item_type.map(str::to_owned),
                count,
                fingerprint: fingerprint.map(str::to_owned),
            }
        };
        let screen = ScannedTrade {
            partner: Some(String::from("TestSquadA\u{e000}")),
            offered: vec![
                scanned("Platin", None, 84, None),
                scanned(
                    "Axi A21 Relikt [STRAHLEND]",
                    Some("/Lotus/Types/Game/Projections/T4VoidProjectionStyanaxPrimeAPlatinum"),
                    2,
                    None,
                ),
            ],
            received: vec![
                scanned("Leibwaechter", Some(arcane), 1, Some("{\"lvl\":3}")),
                scanned("Leibwaechter", Some(arcane), 1, None),
                scanned("Leibwaechter", Some(arcane), 1, None),
                scanned(
                    "Unbekannt Defiled",
                    Some("/Lotus/Types/Items/Unknown/NotInTheCatalog"),
                    1,
                    None,
                ),
            ],
        };
        let trade = Trade::from_screen(&screen, &fixtures::upgrade_catalog());
        assert_eq!(trade.plat, -84);
        assert_eq!(
            trade.offered,
            [TradeItem {
                name: String::from("Axi A21 Relic [RADIANT]"),
                count: 2,
                rank: None,
            }]
        );
        assert_eq!(
            trade.received,
            [
                TradeItem {
                    name: String::from("Arcane Bodyguard"),
                    count: 1,
                    rank: Some(3),
                },
                TradeItem {
                    name: String::from("Arcane Bodyguard"),
                    count: 2,
                    rank: Some(0),
                },
                TradeItem {
                    name: String::from("Unbekannt"),
                    count: 1,
                    rank: None,
                },
            ]
        );
        assert_eq!(
            relic_refinement(&trade.offered[0].name),
            Some(("Axi A21 Relic", String::from("radiant")))
        );
    }

    #[test]
    fn partner_name_loses_the_platform_glyph() {
        assert_eq!(player_name("TestSquadA\u{e000}"), "TestSquadA");
        assert_eq!(player_name("TestSquadA"), "TestSquadA");
    }

    #[test]
    fn full_set_collapses() {
        let catalog = crate::catalog::fixtures::catalog();
        let item = |name: &str, count: i64| TradeItem {
            name: name.to_owned(),
            count,
            rank: None,
        };
        let parts = [
            item("Braton Prime Barrel", 2),
            item("Braton Prime Receiver", 1),
            item("Braton Prime Stock", 1),
            item("Braton Prime Blueprint", 1),
        ];
        assert_eq!(
            traded_set(&catalog, &parts),
            Some(item("Braton Prime Set", 1))
        );
        assert_eq!(traded_set(&catalog, &parts[..1]), None);
        let mixed = [item("Braton Prime Barrel", 1), item("Forma Blueprint", 1)];
        assert_eq!(traded_set(&catalog, &mixed), None);
    }
}
